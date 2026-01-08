use chinese_chess::location::Move;
pub enum ArbiterMessage {
    Game { fen: String, red_turn: bool },
    Prompt { time: u32 },
    Update { mv: Move },
}

pub enum PlayerMessage {
    Init { version: u32 },
    Info { name: String },
    Ready,
    Play { mv: Move },
}

pub struct Protocol;

impl Protocol {
    fn decode(line: &str) -> (&str, impl Iterator<Item = &str>) {
        let mut parts = line.trim().split_whitespace().fuse();
        (parts.next().unwrap_or(""), parts)
    }

    pub fn decode_arbiter(line: &str) -> Option<ArbiterMessage> {
        let (kind, mut arguments) = Protocol::decode(line);
        let message = match kind {
            "game" => ArbiterMessage::Game {
                fen: arguments.next()?.to_string(),
                red_turn: arguments.next()?.parse().ok()?,
            },
            "prompt" => ArbiterMessage::Prompt {
                time: arguments.next()?.parse().ok()?,
            },
            "update" => ArbiterMessage::Update {
                mv: arguments.next()?.parse().ok()?,
            },
            _ => return None,
        };
        Some(message)
    }

    pub fn decode_player(line: &str) -> Option<PlayerMessage> {
        let (kind, mut arguments) = Protocol::decode(line);
        let message = match kind {
            "init" => PlayerMessage::Init {
                version: arguments.next()?.parse().ok()?,
            },
            "info" => PlayerMessage::Info {
                name: arguments.next()?.to_string(),
            },
            "ready" => PlayerMessage::Ready,
            "play" => PlayerMessage::Play {
                mv: arguments.next()?.parse().ok()?,
            },
            _ => return None,
        };
        Some(message)
    }

    pub fn encode_arbiter(message: ArbiterMessage) -> String {
        match message {
            ArbiterMessage::Game { fen, red_turn } => format!("game {fen} {red_turn}"),
            ArbiterMessage::Prompt { time } => format!("prompt {time}"),
            ArbiterMessage::Update { mv } => format!("update {mv}"),
        }
    }

    pub fn encode_player(message: PlayerMessage) -> String {
        match message {
            PlayerMessage::Init { version } => format!("init {version}"),
            PlayerMessage::Info { name } => format!("info {name}"),
            PlayerMessage::Ready => "ready".to_string(),
            PlayerMessage::Play { mv } => format!("play {mv}"),
        }
    }
}
