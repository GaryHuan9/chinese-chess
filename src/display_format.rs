use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;

#[derive(Copy, Clone)]
pub struct DisplayFormat {
    pub chinese: bool,
    pub effects: bool,
    pub concise: bool,
}

pub struct AnsiEffects;

static DEFAULT_CHINESE: LazyLock<AtomicBool> =
    LazyLock::new(|| AtomicBool::new(std::env::var("CHINESE").ok().and_then(|s| s.parse().ok()) != Some(false)));

static DEFAULT_EFFECTS: LazyLock<AtomicBool> = LazyLock::new(|| {
    AtomicBool::new(
        std::env::var("EFFECTS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| std::io::IsTerminal::is_terminal(&std::io::stdout())),
    )
});

impl DisplayFormat {
    pub fn default(concise: bool) -> Self {
        Self {
            chinese: DEFAULT_CHINESE.load(Ordering::Relaxed),
            effects: DEFAULT_EFFECTS.load(Ordering::Relaxed),
            concise,
        }
    }

    pub fn pretty() -> Self {
        Self::default(false)
    }

    pub fn string() -> Self {
        Self {
            effects: false,
            ..Self::default(true)
        }
    }

    pub fn with_concise(&self, concise: bool) -> Self {
        Self { concise, ..*self }
    }

    pub fn set_default_chinese(chinese: bool) {
        DEFAULT_CHINESE.store(chinese, Ordering::Relaxed);
    }

    pub fn set_default_effects(effects: bool) {
        DEFAULT_EFFECTS.store(effects, Ordering::Relaxed);
    }
}

impl AnsiEffects {
    pub const CLEAR: &'static str = "\x1B[0m";
    pub const RED: &'static str = "\x1B[31m";
    pub const BOLD: &'static str = "\x1B[1m";
    pub const ITALICS: &'static str = "\x1B[3m";
    pub const UNDERLINE: &'static str = "\x1B[4m";
}
