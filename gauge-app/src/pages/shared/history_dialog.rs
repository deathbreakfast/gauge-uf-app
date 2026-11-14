//! Show History dialog — Orbital Dialog hosting a paginated Record History timeline.

use std::collections::HashMap;
use std::sync::Arc;

use leptos::prelude::*;
use orbital_history::{
    page_fetcher, HistoryPagingMode, HistorySource, HistoryTimeline as OrbitalHistoryTimeline,
};
use record_history_leptos::{
    map_history_page, HistoryChange, HistoryEntryView, HistoryKindEntryRow, HistoryRenderContext,
    HistoryRenderers, RECORD_HISTORY_PAGE_SIZE,
};
use uf_product::components::{Body1, Caption1};
use uf_product::primitives::{
    Avatar, AvatarConfig, Button, ButtonAppearance, Dialog, DialogActions, DialogBody,
    DialogContent, DialogSurface, DialogTitle, Flex, FlexAlign, FlexGap, Link, MessageBar,
    MessageBarIntent,
};
use valence::RecordId;

use crate::server::get_gauge_history_page_by_cursor;

const RELATION_FIELDS: &[&str] = &[
    "granted_users",
    "granted_groups",
    "member_users",
    "member_groups",
    "owner_users",
    "owner_groups",
];

fn is_relation_field(field: &str) -> bool {
    RELATION_FIELDS.contains(&field)
}

fn is_group_relation_field(field: &str) -> bool {
    matches!(field, "granted_groups" | "member_groups" | "owner_groups")
}

fn bare_principal_id(raw: &str) -> String {
    raw.split_once(':')
        .map(|(_, id)| id.to_string())
        .unwrap_or_else(|| raw.to_string())
}

fn initials_from_id(id: &str) -> String {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return "?".into();
    }
    let mut chars = trimmed.chars().filter(|c| c.is_alphanumeric());
    match (chars.next(), chars.next()) {
        (Some(a), Some(b)) => format!("{}{}", a.to_ascii_uppercase(), b.to_ascii_uppercase()),
        (Some(a), None) => a.to_ascii_uppercase().to_string(),
        _ => "?".into(),
    }
}

/// Verb for a relation add/remove row (`Added` / `Removed`), or `None` if neither.
fn relation_change_verb(old_value: &str, new_value: &str) -> Option<&'static str> {
    if old_value.is_empty() && !new_value.is_empty() {
        Some("Added")
    } else if !old_value.is_empty() && new_value.is_empty() {
        Some("Removed")
    } else {
        None
    }
}

fn is_history_acl_error(err: &ServerFnError) -> bool {
    let msg = err.to_string();
    msg.contains("Not authorized") || msg.contains("Authentication required")
}

fn gauge_history_renderers() -> HistoryRenderers {
    let mut kind_views = HashMap::new();
    kind_views.insert(
        "permission_history".into(),
        Arc::new(|ctx: HistoryRenderContext| {
            let entry = ctx.entry.clone();
            let HistoryChange::FieldDiff {
                field,
                old_value,
                new_value,
            } = &entry.change
            else {
                return None;
            };
            if !is_relation_field(field) {
                return None;
            }

            let (verb, target_raw) = match relation_change_verb(old_value, new_value) {
                Some("Added") => ("Added", new_value.clone()),
                Some("Removed") => ("Removed", old_value.clone()),
                _ => return None,
            };

            let display_id = bare_principal_id(&target_raw);
            let initials = initials_from_id(&display_id);
            let link_group = is_group_relation_field(field);
            let group_href = format!("/permission/groups/{display_id}");
            let avatar_name = display_id.clone();

            // Persona is not re-exported on uf_product; Avatar + Body1 verb (G7).
            Some(
                view! {
                    <HistoryKindEntryRow entry=entry>
                        <div data-testid="gauge-history-relation-row">
                            <Flex gap=FlexGap::Small align=FlexAlign::Center>
                                <Avatar config=AvatarConfig {
                                    initials: Some(initials),
                                    name: Some(avatar_name),
                                    size: Some(24),
                                    ..Default::default()
                                } />
                                <Body1>{verb}</Body1>
                                {
                                    if link_group {
                                        view! {
                                            <Link href=group_href>
                                                <Caption1>{display_id.clone()}</Caption1>
                                            </Link>
                                        }
                                        .into_any()
                                    } else {
                                        view! { <Caption1>{display_id}</Caption1> }.into_any()
                                    }
                                }
                            </Flex>
                        </div>
                    </HistoryKindEntryRow>
                }
                .into_any(),
            )
        }) as HistoryEntryView,
    );
    HistoryRenderers {
        kind_views,
        ..Default::default()
    }
}

/// Paginated Orbital timeline for a Gauge history source (mounted only while open).
#[component]
fn GaugeHistoryTimeline(source: RecordId) -> impl IntoView {
    let access_denied = RwSignal::new(false);
    let load_failed = RwSignal::new(false);
    let sid = source;
    let fetcher = page_fetcher(move |page| {
        let sid = sid.clone();
        async move {
            match get_gauge_history_page_by_cursor(page.offset, page.limit, sid, None, page.cursor)
                .await
            {
                Ok(page) => {
                    access_denied.set(false);
                    load_failed.set(false);
                    Ok(map_history_page(page))
                }
                Err(err) => {
                    if is_history_acl_error(&err) {
                        access_denied.set(true);
                        load_failed.set(false);
                    } else {
                        access_denied.set(false);
                        load_failed.set(true);
                    }
                    Err(err)
                }
            }
        }
    });
    let renderers = gauge_history_renderers();

    view! {
        <div
            data-testid="gauge-history-access-denied"
            hidden=move || !access_denied.get()
        >
            <MessageBar intent=MessageBarIntent::Error>
                "Not authorized to view this history"
            </MessageBar>
        </div>
        <div
            data-testid="gauge-history-load-failed"
            hidden=move || !load_failed.get()
        >
            <MessageBar intent=MessageBarIntent::Error>
                "Failed to load record history"
            </MessageBar>
        </div>
        <div
            data-testid="record-history-timeline"
            hidden=move || access_denied.get() || load_failed.get()
        >
            <OrbitalHistoryTimeline
                data_source=HistorySource::Server {
                    fetcher,
                    page_size: RECORD_HISTORY_PAGE_SIZE,
                }
                max_height="400px".to_string()
                paging=HistoryPagingMode::InfiniteScroll
                renderers=renderers
            />
        </div>
    }
}

/// Dialog hosting paginated Record History for a Gauge subject, loaded when opened.
#[component]
pub fn HistoryDialog(
    #[prop(into)] subject_kind: Signal<String>,
    #[prop(into)] subject_id: Signal<String>,
    /// Spotlight anchor on the Show History trigger (`id=`).
    #[prop(into)]
    trigger_id: &'static str,
) -> impl IntoView {
    let open = RwSignal::new(false);

    view! {
        <div id=trigger_id>
            <Button appearance=ButtonAppearance::Subtle on_click=Callback::new(move |_| open.set(true))>
                "Show History"
            </Button>
        </div>

        <Dialog open=open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>"History"</DialogTitle>
                    <DialogContent>
                        {move || {
                            if !open.get() {
                                return ().into_any();
                            }
                            let kind = subject_kind.get();
                            let id = subject_id.get();
                            if kind.is_empty() || id.is_empty() {
                                return view! {
                                    <MessageBar intent=MessageBarIntent::Info>
                                        "No history entries yet."
                                    </MessageBar>
                                }
                                .into_any();
                            }
                            let source = RecordId::new(kind, id);
                            view! { <GaugeHistoryTimeline source=source /> }.into_any()
                        }}
                    </DialogContent>
                    <DialogActions>
                        <Button appearance=ButtonAppearance::Secondary on_click=Callback::new(move |_| open.set(false))>
                            "Close"
                        </Button>
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
        </Dialog>
    }
}

#[cfg(test)]
mod relation_field_helpers {
    use super::{
        bare_principal_id, initials_from_id, is_group_relation_field, is_relation_field,
        relation_change_verb, RELATION_FIELDS,
    };

    #[test]
    fn relation_fields_include_owner_groups() {
        assert!(RELATION_FIELDS.contains(&"owner_groups"));
        assert!(is_relation_field("owner_groups"));
        assert!(is_group_relation_field("owner_groups"));
        assert!(!is_group_relation_field("owner_users"));
    }

    #[test]
    fn relation_change_verb_added_removed() {
        assert_eq!(relation_change_verb("", "user:alice"), Some("Added"));
        assert_eq!(relation_change_verb("user:alice", ""), Some("Removed"));
        assert_eq!(relation_change_verb("a", "b"), None);
        assert_eq!(relation_change_verb("", ""), None);
    }

    #[test]
    fn bare_principal_and_initials() {
        assert_eq!(bare_principal_id("user:alice"), "alice");
        assert_eq!(bare_principal_id("alice"), "alice");
        assert_eq!(initials_from_id("alice"), "AL");
        assert_eq!(initials_from_id(""), "?");
    }
}
