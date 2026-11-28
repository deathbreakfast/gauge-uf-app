//! Spotlight steps for a person's page (`/permission/people/:id`).

use leptos::prelude::*;
use uf_help_macros::help_spotlight_step;

use super::help_stack;

/// What they can do.
#[help_spotlight_step(
    route = "/permission/people/:id",
    feature_highlight = "person-can-do",
    title = "What they can do",
    spotlight = "gauge-person-can-do",
    position = "bottom",
    order = 10
)]
#[component]
pub fn PersonCanDoHelp() -> impl IntoView {
    help_stack(
        "help-step-person-can-do",
        "Every check this person passes today, with the group or allow list behind it.",
        Some("To take a permission away, remove them from the group it comes through."),
        &[],
    )
}

/// Their groups.
#[help_spotlight_step(
    route = "/permission/people/:id",
    feature_highlight = "person-groups",
    title = "Groups",
    spotlight = "gauge-person-groups",
    position = "top",
    order = 20
)]
#[component]
pub fn PersonGroupsHelp() -> impl IntoView {
    help_stack(
        "help-step-person-groups",
        "Removing someone from a group asks for your authenticator code and tells them.",
        Some("Before you confirm, Gauge shows what they lose and what they keep through other groups."),
        &[],
    )
}
