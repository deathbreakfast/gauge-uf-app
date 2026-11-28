//! Spotlight steps for the Permissions list (`/permission/permissions`).

use leptos::prelude::*;
use uf_help_macros::help_spotlight_step;

use super::help_stack;

/// Centered intro: what Gauge decides and its words.
#[help_spotlight_step(
    route = "/permission/permissions",
    feature_highlight = "permission-intro",
    title = "Welcome to Gauge",
    order = 10
)]
#[component]
pub fn PermissionIntroHelp() -> impl IntoView {
    help_stack(
        "help-step-permission-intro",
        "Gauge decides who can do what in each app. Code checks a permission by its key; people read its name.",
        Some("Anyone signed in can look around. Changing who has access needs ownership of the record or GaugeAdmin."),
        &[
            "Permission: one thing someone can do",
            "Group: people who get the same permissions",
            "Domain: the app or area a permission belongs to",
            "Request: asking the owners for access",
        ],
    )
}

/// Left navigation destinations.
#[help_spotlight_step(
    route = "/permission/permissions",
    feature_highlight = "permission-nav",
    title = "Finding your way",
    spotlight = "permission-left-nav",
    position = "right",
    order = 20
)]
#[component]
pub fn PermissionNavHelp() -> impl IntoView {
    help_stack(
        "help-step-permission-nav",
        "Move between permissions, groups and requests from here. Requests waiting for your answer show up under Requests.",
        None,
        &[],
    )
}

/// The permissions table and its search.
#[help_spotlight_step(
    route = "/permission/permissions",
    feature_highlight = "permission-list",
    title = "Find a permission",
    spotlight = "gauge-permissions-list",
    position = "top",
    order = 30
)]
#[component]
pub fn PermissionListHelp() -> impl IntoView {
    help_stack(
        "help-step-permission-list",
        "Search matches names, keys and descriptions. Open a row to see who has that permission and why.",
        Some("Owned by names the group whose owners review requests and manage the allow list."),
        &[],
    )
}

/// Create permission, for Gauge admins.
#[help_spotlight_step(
    route = "/permission/permissions",
    feature_highlight = "permission-create-cta",
    title = "Create a permission",
    spotlight = "gauge-permissions-create",
    position = "bottom",
    order = 40
)]
#[component]
pub fn PermissionCreateCtaHelp() -> impl IntoView {
    help_stack(
        "help-step-permission-create-cta",
        "Gauge admins add a permission when an app needs a new check.",
        Some("Only Gauge admins see this button."),
        &[],
    )
}
