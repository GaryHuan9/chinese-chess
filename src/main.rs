use chinese_chess::game::Game;
use chinese_chess::location::{Location, Move};
use std::io;

fn main() {
    let mut game = Game::opening();

    loop {
        std::println!("{}", game);

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
