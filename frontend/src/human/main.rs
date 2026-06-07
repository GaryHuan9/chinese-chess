use chinese_chess::board::Board;
use chinese_chess::display_format::DisplayFormat;
use chinese_chess::game::Game;
use chinese_chess::location::{Location, Move};
use chinese_chess::piece::{Piece, PieceKind};
use clap::Parser;
use eframe::egui;
use frontend::line_stream::AsyncLineStream;
use frontend::protocol::{ArbiterMessage, PlayerMessage, Protocol};
use smol::channel::{Receiver, Sender};
use std::net::{IpAddr, SocketAddr};
use std::thread;
use std::time::Duration;

#[derive(Parser, Debug)]
struct Arguments {
    #[arg(short, long, default_value = "127.0.0.1")]
    ip: IpAddr,

    #[arg(short, long, default_value_t = 6000)]
    port: u16,

    #[arg(short, long, default_value = "human")]
    name: String,
}

struct Application {
    game: Option<Game>,
    receiver: Receiver<ArbiterMessage>,
    sender: Sender<PlayerMessage>,
    selected_location: Option<Location>,
    pending_moves: Vec<Move>,
}

impl Application {
    fn new(
        cc: &eframe::CreationContext<'_>,
        receiver: Receiver<ArbiterMessage>,
        sender: Sender<PlayerMessage>,
    ) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        Self {
            game: None,
            receiver,
            sender,
            selected_location: None,
            pending_moves: Vec::new(),
        }
    }

    fn draw_board(&mut self, ui: &mut egui::Ui) {
        let cell_size = 60.0;
        let margin_x = 25.0;
        let margin_y = 25.0;
        let board_width = Board::WIDTH as f32 * cell_size;
        let board_height = Board::HEIGHT as f32 * cell_size;

        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(board_width + margin_x * 2.0, board_height + margin_y * 2.0),
            egui::Sense::click(),
        );

        let painter = ui.painter_at(rect);

        // Draw background
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(240, 217, 181));

        let get_center = |x: i8, y: i8| -> egui::Pos2 {
            egui::pos2(
                rect.left() + margin_x + x as f32 * cell_size + cell_size / 2.0,
                rect.bottom() - margin_y - y as f32 * cell_size - cell_size / 2.0,
            )
        };

        // Draw coordinates
        for x in 0..Board::WIDTH {
            let text = format!("{}", (b'A' + x as u8) as char);
            painter.text(
                egui::pos2(get_center(x, 0).x, rect.bottom() - margin_y / 2.0),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(14.0),
                egui::Color32::BLACK,
            );
        }
        for y in 0..Board::HEIGHT {
            let text = format!("{}", y);
            painter.text(
                egui::pos2(rect.left() + margin_x / 2.0, get_center(0, y).y),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(14.0),
                egui::Color32::BLACK,
            );
        }

        // Horizontal lines
        for y in 0..Board::HEIGHT {
            painter.line_segment(
                [get_center(0, y), get_center(Board::WIDTH - 1, y)],
                (1.0, egui::Color32::BLACK),
            );
        }

        // Vertical lines
        for x in 0..Board::WIDTH {
            painter.line_segment([get_center(x, 0), get_center(x, 4)], (1.0, egui::Color32::BLACK));
            painter.line_segment([get_center(x, 5), get_center(x, 9)], (1.0, egui::Color32::BLACK));
        }

        // Connect the sides over the river
        painter.line_segment([get_center(0, 4), get_center(0, 5)], (1.0, egui::Color32::BLACK));
        painter.line_segment(
            [get_center(Board::WIDTH - 1, 4), get_center(Board::WIDTH - 1, 5)],
            (1.0, egui::Color32::BLACK),
        );

        // Red palace (bottom)
        painter.line_segment([get_center(3, 0), get_center(5, 2)], (1.0, egui::Color32::BLACK));
        painter.line_segment([get_center(5, 0), get_center(3, 2)], (1.0, egui::Color32::BLACK));

        // Black palace (top)
        painter.line_segment([get_center(3, 7), get_center(5, 9)], (1.0, egui::Color32::BLACK));
        painter.line_segment([get_center(5, 7), get_center(3, 9)], (1.0, egui::Color32::BLACK));

        let mut clicked_loc = None;
        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let x = ((pos.x - rect.left() - margin_x) / cell_size).floor() as i8;
            let y = ((rect.bottom() - pos.y - margin_y) / cell_size).floor() as i8;
            if (0..Board::WIDTH).contains(&x) && (0..Board::HEIGHT).contains(&y) {
                clicked_loc = Some(Location::from_xy(x, y).unwrap());
            }
        }

        if let Some(loc) = clicked_loc {
            self.handle_click(loc);
        }

        let game = self.game.as_ref().unwrap();
        // Highlight last move
        if let Some(mv) = game.history().last().map(|&(mv, _)| mv) {
            for &loc in &[mv.from, mv.to] {
                let center = get_center(loc.x(), loc.y());
                painter.rect_filled(
                    egui::Rect::from_center_size(center, egui::vec2(cell_size * 0.9, cell_size * 0.9)),
                    4.0,
                    egui::Color32::from_rgba_unmultiplied(255, 255, 0, 100),
                );
            }
        }

        // Draw pieces
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        for x in 0..Board::WIDTH {
            for y in 0..Board::HEIGHT {
                let loc = Location::from_xy(x, y).unwrap();
                let center = get_center(x, y);

                if let Some(piece) = game.board()[loc] {
                    let color_str = if piece.is_red() { "red" } else { "black" };
                    let svg_name = format!("piece_char_{}_{}.svg", color_str, piece.fen());

                    let path = format!("file://assets/{}", svg_name);

                    let image = egui::Image::new(path).fit_to_exact_size(egui::vec2(cell_size * 0.8, cell_size * 0.8));
                    let img_rect = egui::Rect::from_center_size(center, egui::vec2(cell_size * 0.8, cell_size * 0.8));
                    image.paint_at(ui, img_rect);

                    if Some(loc) == self.selected_location {
                        painter.rect_stroke(img_rect, 0.0, (2.0, egui::Color32::GREEN), egui::StrokeKind::Inside);
                    }
                } else if Some(loc) == self.selected_location {
                    let empty_rect = egui::Rect::from_center_size(center, egui::vec2(cell_size * 0.8, cell_size * 0.8));
                    painter.rect_stroke(empty_rect, 0.0, (2.0, egui::Color32::GREEN), egui::StrokeKind::Inside);
                }

                // Highlight valid moves
                if let Some(selected) = self.selected_location {
                    for mv in &self.pending_moves {
                        if mv.from == selected && mv.to == loc {
                            painter.circle_filled(
                                center,
                                cell_size * 0.2,
                                egui::Color32::from_rgba_unmultiplied(0, 255, 0, 128),
                            );
                        }
                    }
                }
            }
        }
    }

    fn handle_click(&mut self, loc: Location) {
        if let Some(from) = self.selected_location {
            if from == loc {
                self.selected_location = None;
                return;
            }

            if let Some(mv) = self.pending_moves.iter().find(|m| m.from == from && m.to == loc) {
                let _ = self.sender.try_send(PlayerMessage::Play { mv: *mv });
                self.pending_moves.clear();
                self.selected_location = None;
            } else if let Some(game) = &self.game {
                if game.board()[loc].is_some() {
                    self.selected_location = Some(loc);
                } else {
                    self.selected_location = None;
                }
            }
        } else if let Some(game) = &self.game
            && game.board()[loc].is_some()
        {
            self.selected_location = Some(loc);
        }
    }
}

impl eframe::App for Application {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(msg) = self.receiver.try_recv() {
            match msg {
                ArbiterMessage::Game { fen, red_turn } => {
                    let board = Board::from_fen(&fen).unwrap();
                    self.game = Some(Game::new(board, red_turn));
                    self.selected_location = None;
                    let _ = self.sender.try_send(PlayerMessage::Ready);
                }
                ArbiterMessage::Update { mv } => {
                    if let Some(game) = &mut self.game {
                        game.make_move(mv);
                    }
                }
                ArbiterMessage::Prompt { .. } => {
                    if let Some(game) = &self.game {
                        self.pending_moves.clear();
                        game.fill_moves(&mut self.pending_moves);
                    }
                }
            }
        }
        ctx.request_repaint_after(Duration::from_millis(50));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            if self.game.is_some() {
                ui.horizontal(|ui| {
                    // Left column: Board + Status
                    ui.vertical(|ui| {
                        self.draw_board(ui);

                        ui.add_space(10.0);
                        let game = self.game.as_ref().unwrap();
                        let mut status = String::new();
                        let format = DisplayFormat {
                            chinese: false,
                            effects: false,
                            concise: false,
                        };

                        if let Some(mv) = game.history().last().map(|&(mv, _)| mv) {
                            let piece = game.board()[mv.to].unwrap().display(format.with_concise(true));
                            status.push_str(&format!("({}) {} {} - ", game.history().len(), mv, piece));
                        }

                        if let Some(outcome) = game.outcome() {
                            status.push_str(&format!("{}", outcome.display(format)));
                        } else {
                            let check = game.board().king_in_check(game.red_turn());
                            let king = Piece::from_kind(PieceKind::King, game.red_turn()).display(format);
                            status.push_str(&format!("{} {} - ", king, if check { "in check" } else { "to play" }));
                            status.push_str(&format!("{} legal moves", game.iter_moves().count()));
                        }

                        ui.label(status);
                        ui.label(game.board().fen());
                    });

                    ui.add_space(20.0);

                    // Right column: Capture list
                    let game = self.game.as_ref().unwrap();
                    let captured: Vec<_> = game.history().iter().filter_map(|&(_, capture)| capture).collect();
                    if !captured.is_empty() {
                        ui.vertical(|ui| {
                            ui.heading("Captured");
                            ui.add_space(10.0);

                            ui.horizontal(|ui| {
                                // Red column
                                let red_captured: Vec<_> = captured.iter().filter(|p| p.is_red()).collect();
                                if !red_captured.is_empty() {
                                    ui.horizontal(|ui| {
                                        for chunk in red_captured.chunks(10) {
                                            ui.vertical(|ui| {
                                                for &&piece in chunk {
                                                    let svg_name = format!("piece_char_red_{}.svg", piece.fen());
                                                    let path = format!("file://assets/{}", svg_name);
                                                    ui.add(
                                                        egui::Image::new(path)
                                                            .fit_to_exact_size(egui::vec2(40.0, 40.0)),
                                                    );
                                                }
                                            });
                                        }
                                    });
                                }

                                ui.add_space(20.0);

                                // Black column
                                let black_captured: Vec<_> = captured.iter().filter(|p| !p.is_red()).collect();
                                if !black_captured.is_empty() {
                                    ui.horizontal(|ui| {
                                        for chunk in black_captured.chunks(10) {
                                            ui.vertical(|ui| {
                                                for &&piece in chunk {
                                                    let svg_name = format!("piece_char_black_{}.svg", piece.fen());
                                                    let path = format!("file://assets/{}", svg_name);
                                                    ui.add(
                                                        egui::Image::new(path)
                                                            .fit_to_exact_size(egui::vec2(40.0, 40.0)),
                                                    );
                                                }
                                            });
                                        }
                                    });
                                }
                            });
                        });
                    }
                });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.heading("Waiting for game to start...");
                });
            }
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let args = Arguments::parse();

    let address = SocketAddr::new(args.ip, args.port);

    let (tx_to_ui, rx_in_ui) = smol::channel::unbounded();
    let (tx_from_ui, rx_in_thread) = smol::channel::unbounded();

    let args_name = args.name.clone();

    thread::spawn(move || {
        smol::block_on(async {
            loop {
                // Attempt to connect
                let stream = match smol::net::TcpStream::connect(address).await {
                    Ok(s) => s,
                    Err(_) => {
                        smol::Timer::after(Duration::from_millis(50)).await;
                        continue;
                    }
                };

                let line_stream = AsyncLineStream::new(stream);

                // Send init and info
                let _ = line_stream
                    .write_line(Protocol::encode_player(&PlayerMessage::Init { version: 1 }))
                    .await;
                let _ = line_stream
                    .write_line(Protocol::encode_player(&PlayerMessage::Info {
                        name: args_name.clone(),
                    }))
                    .await;

                // Race the read loop against the write loop
                smol::future::race(
                    async {
                        // Read loop
                        while let Some(line) = line_stream.read_line().await {
                            if let Some(msg) = Protocol::decode_arbiter(&line) {
                                if tx_to_ui.send(msg).await.is_err() {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                    },
                    async {
                        // Write loop
                        while let Ok(msg) = rx_in_thread.recv().await {
                            let encoded = Protocol::encode_player(&msg);
                            if line_stream.write_line(encoded).await.is_err() {
                                break;
                            }
                        }
                    },
                )
                .await;
                // If the race completes, it means either we lost connection or the UI closed.
                // We pause slightly, then reconnect!
                smol::Timer::after(Duration::from_millis(50)).await;
            }
        });
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Chinese Chess",
        options,
        Box::new(|cc| Ok(Box::new(Application::new(cc, rx_in_ui, tx_from_ui)) as Box<dyn eframe::App>)),
    )
}
