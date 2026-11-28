//! Spotlight steps for a permission's page (`/permission/permissions/:id`).

use leptos::prelude::*;
use uf_help_macros::help_spotlight_step;

use super::help_stack;

/// Permission page intro.
#[help_spotlight_step(
    route = "/permission/permissions/:id",
    feature_highlight = "permission-detail-intro",
    title = "This permission",
    spotlight = "gauge-perm-detail-header",
    position = "bottom",
    order = 10
)]
#[component]
pub fn PermissionDetailIntroHelp() -> impl IntoView {
    help_stack(
        "help-step-permission-detail-intro",
        "Code checks the key; everyone else reads the name and description.",
        Some("Owned by names the group whose owners decide who gets it."),
        &[],
    )
}

/// Give access, for editors.
#[help_spotlight_step(
    route = "/permission/permissions/:id",
    feature_highlight = "permission-detail-give",
    title = "Give access",
    spotlight = "gauge-perm-give",
    position = "bottom",
    order = 20
)]
#[component]
pub fn PermissionDetailGiveHelp() -> impl IntoView {
    help_stack(
        "help-step-permission-detail-give",
        "When a group already holds this permission, adding the person to that group keeps access in one place.",
        Some("Each change asks for your authenticator code."),
        &[],
    )
}

/// Who has access, Allow list and Activity.
#[help_spotlight_step(
    route = "/permission/permissions/:id",
    feature_highlight = "permission-detail-tabs",
    title = "Who has it, and why",
    spotlight = "gauge-perm-tabs",
    position = "top",
    order = 30
)]
#[component]
pub fn PermissionDetailTabsHelp() -> impl IntoView {
    help_stack(
        "help-step-permission-detail-tabs",
        "Who has access lists everyone who passes the check today and the path that gives it to them. The allow list holds only the direct grants.",
        Some("Removing someone from the allow list takes away only what that list gave them."),
        &[],
    )
}

/// Request access, for people who can't edit.
#[help_spotlight_step(
    route = "/permission/permissions/:id",
    feature_highlight = "permission-detail-request",
    title = "Ask for access",
    spotlight = "gauge-perm-request-access",
    position = "bottom",
    order = 40
)]
#[component]
pub fn PermissionDetailRequestHelp() -> impl IntoView {
    help_stack(
        "help-step-permission-detail-request",
        "Request access asks the owners for this permission. You'll see their answer under Requests.",
        None,
        &[],
    )
}
