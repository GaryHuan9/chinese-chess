use chinese_chess::board::Board;
use chinese_chess::game::Game;
use chinese_chess::location::{Location, Move};
use std::io;

fn main() {
    let mut board = Board::from_fen(&mut "CRH1k1e2/3ca4/4ea3/9/2hr5/9/9/4E4/4A4/4KA3".chars()).unwrap();
    println!("{}", board);
    println!("{}", board.iter_legal_moves(true).count());

    for mv in board.iter_legal_moves(true).collect::<Vec<Move>>() {
        let (_, capture) = board.play(mv);
        println!("{}", board);
        board.undo(mv, capture);
    }

    return;
    let mut game = Game::opening();

    loop {
        std::println!("{}", game);

        let mut moves = vec![];
        game.fill_moves(&mut moves);

        for mv in moves {
            // println!("{}{}", mv.from, mv.to);
            // game.play(mv);
            // println!("{}", game);
            // game.undo();
        }

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim().to_ascii_lowercase();

        let mut chars = input.chars();

        if let Some(from) = Location::from_chars(&mut chars)
            && let Some(to) = Location::from_chars(&mut chars)
        {
            let mv = Move { from, to };
            game.play(mv);
        } else if input == "undo" {
            game.undo();
        }
    }
}
