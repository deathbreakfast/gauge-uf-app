//! Process-local overrides for gauge-uf-app-e2e Playwright seeds.
//!
//! Compiled only with Cargo feature `e2e-lab` (enabled by the e2e host). Production
//! hosts must not enable that feature. Default remains normal service behavior.

use std::sync::atomic::{AtomicU8, Ordering};

use leptos::prelude::*;

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

/// RFC 6238 fixture secret (base32). Lab harness only.
#[cfg(feature = "ssr")]
pub const HARNESS_TOTP_SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

/// Current 6-digit TOTP for the harness fixture secret (lab only).
///
/// Used by [`crate::pages::step_up::spawn_with_fresh_totp`] when `e2e-lab` is on
/// so Playwright hosts need not mount axum-login for Super User membership flows.
#[server(E2eLabTotpCode)]
pub async fn e2e_lab_totp_code() -> Result<String, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use chrono::Utc;
        use totp_rs::{Algorithm, Secret, TOTP};

        let ctx = higgs::Higgs::from_request()
            .await
            .map_err(|_| ServerFnError::new("STEP_UP:auth_required: authentication required"))?;
        if ctx.session_user_id().is_none() {
            return Err(ServerFnError::new(
                "STEP_UP:auth_required: authentication required",
            ));
        }
        let secret = Secret::Encoded(HARNESS_TOTP_SECRET.to_string())
            .to_bytes()
            .map_err(|_| ServerFnError::new("STEP_UP:totp_secret: invalid harness secret"))?;
        let totp = TOTP::new(Algorithm::SHA1, 6, 1, 30, secret)
            .map_err(|_| ServerFnError::new("STEP_UP:totp_secret: totp build failed"))?;
        return Ok(totp.generate(Utc::now().timestamp() as u64));
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::new("ssr required"))
    }
}
