//! Access requests: Waiting for you and Your requests, each a table with who asked, what
//! for, the start of their reason, when, and the status in words.
//!
//! Search, status and for filters live in the URL and narrow the loaded rows; switching tabs
//! never refetches.

use std::collections::HashMap;
use std::sync::Arc;

use gauge::types::{
    PermissionRequestRowDto, PermissionRequestStatusDto, PermissionRequestTargetKind,
};
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;
use uf_product::components::{
    Body1, Caption1, ContentContainer, EmptyState, Persona, PersonaConfig, PersonaSecondaryText,
    TextTag, Title2,
};
use uf_product::primitives::{
    use_breakpoint_down, Badge, BadgeAppearance, BadgeColor, BadgeSize, Breakpoint, DataRecord,
    DataTable, DataTableColumnDef, DataTableEvents, DataTableFeatures, DataTableNoResultsView,
    DataTableRowModel, DataTableSource, Field, Flex, FlexAlign, FlexGap, FlexWrap, Input,
    InputAppearance, ListViewConfig, PagingMode, Select, SkeletonItem, Tab, TabList,
};

use crate::components::error_bar::ErrorBar;
use crate::pages::shared::query_state::use_query_signal;
use crate::pages::shared::reviewers::{reviewers_for, reviewers_of};
use crate::pages::shared::words::{for_line, plain_name, reason_start, status_words, when_view};
use crate::server::{list_my_permission_requests, list_review_permission_requests};

const fn request_target_label(kind: &PermissionRequestTargetKind) -> &'static str {
    match kind {
        PermissionRequestTargetKind::Permission => "Permission",
        PermissionRequestTargetKind::Group => "Group",
    }
}

/// Requests-list filters, each kept in the URL (`q`, `status`, `for`). Empty means "any".
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct RequestFilters {
    /// Matches the requester's name, the target label, or the service user's handle or owner.
    search: String,
    /// `pending`, `approved` or `denied`.
    status: String,
    /// `people` (no service user) or `service_users`.
    for_kind: String,
}

/// Handle of the service user a request is for, when it names one.
fn on_behalf_of_handle(row: &PermissionRequestRowDto) -> Option<&str> {
    row.on_behalf_of
        .as_ref()
        .map(|principal| principal.label.as_str())
}

fn request_matches(row: &PermissionRequestRowDto, filters: &RequestFilters) -> bool {
    let status_ok = match filters.status.as_str() {
        "pending" => row.status == PermissionRequestStatusDto::Pending,
        "approved" => row.status == PermissionRequestStatusDto::Approved,
        "denied" => row.status == PermissionRequestStatusDto::Denied,
        _ => true,
    };
    let for_ok = match filters.for_kind.as_str() {
        "people" => row.on_behalf_of.is_none(),
        "service_users" => row.on_behalf_of.is_some(),
        _ => true,
    };
    let needle = filters.search.trim().to_lowercase();
    let search_ok = needle.is_empty()
        || [
            Some(row.requester_label.as_str()),
            Some(row.target_label.as_str()),
            on_behalf_of_handle(row),
            row.on_behalf_of
                .as_ref()
                .and_then(|p| p.owner_label.as_deref()),
        ]
        .into_iter()
        .flatten()
        .any(|field| field.to_lowercase().contains(&needle));
    status_ok && for_ok && search_ok
}

/// The text under a requester's name: "for @release-bot, owned by Sol Bergström" for a
/// service user, otherwise "Person".
#[must_use]
pub fn who_secondary(row: &PermissionRequestRowDto) -> String {
    row.on_behalf_of
        .as_ref()
        .map_or_else(|| "Person".to_string(), for_line)
}

/// Tab value for requests waiting for the viewer.
const WAITING_TAB: &str = "waiting";
/// Tab value for the viewer's own requests.
const MINE_TAB: &str = "mine";

fn table_rows(
    rows: &[PermissionRequestRowDto],
    filters: &RequestFilters,
) -> Vec<DataTableRowModel> {
    rows.iter()
        .filter(|row| request_matches(row, filters))
        .map(|row| {
            let mut cells = HashMap::new();
            cells.insert("who".into(), plain_name(&row.requester_label));
            cells.insert("target".into(), row.target_label.clone());
            cells.insert("reason".into(), reason_start(&row.reason, 80));
            cells.insert("asked".into(), row.created_at.clone());
            cells.insert("status".into(), String::new());
            DataTableRowModel::from_text_cells(row.id.clone(), cells)
        })
        .collect()
}

/// One tab's requests as a table, narrowed by `filters`.
#[component]
fn RequestTable(
    rows: Vec<PermissionRequestRowDto>,
    filters: Signal<RequestFilters>,
    /// On Waiting for you the viewer is the reviewer, so the status says "Waiting" alone.
    waiting_tab: bool,
    empty_message: &'static str,
    empty_description: &'static str,
) -> impl IntoView {
    if rows.is_empty() {
        return view! {
            <div data-testid="gauge-requests-empty">
                <EmptyState message=empty_message description=empty_description />
            </div>
        }
        .into_any();
    }
    let all = StoredValue::new(rows.clone());
    let by_id = StoredValue::new(
        rows.iter()
            .map(|r| (r.id.clone(), r.clone()))
            .collect::<HashMap<_, _>>(),
    );
    let reviewers = reviewers_for(Signal::derive(move || all.get_value()));
    let items = RwSignal::new(Vec::new());
    Effect::new(move |_| {
        let f = filters.get();
        items.set(all.with_value(|rows| table_rows(rows, &f)));
    });
    let navigate = use_navigate();
    let on_row_click = Callback::new(move |(id,): (String,)| {
        navigate(
            &format!("/permission/requests/{id}"),
            NavigateOptions::default(),
        );
    });
    let row_of = move |id: &str| by_id.with_value(|m| m.get(id).cloned());
    let is_narrow = use_breakpoint_down(Breakpoint::Md);
    let columns = vec![
        DataTableColumnDef::new("who", "Who").with_cell_view(Arc::new(move |r: DataRecord| {
            row_of(&r.id).map_or_else(|| view! { <span /> }.into_any(), |row| view! {
                <Persona config=PersonaConfig::named(plain_name(&row.requester_label))>
                    <PersonaSecondaryText slot>{who_secondary(&row)}</PersonaSecondaryText>
                </Persona>
            }.into_any())
        })),
        DataTableColumnDef::new("target", "Asking for").with_cell_view(Arc::new(move |r: DataRecord| {
            row_of(&r.id).map_or_else(|| view! { <span /> }.into_any(), |row| view! {
                <Flex vertical=true gap=FlexGap::Size(0)>
                    <A href=format!("/permission/requests/{}", row.id)>{row.target_label.clone()}</A>
                    <Caption1>{request_target_label(&row.target_kind)}</Caption1>
                </Flex>
            }.into_any())
        })),
        DataTableColumnDef::new("reason", "Reason"),
        DataTableColumnDef::new("asked", "Asked").with_cell_view(Arc::new(move |r: DataRecord| {
            row_of(&r.id).map_or_else(|| view! { <span /> }.into_any(), |row| when_view(row.created_at).into_any())
        })),
        DataTableColumnDef::new("status", "Status").with_cell_view(Arc::new(move |r: DataRecord| {
            row_of(&r.id).map_or_else(|| view! { <span /> }.into_any(), |row| {
                let color = match row.status {
                    PermissionRequestStatusDto::Pending => BadgeColor::Warning,
                    PermissionRequestStatusDto::Approved => BadgeColor::Success,
                    PermissionRequestStatusDto::Denied => BadgeColor::Danger,
                    PermissionRequestStatusDto::Withdrawn => BadgeColor::Subtle,
                };
                view! {
                    <span data-testid="gauge-request-status">
                        <Badge appearance=BadgeAppearance::Tint color=color size=BadgeSize::Small>
                            {move || if waiting_tab && row.status == PermissionRequestStatusDto::Pending {
                                "Waiting".to_string()
                            } else {
                                status_words(&row.status, reviewers_of(reviewers, &row).as_deref())
                            }}
                        </Badge>
                    </span>
                }.into_any()
            })
        })),
    ];
    view! {
        <DataTable
            data_source=DataTableSource::Client(items)
            paging=PagingMode::Paged
            auto_row_height=true
            features=if is_narrow.get_untracked() { DataTableFeatures::LIST_VIEW } else { DataTableFeatures::empty() }
            list_view=ListViewConfig::new("target").with_secondary_fields(vec!["who".into(), "status".into()])
            columns=columns
            events=DataTableEvents {
                on_row_click: Some(on_row_click),
                ..Default::default()
            }
        >
            <DataTableNoResultsView slot>
                <div data-testid="gauge-requests-no-match">
                    <EmptyState message="No requests match" description="Search looks at people, groups, permissions and service users." />
                </div>
            </DataTableNoResultsView>
        </DataTable>
    }
    .into_any()
}

/// Access requests: Waiting for you (requests the viewer can answer) and Your requests.
#[component]
pub fn RequestsIndexPage() -> impl IntoView {
    let search = use_query_signal("q", None);
    let status = use_query_signal("status", None);
    let for_kind = use_query_signal("for", None);
    let tab = use_query_signal("tab", None);
    let selected_tab = RwSignal::new(if tab.get_untracked() == MINE_TAB {
        MINE_TAB.to_string()
    } else {
        WAITING_TAB.to_string()
    });
    Effect::new(move |_| {
        let t = selected_tab.get();
        tab.set(if t == WAITING_TAB { String::new() } else { t });
    });
    let filters = Signal::derive(move || RequestFilters {
        search: search.get(),
        status: status.get(),
        for_kind: for_kind.get(),
    });
    let refresh = RwSignal::new(0u32);
    let my_requests = Resource::new(move || refresh.get(), |_| list_my_permission_requests());
    let review_requests =
        Resource::new(move || refresh.get(), |_| list_review_permission_requests());
    let waiting_count = move || {
        review_requests.get().and_then(Result::ok).map(|rows| {
            rows.iter()
                .filter(|r| r.status == PermissionRequestStatusDto::Pending)
                .count()
        })
    };
    let skeleton = || {
        view! {
            <div data-testid="gauge-requests-skeleton">
                <Flex vertical=true gap=FlexGap::Small>
                    {(0..4).map(|_| view! { <SkeletonItem width="100%".to_string() /> }).collect_view()}
                </Flex>
            </div>
        }
    };

    view! {
        <div id="gauge-requests-page">
        <ContentContainer max_width="1100px" data_testid="gauge-requests-index">
            <Flex vertical=true gap=FlexGap::Medium>
                <Flex vertical=true gap=FlexGap::Small>
                    <Title2 tag=TextTag::H1>"Requests"</Title2>
                    <Body1>"Asks for access to the groups and permissions you own, and the ones you've made."</Body1>
                </Flex>
                <div id="gauge-requests-tabs">
                    <TabList selected_value=selected_tab>
                        <Tab value=WAITING_TAB>
                            {move || match waiting_count() {
                                Some(n) if n > 0 => format!("Waiting for you ({n})"),
                                _ => "Waiting for you".to_string(),
                            }}
                        </Tab>
                        <Tab value=MINE_TAB>"Your requests"</Tab>
                    </TabList>
                </div>
                <div id="gauge-requests-filters">
                    <Flex gap=FlexGap::Medium wrap=FlexWrap::Wrap align=FlexAlign::End>
                        <Field label="Search">
                            <Input
                                bind=search
                                appearance=InputAppearance::with_placeholder("Search by person, group or permission")
                            />
                        </Field>
                        <div id="gauge-requests-filter-status">
                        <Field label="Status">
                            <Select bind=status>
                                <option value="">"Any"</option>
                                <option value="pending">"Waiting"</option>
                                <option value="approved">"Approved"</option>
                                <option value="denied">"Denied"</option>
                            </Select>
                        </Field>
                        </div>
                        <div id="gauge-requests-filter-for">
                        <Field label="For">
                            <Select bind=for_kind>
                                <option value="">"Anyone"</option>
                                <option value="people">"People"</option>
                                <option value="service_users">"Service users"</option>
                            </Select>
                        </Field>
                        </div>
                    </Flex>
                </div>
                <Show when=move || selected_tab.get() == WAITING_TAB>
                    <div id="gauge-requests-needs-review">
                        <Transition fallback=skeleton>
                            {move || review_requests.get().map(|result| match result {
                                Ok(rows) => view! {
                                    <RequestTable
                                        rows=rows
                                        filters=filters
                                        waiting_tab=true
                                        empty_message="Nothing is waiting for you"
                                        empty_description="When someone asks to join a group or for a permission you own, it shows here and you get a notification."
                                    />
                                }.into_any(),
                                Err(err) => view! {
                                    <ErrorBar error=err.to_string() what="requests" on_retry=Callback::new(move |()| refresh.update(|n| *n += 1)) />
                                }.into_any(),
                            })}
                        </Transition>
                    </div>
                </Show>
                <Show when=move || selected_tab.get() == MINE_TAB>
                    <div id="gauge-requests-mine">
                        <Transition fallback=skeleton>
                            {move || my_requests.get().map(|result| match result {
                                Ok(rows) => view! {
                                    <RequestTable
                                        rows=rows
                                        filters=filters
                                        waiting_tab=false
                                        empty_message="You haven't asked for anything yet"
                                        empty_description="Open a group or permission and choose Request access. Its owners review it."
                                    />
                                }.into_any(),
                                Err(err) => view! {
                                    <ErrorBar error=err.to_string() what="your requests" on_retry=Callback::new(move |()| refresh.update(|n| *n += 1)) />
                                }.into_any(),
                            })}
                        </Transition>
                    </div>
                </Show>
            </Flex>
        </ContentContainer>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use gauge::types::{PrincipalKind, PrincipalRefDto};

    use super::*;

    fn row(
        status: PermissionRequestStatusDto,
        for_handle: Option<&str>,
    ) -> PermissionRequestRowDto {
        PermissionRequestRowDto {
            id: "r1".into(),
            target_kind: PermissionRequestTargetKind::Group,
            target_id: "g1".into(),
            target_label: "Service user creators".into(),
            requestor_user_id: "requestor".into(),
            approver_user_id: None,
            reason: "need it".into(),
            status,
            created_at: String::new(),
            updated_at: String::new(),
            can_review: false,
            requester_label: "Rae Requestor".into(),
            decided_by: None,
            decided_by_label: None,
            decided_at: None,
            deny_reason: None,
            on_behalf_of: for_handle.map(|handle| PrincipalRefDto {
                kind: PrincipalKind::ServiceUser,
                id: "su1".into(),
                label: handle.into(),
                owner_label: None,
            }),
        }
    }

    fn filters(search: &str, status: &str, for_kind: &str) -> RequestFilters {
        RequestFilters {
            search: search.into(),
            status: status.into(),
            for_kind: for_kind.into(),
        }
    }

    #[test]
    fn empty_filters_match_everything() {
        let pending = row(PermissionRequestStatusDto::Pending, None);
        assert!(request_matches(&pending, &RequestFilters::default()));
    }

    #[test]
    fn status_filter_narrows() {
        let pending = row(PermissionRequestStatusDto::Pending, None);
        assert!(request_matches(&pending, &filters("", "pending", "")));
        assert!(!request_matches(&pending, &filters("", "approved", "")));
        assert!(!request_matches(&pending, &filters("", "denied", "")));
    }

    #[test]
    fn for_filter_splits_people_and_service_users() {
        let person = row(PermissionRequestStatusDto::Pending, None);
        let agent = row(PermissionRequestStatusDto::Pending, Some("spectra-agent"));
        assert!(request_matches(&person, &filters("", "", "people")));
        assert!(!request_matches(&person, &filters("", "", "service_users")));
        assert!(request_matches(&agent, &filters("", "", "service_users")));
        assert!(!request_matches(&agent, &filters("", "", "people")));
    }

    #[test]
    fn search_matches_requester_target_or_handle() {
        let agent = row(PermissionRequestStatusDto::Pending, Some("spectra-agent"));
        assert!(request_matches(&agent, &filters("RAE", "", "")));
        assert!(request_matches(&agent, &filters("creators", "", "")));
        assert!(request_matches(&agent, &filters("spectra", "", "")));
        assert!(!request_matches(&agent, &filters("deployers", "", "")));
    }

    #[test]
    fn who_line_names_the_service_user_and_owner() {
        let mut agent = row(PermissionRequestStatusDto::Approved, Some("spectra-agent"));
        assert_eq!(who_secondary(&agent), "for @spectra-agent");
        if let Some(p) = agent.on_behalf_of.as_mut() {
            p.owner_label = Some("Rae Requestor".into());
        }
        assert_eq!(
            who_secondary(&agent),
            "for @spectra-agent, owned by Rae Requestor"
        );
        let person = row(PermissionRequestStatusDto::Pending, None);
        assert_eq!(who_secondary(&person), "Person");
    }
}
