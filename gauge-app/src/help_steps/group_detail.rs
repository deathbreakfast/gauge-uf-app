//! Spotlight steps for a group's page (`/permission/groups/:id`).

use leptos::prelude::*;
use uf_help_macros::help_spotlight_step;

use super::help_stack;

/// Group page intro.
#[help_spotlight_step(
    route = "/permission/groups/:id",
    feature_highlight = "group-detail-intro",
    title = "This group",
    spotlight = "gauge-group-detail-header",
    position = "bottom",
    order = 10
)]
#[component]
pub fn GroupDetailIntroHelp() -> impl IntoView {
    help_stack(
        "help-step-group-detail-intro",
        "Everyone in a group gets what the group holds. Groups inside it get the same.",
        Some("Owners approve requests to join and add or remove members."),
        &[],
    )
}

/// Add members, for owners.
#[help_spotlight_step(
    route = "/permission/groups/:id",
    feature_highlight = "group-detail-add",
    title = "Add members",
    spotlight = "gauge-group-add-members",
    position = "bottom",
    order = 20
)]
#[component]
pub fn GroupDetailAddHelp() -> impl IntoView {
    help_stack(
        "help-step-group-detail-add",
        "Add people, service users or other groups. Each change asks for your authenticator code.",
        Some("Super User changes need a fresh code every time."),
        &[],
    )
}

/// The group's tabs.
#[help_spotlight_step(
    route = "/permission/groups/:id",
    feature_highlight = "group-detail-tabs",
    title = "Members, access and owners",
    spotlight = "gauge-group-tabs",
    position = "top",
    order = 30
)]
#[component]
pub fn GroupDetailTabsHelp() -> impl IntoView {
    help_stack(
        "help-step-group-detail-tabs",
        "Access shows what every member gets from this group. Removing a member shows what they lose and keep before you confirm.",
        Some("After a removal you can undo it for a few seconds."),
        &[],
    )
}

/// Request access, for people who can't manage the group.
#[help_spotlight_step(
    route = "/permission/groups/:id",
    feature_highlight = "group-detail-request",
    title = "Ask to join",
    spotlight = "gauge-group-request-access",
    position = "bottom",
    order = 40
)]
#[component]
pub fn GroupDetailRequestHelp() -> impl IntoView {
    help_stack(
        "help-step-group-detail-request",
        "Request access asks the owners to add you. You'll see their answer under Requests.",
        None,
        &[],
    )
}
