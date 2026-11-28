//! Spotlight steps for the Overview (`/permission`).

use leptos::prelude::*;
use uf_help_macros::help_spotlight_step;

use super::help_stack;

/// Overview intro.
#[help_spotlight_step(
    route = "/permission",
    feature_highlight = "overview-intro",
    title = "Gauge",
    order = 10
)]
#[component]
pub fn OverviewIntroHelp() -> impl IntoView {
    help_stack(
        "help-step-overview-intro",
        "Gauge decides who can do what in every app: each check is a permission, and people get permissions through groups.",
        Some("Owners of a group or permission decide who joins it; Gauge admins manage everything."),
        &[],
    )
}

/// Waiting for you.
#[help_spotlight_step(
    route = "/permission",
    feature_highlight = "overview-waiting",
    title = "Waiting for you",
    spotlight = "gauge-overview-waiting",
    position = "bottom",
    order = 20
)]
#[component]
pub fn OverviewWaitingHelp() -> impl IntoView {
    help_stack(
        "help-step-overview-waiting",
        "Requests for the groups and permissions you own land here, and nowhere else waits on your answer.",
        Some("The Requests count in the menu is the same list."),
        &[],
    )
}

/// Your access.
#[help_spotlight_step(
    route = "/permission",
    feature_highlight = "overview-access",
    title = "Your access",
    spotlight = "gauge-overview-access",
    position = "top",
    order = 30
)]
#[component]
pub fn OverviewAccessHelp() -> impl IntoView {
    help_stack(
        "help-step-overview-access",
        "What you can do today, and the requests you're still waiting on.",
        Some("Your page lists each permission with the group it comes through."),
        &[],
    )
}
