//! Spotlight steps for Request detail (`/permission/requests/:id`).

use leptos::prelude::*;
use uf_help_macros::help_spotlight_step;

use super::help_stack;

/// Request detail summary.
#[help_spotlight_step(
    route = "/permission/requests/:id",
    feature_highlight = "request-detail-intro",
    title = "Reading a request",
    spotlight = "gauge-request-detail-summary",
    position = "bottom",
    order = 10
)]
#[component]
pub fn RequestDetailIntroHelp() -> impl IntoView {
    help_stack(
        "help-step-request-detail-intro",
        "Members get is what approving gives; has now is what they already hold.",
        Some("A request for a service user names it and its owner under For."),
        &[],
    )
}

/// Approve access request.
#[help_spotlight_step(
    route = "/permission/requests/:id",
    feature_highlight = "request-detail-approve",
    title = "Approve the ask",
    spotlight = "gauge-request-approve",
    position = "top",
    order = 20
)]
#[component]
pub fn RequestDetailApproveHelp() -> impl IntoView {
    help_stack(
        "help-step-request-detail-approve",
        "Approve gives them what Members get right away, after your authenticator code.",
        Some("They get a notification, and the page keeps who approved it and when."),
        &[],
    )
}

/// Deny access request.
#[help_spotlight_step(
    route = "/permission/requests/:id",
    feature_highlight = "request-detail-deny",
    title = "Deny the ask",
    spotlight = "gauge-request-deny",
    position = "top",
    order = 30
)]
#[component]
pub fn RequestDetailDenyHelp() -> impl IntoView {
    help_stack(
        "help-step-request-detail-deny",
        "Deny asks for a reason, and the requester sees it in the notification and on this page.",
        Some("Say what would change your answer, so they know whether to ask again."),
        &[],
    )
}
