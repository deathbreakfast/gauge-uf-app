//! Help spotlight tour steps for Gauge (`/permission`) routes.
//!
//! Inventory is registered via [`uf_help_macros::help_spotlight_step`]. Call
//! [`ensure_help_steps_linked`] from the host or [`crate::PermissionRoutes`] so
//! `inventory` submissions survive linking.

mod activity;
mod domain_create;
mod domains_index;
mod group_create;
mod group_detail;
mod groups_index;
mod overview;
mod people_index;
mod permission_create;
mod permission_detail;
mod permissions_index;
mod person_detail;
mod request_detail;
mod requests_index;

use leptos::prelude::*;
use uf_product::components::{Body1, Caption1, SpacingSize};
use uf_product::primitives::Flex;

/// Shared step body: lead paragraph, optional detail, optional legend lines.
pub(crate) fn help_stack(
    testid: &'static str,
    lead: &'static str,
    detail: Option<&'static str>,
    legend: &'static [&'static str],
) -> impl IntoView {
    view! {
        <div data-testid=testid>
            <Flex vertical=true gap=SpacingSize::Size80.flex_gap()>
                <Body1>{lead}</Body1>
                {detail.map(|d| view! { <Caption1>{d}</Caption1> })}
                {legend
                    .iter()
                    .copied()
                    .map(|line| view! { <Caption1>{line}</Caption1> })
                    .collect_view()}
            </Flex>
        </div>
    }
}

/// Force-link Gauge's Help spotlight inventory into the host binary.
///
/// Empty body; `#[help_spotlight_step]` submissions in child modules are retained
/// when this crate is linked and this function is called from routes or the host.
pub const fn ensure_help_steps_linked() {}
