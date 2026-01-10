use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Copy, Clone)]
pub struct DisplayFormat {
    pub chinese: bool,
    pub effects: bool,
    pub concise: bool,
}

static DEFAULT_CHINESE: AtomicBool = AtomicBool::new(true);
static DEFAULT_EFFECTS: AtomicBool = AtomicBool::new(true);

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
