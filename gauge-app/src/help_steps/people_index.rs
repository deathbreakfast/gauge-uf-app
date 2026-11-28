//! Spotlight steps for People (`/permission/people`).

use leptos::prelude::*;
use uf_help_macros::help_spotlight_step;

use super::help_stack;

/// People intro.
#[help_spotlight_step(
    route = "/permission/people",
    feature_highlight = "people-intro",
    title = "People",
    spotlight = "gauge-people-tabs",
    position = "bottom",
    order = 10
)]
#[component]
pub fn PeopleIntroHelp() -> impl IntoView {
    help_stack(
        "help-step-people-intro",
        "Everyone Gauge knows, people and service users apart. Open one to see each permission they pass and why.",
        Some("People appear after they first sign in."),
        &[],
    )
}

/// People search.
#[help_spotlight_step(
    route = "/permission/people",
    feature_highlight = "people-search",
    title = "Find someone",
    spotlight = "gauge-people-search",
    position = "bottom",
    order = 20
)]
#[component]
pub fn PeopleSearchHelp() -> impl IntoView {
    help_stack(
        "help-step-people-search",
        "Search by display name, or by @handle for a service user.",
        None,
        &[],
    )
}
