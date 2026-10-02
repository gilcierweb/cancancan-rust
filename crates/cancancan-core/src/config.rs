use std::sync::atomic::{AtomicBool, Ordering};

static RULES_COMPRESSOR_ENABLED: AtomicBool = AtomicBool::new(true);

/// Whether adapters compress rules before translating them to queries.
///
/// Mirrors `CanCan.rules_compressor_enabled`: enabled by default, disable to
/// inspect unoptimized rule sets.
#[must_use]
pub fn rules_compressor_enabled() -> bool {
    RULES_COMPRESSOR_ENABLED.load(Ordering::Relaxed)
}

/// Toggles rule compression for query adapters.
pub fn set_rules_compressor_enabled(enabled: bool) {
    RULES_COMPRESSOR_ENABLED.store(enabled, Ordering::Relaxed);
}
