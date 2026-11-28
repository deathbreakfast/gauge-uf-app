//! Spotlight steps for Domains (`/permission/domains`).

use leptos::prelude::*;
use uf_help_macros::help_spotlight_step;

use super::help_stack;

/// Domains intro.
#[help_spotlight_step(
    route = "/permission/domains",
    feature_highlight = "domains-intro",
    title = "Domains",
    spotlight = "gauge-domains-list",
    position = "top",
    order = 10
)]
#[component]
pub fn DomainsIntroHelp() -> impl IntoView {
    help_stack(
        "help-step-domains-intro",
        "Each permission belongs to one domain, usually one per app.",
        Some("A domain can be deleted only once its permissions are moved or deleted."),
        &[],
    )
}
