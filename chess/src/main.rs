use chinese_chess::game::Game;
use chinese_chess::location::Move;
use std::io;

fn main() {
    let mut game = Game::opening();

    loop {
        std::println!("{}", game);

        let mut moves = vec![];
        game.fill_moves(&mut moves);

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim().to_ascii_lowercase();

        if let Ok(mv) = input.parse::<Move>() {
            game.play(mv);
        } else if input == "undo" {
            game.undo();
        }
    }
}
