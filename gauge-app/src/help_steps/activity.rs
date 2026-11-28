//! Spotlight steps for Activity (`/permission/activity`).

use leptos::prelude::*;
use uf_help_macros::help_spotlight_step;

use super::help_stack;

/// Activity filters.
#[help_spotlight_step(
    route = "/permission/activity",
    feature_highlight = "activity-filters",
    title = "Narrow the log",
    spotlight = "gauge-activity-filters",
    position = "bottom",
    order = 10
)]
#[component]
pub fn ActivityFiltersHelp() -> impl IntoView {
    help_stack(
        "help-step-activity-filters",
        "Every change to access across Gauge, newest first. Filter by kind and time, or search for a person, group or permission.",
        Some("Filters stay in the link, so you can share a narrowed log."),
        &[],
    )
}

/// Activity CSV.
#[help_spotlight_step(
    route = "/permission/activity",
    feature_highlight = "activity-csv",
    title = "Download",
    spotlight = "gauge-activity-csv",
    position = "left",
    order = 20
)]
#[component]
pub fn ActivityCsvHelp() -> impl IntoView {
    help_stack(
        "help-step-activity-csv",
        "Download CSV saves the rows loaded now, with your filters, for an access review.",
        None,
        &[],
    )
}
