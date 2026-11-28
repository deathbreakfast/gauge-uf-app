//! Spotlight steps for Create permission (`/permission/create-permission`).

use leptos::prelude::*;
use uf_help_macros::help_spotlight_step;

use super::help_stack;

/// Create permission intro.
#[help_spotlight_step(
    route = "/permission/create-permission",
    feature_highlight = "create-permission-intro",
    title = "Creating a permission",
    spotlight = "gauge-create-perm-page",
    position = "bottom",
    order = 10
)]
#[component]
pub fn CreatePermissionIntroHelp() -> impl IntoView {
    help_stack(
        "help-step-create-permission-intro",
        "A permission is one thing an app checks before it lets someone act.",
        Some("Nobody holds it until you give access on its page."),
        &[],
    )
}

/// Name makes the key.
#[help_spotlight_step(
    route = "/permission/create-permission",
    feature_highlight = "create-permission-key",
    title = "The key",
    spotlight = "gauge-create-perm-key",
    position = "bottom",
    order = 20
)]
#[component]
pub fn CreatePermissionKeyHelp() -> impl IntoView {
    help_stack(
        "help-step-create-permission-key",
        "Code checks the key, so it can't change once saved. It follows the name until you edit it.",
        None,
        &[],
    )
}

/// Owned by defaults to a new group.
#[help_spotlight_step(
    route = "/permission/create-permission",
    feature_highlight = "create-permission-owners",
    title = "Who owns it",
    spotlight = "gauge-create-perm-owners",
    position = "top",
    order = 30
)]
#[component]
pub fn CreatePermissionOwnersHelp() -> impl IntoView {
    help_stack(
        "help-step-create-permission-owners",
        "Owners review requests and manage the allow list. Leave it as is to start a new owners group with you in it.",
        None,
        &[],
    )
}

/// Create permission.
#[help_spotlight_step(
    route = "/permission/create-permission",
    feature_highlight = "create-permission-submit",
    title = "Save it",
    spotlight = "gauge-create-perm-submit",
    position = "top",
    order = 40
)]
#[component]
pub fn CreatePermissionSubmitHelp() -> impl IntoView {
    help_stack(
        "help-step-create-permission-submit",
        "Creating asks for your authenticator code, then opens the new permission.",
        None,
        &[],
    )
}
