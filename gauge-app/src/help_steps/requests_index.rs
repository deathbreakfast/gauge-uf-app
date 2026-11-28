//! Spotlight steps for the Requests index (`/permission/requests`).

use leptos::prelude::*;
use uf_help_macros::help_spotlight_step;

use super::help_stack;

/// Requests index intro.
#[help_spotlight_step(
    route = "/permission/requests",
    feature_highlight = "requests-intro",
    title = "Access requests",
    spotlight = "gauge-requests-tabs",
    position = "bottom",
    order = 10
)]
#[component]
pub fn RequestsIntroHelp() -> impl IntoView {
    help_stack(
        "help-step-requests-intro",
        "Waiting for you holds requests for what you own; Your requests holds the ones you sent.",
        Some("Nothing changes until an owner approves."),
        &[],
    )
}

/// Requests filters.
#[help_spotlight_step(
    route = "/permission/requests",
    feature_highlight = "requests-filters",
    title = "Find a request",
    spotlight = "gauge-requests-filters",
    position = "bottom",
    order = 20
)]
#[component]
pub fn RequestsFiltersHelp() -> impl IntoView {
    help_stack(
        "help-step-requests-filters",
        "Search by person, group, permission or service user; the filters stay in the link.",
        None,
        &[],
    )
}

/// Waiting for you list.
#[help_spotlight_step(
    route = "/permission/requests",
    feature_highlight = "requests-needs-review",
    title = "Answer a request",
    spotlight = "gauge-requests-needs-review",
    position = "top",
    order = 30
)]
#[component]
pub fn RequestsNeedsReviewHelp() -> impl IntoView {
    help_stack(
        "help-step-requests-needs-review",
        "Open a row to read the reason and what approving gives, then approve or deny with a reason they'll see.",
        None,
        &[],
    )
}
