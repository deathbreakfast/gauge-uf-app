//! Gauge's change log, for Gauge admins: every change across Gauge, newest first, by name.
//!
//! What changed, When and the search box go to the server and live in the URL (`?kind=`,
//! `?when=`, `?q=`); Changed by narrows the loaded rows. Everyone else gets a page saying
//! where owners see changes instead.

use std::collections::HashMap;
use std::sync::Arc;

use gauge::types::{ActivityFilter, ActivityKind, ActivityRow};
use leptos::prelude::*;
use leptos::task::spawn_local_scoped;
use uf_product::components::{
    Body1, Caption1, ContentContainer, EmptyState, EmptyStateCallToAction, Persona, PersonaConfig,
    TextTag, Title2,
};
use uf_product::primitives::{
    use_breakpoint_down, Breakpoint, Button, ButtonAppearance, DataRecord, DataTable,
    DataTableColumnDef, DataTableFeatures, DataTableNoResultsView, DataTableRowModel,
    DataTableSource, Field, Flex, FlexAlign, FlexGap, FlexJustify, FlexWrap, Input,
    InputAppearance, Link, ListViewConfig, PagingMode, Select, SkeletonItem,
};

use crate::components::error_bar::{ErrorBar, MaybeErrorBar};
use crate::pages::person_detail::record_href;
use crate::pages::shared::denied::DeniedPage;
use crate::pages::shared::query_state::use_query_signal;
use crate::pages::shared::words::{plain_name, when_view};
use crate::server::surfaces::get_activity_window;
use crate::server::viewer::use_viewer_access;

/// What changed, as the filter offers it: URL value, label, kind.
const KINDS: [(&str, &str, ActivityKind); 6] = [
    ("membership", "Membership", ActivityKind::Membership),
    ("grants", "Allow lists", ActivityKind::Grant),
    ("groups", "Groups and owners", ActivityKind::Group),
    ("permissions", "Permissions", ActivityKind::Permission),
    ("domains", "Domains", ActivityKind::Domain),
    ("requests", "Requests", ActivityKind::Request),
];

/// When, as the filter offers it: URL value, label, days back.
const WHEN: [(&str, &str, i64); 4] = [
    ("1d", "Last 24 hours", 1),
    ("7d", "Last 7 days", 7),
    ("30d", "Last 30 days", 30),
    ("90d", "Last 90 days", 90),
];

/// The filter word for a change's kind ("Membership", "Allow lists").
#[must_use]
pub fn kind_label(kind: ActivityKind) -> &'static str {
    KINDS
        .iter()
        .find(|(_, _, k)| *k == kind)
        .map_or("Change", |(_, label, _)| label)
}

/// The kinds a `?kind=` value asks for; empty means every kind.
#[must_use]
pub fn kinds_for(value: &str) -> Vec<ActivityKind> {
    KINDS
        .iter()
        .filter(|(v, _, _)| *v == value)
        .map(|(_, _, k)| *k)
        .collect()
}

/// How many days back a `?when=` value reaches; `None` for any time.
#[must_use]
pub fn days_for(value: &str) -> Option<i64> {
    WHEN.iter()
        .find(|(v, _, _)| *v == value)
        .map(|(_, _, d)| *d)
}

/// One CSV field, quoted when it holds a comma, quote or line break.
fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// The rows as CSV, with a header line.
#[must_use]
pub fn to_csv(rows: &[ActivityRow]) -> String {
    let mut out = String::from("When,Who,What changed,Where,Kind\n");
    for row in rows {
        let line = [
            row.at.to_rfc3339(),
            plain_name(&row.actor_label),
            row.summary.clone(),
            row.record.label.clone(),
            kind_label(row.kind).to_string(),
        ]
        .iter()
        .map(|f| csv_field(f))
        .collect::<Vec<_>>()
        .join(",");
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// A `data:` link for `csv`, so Download CSV works without a server round trip.
#[must_use]
pub fn csv_href(csv: &str) -> String {
    let mut out = String::from("data:text/csv;charset=utf-8,");
    for byte in csv.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// The Activity page.
#[component]
pub fn ActivityPage() -> impl IntoView {
    let viewer = use_viewer_access();
    view! {
        <ContentContainer max_width="1100px" data_testid="gauge-activity">
            <Suspense fallback=|| view! { <SkeletonItem width="100%".to_string() /> }>
                {move || viewer.get().map(|v| match v {
                    Ok(v) if v.gauge_admin => view! { <ActivityLog /> }.into_any(),
                    _ => view! {
                        <DeniedPage
                            heading="Activity is for Gauge admins"
                            text="Owners see the changes to their own groups and permissions on each one's Activity tab."
                            action="Open your groups"
                            action_href="/permission/groups"
                            parent=("Overview", "/permission")
                            current="Activity"
                        />
                    }.into_any(),
                })}
            </Suspense>
        </ContentContainer>
    }
}

#[component]
fn ActivityLog() -> impl IntoView {
    let kind = use_query_signal("kind", None);
    let when = use_query_signal("when", None);
    let search = use_query_signal("q", None);
    let actor = RwSignal::new(String::new());
    let refresh = RwSignal::new(0u32);
    let first = Resource::new(
        move || (kind.get(), when.get(), search.get(), refresh.get()),
        |(kind, when, q, _)| async move {
            let q = q.trim().to_string();
            let filter = ActivityFilter {
                kinds: kinds_for(&kind),
                query: (!q.is_empty()).then_some(q),
                ..Default::default()
            };
            let days = days_for(&when);
            get_activity_window(filter.clone(), days, None)
                .await
                .map(|page| ((filter, days), page))
        },
    );
    let more = RwSignal::new(Vec::<ActivityRow>::new());
    let next = RwSignal::new(None::<((ActivityFilter, Option<i64>), String)>);
    let loading_more = RwSignal::new(false);
    let more_error = RwSignal::new(None::<String>);
    let is_narrow = use_breakpoint_down(Breakpoint::Md);
    // Every loaded row, for Changed by's choices and the CSV.
    let loaded = RwSignal::new(Vec::<ActivityRow>::new());
    let actors = Memo::new(move |_| {
        let mut names: Vec<String> =
            loaded.with(|rows| rows.iter().map(|r| plain_name(&r.actor_label)).collect());
        names.sort();
        names.dedup();
        names
    });
    let visible = Memo::new(move |_| {
        let who = actor.get();
        loaded.with(|rows| {
            rows.iter()
                .filter(|r| who.is_empty() || plain_name(&r.actor_label) == who)
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    let csv = Signal::derive(move || csv_href(&to_csv(&visible.get())));
    let clear_filters = Callback::new(move |()| {
        kind.set(String::new());
        when.set(String::new());
        search.set(String::new());
        actor.set(String::new());
    });

    let skeleton = || {
        view! {
            <div data-testid="gauge-activity-skeleton">
                <Flex vertical=true gap=FlexGap::Small>
                    {(0..6).map(|_| view! { <SkeletonItem width="100%".to_string() /> }).collect_view()}
                </Flex>
            </div>
        }
    };

    view! {
        <Flex vertical=true gap=FlexGap::Medium>
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center gap=FlexGap::Medium wrap=FlexWrap::Wrap>
                <Flex vertical=true gap=FlexGap::Small>
                    <Title2 tag=TextTag::H1>"Activity"</Title2>
                    <Body1>"Who changed access, what changed and when. Gauge keeps every change; none expire."</Body1>
                </Flex>
                <div id="gauge-activity-csv">
                    {move || view! { <Link href=csv.get() attr:download="gauge-activity.csv">"Download CSV"</Link> }}
                </div>
            </Flex>
            <div id="gauge-activity-filters">
                <Flex gap=FlexGap::Medium wrap=FlexWrap::Wrap align=FlexAlign::End>
                    <div data-testid="gauge-activity-filter-kind">
                        <Field label="What changed">
                            <Select bind=kind>
                                <option value="">"Any"</option>
                                {KINDS.iter().map(|(v, label, _)| view! { <option value=*v>{*label}</option> }).collect_view()}
                            </Select>
                        </Field>
                    </div>
                    <div data-testid="gauge-activity-filter-when">
                        <Field label="When">
                            <Select bind=when>
                                <option value="">"Any time"</option>
                                {WHEN.iter().map(|(v, label, _)| view! { <option value=*v>{*label}</option> }).collect_view()}
                            </Select>
                        </Field>
                    </div>
                    <div data-testid="gauge-activity-filter-actor">
                        <Field label="Changed by">
                            <Select bind=actor>
                                <option value="">"Anyone"</option>
                                <For each=move || actors.get() key=|name| name.clone() let:name>
                                    <option value=name.clone()>{name.clone()}</option>
                                </For>
                            </Select>
                        </Field>
                    </div>
                    <div data-testid="gauge-activity-search">
                        <Field label="Search">
                            <Input bind=search appearance=InputAppearance::with_placeholder("Search by person, group or permission") />
                        </Field>
                    </div>
                </Flex>
            </div>
            <div id="gauge-activity-list">
                <Transition fallback=skeleton>
                    {move || first.get().map(|result| match result {
                        Ok((filter, page)) => {
                            more.set(Vec::new());
                            next.set(page.next_cursor.clone().map(|c| (filter, c)));
                            let base = page.items;
                            Effect::new(move |_| {
                                let mut rows = base.clone();
                                rows.extend(more.get());
                                loaded.set(rows);
                            });
                            let items = RwSignal::new(Vec::new());
                            Effect::new(move |_| {
                                items.set(visible.get().iter().enumerate().map(|(i, r)| {
                                    let mut cells = HashMap::new();
                                    cells.insert("when".into(), r.at.to_rfc3339());
                                    cells.insert("who".into(), plain_name(&r.actor_label));
                                    cells.insert("what".into(), r.summary.clone());
                                    cells.insert("where".into(), r.record.label.clone());
                                    DataTableRowModel::from_text_cells(i.to_string(), cells)
                                }).collect::<Vec<_>>());
                            });
                            let row_at = move |id: &str| {
                                id.parse::<usize>().ok().and_then(|i| visible.with_untracked(|rows| rows.get(i).cloned()))
                            };
                            let columns = vec![
                                DataTableColumnDef::new("when", "When").with_cell_view(Arc::new(move |r: DataRecord| {
                                    row_at(&r.id).map_or_else(|| view! { <span /> }.into_any(), |row| when_view(row.at.to_rfc3339()).into_any())
                                })),
                                DataTableColumnDef::new("who", "Who").with_cell_view(Arc::new(move |r: DataRecord| {
                                    row_at(&r.id).map_or_else(|| view! { <span /> }.into_any(), |row| view! {
                                        <Persona config=PersonaConfig::named(plain_name(&row.actor_label)) />
                                    }.into_any())
                                })),
                                DataTableColumnDef::new("what", "What changed"),
                                DataTableColumnDef::new("where", "Where").with_cell_view(Arc::new(move |r: DataRecord| {
                                    row_at(&r.id).map_or_else(|| view! { <span /> }.into_any(), |row| view! {
                                        <Flex vertical=true gap=FlexGap::Size(0)>
                                            <Link href=record_href(&row.record)>{row.record.label.clone()}</Link>
                                            <Caption1>{kind_label(row.kind)}</Caption1>
                                        </Flex>
                                    }.into_any())
                                })),
                            ];
                            view! {
                                <div data-testid="gauge-activity-table">
                                    <DataTable
                                        data_source=DataTableSource::Client(items)
                                        paging=PagingMode::Paged
                                        auto_row_height=true
                                        features=if is_narrow.get_untracked() { DataTableFeatures::LIST_VIEW } else { DataTableFeatures::empty() }
                                        list_view=ListViewConfig::new("what").with_secondary_fields(vec!["who".into(), "where".into()])
                                        columns=columns
                                    >
                                        <DataTableNoResultsView slot>
                                            <div data-testid="gauge-activity-no-match">
                                                <EmptyState
                                                    message="No changes match"
                                                    description="Try a longer time range or clear the filters."
                                                >
                                                    <EmptyStateCallToAction slot:call_to_action>
                                                        <Button on_click=Callback::new(move |_| clear_filters.run(()))>"Clear filters"</Button>
                                                    </EmptyStateCallToAction>
                                                </EmptyState>
                                            </div>
                                        </DataTableNoResultsView>
                                    </DataTable>
                                </div>
                            }.into_any()
                        }
                        Err(err) => view! {
                            <ErrorBar error=err.to_string() what="activity" on_retry=Callback::new(move |()| refresh.update(|n| *n += 1)) />
                        }.into_any(),
                    })}
                </Transition>
            </div>
            <Show when=move || next.get().is_some()>
                <div id="gauge-activity-more">
                    <Button
                        appearance=ButtonAppearance::Secondary
                        disabled=Signal::derive(move || loading_more.get())
                        on_click=Callback::new(move |_| {
                            let Some(((filter, days), cursor)) = next.get_untracked() else {
                                return;
                            };
                            loading_more.set(true);
                            spawn_local_scoped(async move {
                                match get_activity_window(filter.clone(), days, Some(cursor)).await {
                                    Ok(page) => {
                                        more.update(|rows| rows.extend(page.items));
                                        next.set(page.next_cursor.map(|c| ((filter, days), c)));
                                        more_error.set(None);
                                    }
                                    Err(e) => more_error.set(Some(e.to_string())),
                                }
                                loading_more.set(false);
                            });
                        })
                    >
                        "Load older changes"
                    </Button>
                </div>
            </Show>
            <MaybeErrorBar error=more_error what="older changes" />
        </Flex>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gauge::types::{RecordKind, RecordRef};

    fn row(summary: &str) -> ActivityRow {
        ActivityRow {
            at: chrono::DateTime::from_timestamp(1_763_632_800, 0).unwrap_or_default(),
            actor_label: "Ines Okoro (abc123)".into(),
            summary: summary.into(),
            record: RecordRef {
                kind: RecordKind::Group,
                id: "g1".into(),
                label: "Release Captains".into(),
            },
            kind: ActivityKind::Membership,
        }
    }

    #[test]
    fn filters_map_to_kinds_and_days_happy() {
        assert_eq!(kinds_for("membership"), vec![ActivityKind::Membership]);
        assert_eq!(days_for("7d"), Some(7));
        assert_eq!(kind_label(ActivityKind::Grant), "Allow lists");
    }

    #[test]
    fn unknown_filters_match_everything_sad() {
        assert!(kinds_for("").is_empty());
        assert!(kinds_for("bogus").is_empty());
        assert_eq!(days_for(""), None);
    }

    #[test]
    fn csv_quotes_commas_and_names_people_happy() {
        let csv = to_csv(&[row("Removed Ravi, then re-added")]);
        assert!(csv.starts_with("When,Who,What changed,Where,Kind\n"));
        assert!(
            csv.contains("Ines Okoro,\"Removed Ravi, then re-added\",Release Captains,Membership")
        );
        assert!(csv_href("a b").ends_with("a%20b"));
    }
}
