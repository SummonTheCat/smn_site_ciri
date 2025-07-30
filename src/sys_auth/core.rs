
use std::env;

/// Returns the configured service key, if any.
pub fn expected_key() -> Option<String> {
    // try both upper- and lower-case env var names for compatibility
    env::var("SMNSERVICEKEY")
        .or_else(|_| env::var("smnservicekey"))
        .ok()
}

/// Compare a provided key against the expected one.
pub fn verify(provided: &str) -> bool {
    match expected_key() {
        Some(ref k) if provided == k => true,
        _ => false,
    }
}
