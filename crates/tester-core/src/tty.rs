use std::sync::atomic::{AtomicBool, Ordering};

static TUI_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn is_active() -> bool {
    TUI_ACTIVE.load(Ordering::SeqCst)
}

pub fn set_active(v: bool) {
    TUI_ACTIVE.store(v, Ordering::SeqCst);
}
