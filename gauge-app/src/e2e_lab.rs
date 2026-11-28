//! Process-local overrides for gauge-uf-app-e2e Playwright seeds.
//!
//! Compiled only with Cargo feature `e2e-lab` (enabled by the e2e host). Production
//! hosts must not enable that feature. Default remains normal service behavior.

use std::sync::atomic::{AtomicU8, Ordering};

static LIST_DOMAINS: AtomicU8 = AtomicU8::new(0);

/// How `list_domains` should behave under an e2e seed override.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListDomainsOverride {
    /// Call the real gauge service.
    Normal,
    /// Return an empty domain list (Select disabled + empty copy).
    Empty,
    /// Return a server error (Select disabled + error MessageBar).
    Error,
}

/// Set by `POST /api/test/seed-data` in gauge-uf-app-e2e only.
pub fn set_list_domains_override(mode: Option<&str>) {
    let v = match mode {
        Some("empty") => 1,
        Some("error") => 2,
        _ => 0,
    };
    LIST_DOMAINS.store(v, Ordering::SeqCst);
}

pub(crate) fn list_domains_override() -> ListDomainsOverride {
    match LIST_DOMAINS.load(Ordering::SeqCst) {
        1 => ListDomainsOverride::Empty,
        2 => ListDomainsOverride::Error,
        _ => ListDomainsOverride::Normal,
    }
}
