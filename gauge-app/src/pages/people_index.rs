//! People and service users Gauge knows, for Gauge admins.
//!
//! Search runs on the server by name and @handle and is kept in the URL (`?q=`). The tab
//! (`?tab=service-users`) only filters the loaded rows, so switching never refetches. Load
//! more fetches the next page and adds it to the table.

use std::collections::HashMap;
use std::sync::Arc;

use gauge::types::{PersonRow, PrincipalKind};
use leptos::prelude::*;
use leptos::task::spawn_local_scoped;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;
use uf_product::components::{
    Body1, Caption1, ContentContainer, EmptyState, EmptyStateCallToAction, Persona, PersonaConfig,
    PersonaSecondaryText, TextTag, Title2,
};
use uf_product::primitives::{
    use_breakpoint_down, Breakpoint, Button, ButtonAppearance, DataRecord, DataTable,
    DataTableColumnDef, DataTableEvents, DataTableFeatures, DataTableNoResultsView,
    DataTableRowModel, DataTableSource, Field, Flex, FlexGap, Input, InputAppearance,
    ListViewConfig, PagingMode, SkeletonItem, Tab, TabList,
};

use crate::components::error_bar::ErrorBar;
use crate::pages::shared::denied::DeniedPage;
use crate::pages::shared::query_state::use_query_signal;
use crate::pages::shared::words::plain_name;
use crate::server::get_people;
use crate::server::viewer::use_viewer_access;

/// Tab value for people.
const PEOPLE_TAB: &str = "people";
/// Tab value for service users (also the breadcrumb link's `?tab=`).
pub const SERVICE_USERS_TAB: &str = "service-users";

/// Where a row opens: a person's page, or a service user's.
#[must_use]
pub fn row_href(row: &PersonRow) -> String {
    match row.principal.kind {
        PrincipalKind::ServiceUser => format!("/permission/service-users/{}", row.principal.id),
        _ => format!("/permission/people/{}", row.principal.id),
    }
}

/// Whether `row` belongs on `tab`.
#[must_use]
pub fn on_tab(row: &PersonRow, tab: &str) -> bool {
    let service_user = row.principal.kind == PrincipalKind::ServiceUser;
    if tab == SERVICE_USERS_TAB {
        service_user
    } else {
        !service_user
    }
}

/// Names joined with ", ", or "None".
#[must_use]
pub fn names_or_none(names: impl Iterator<Item = String>) -> String {
    let joined = names.collect::<Vec<_>>().join(", ");
    if joined.is_empty() {
        "None".to_string()
    } else {
        joined
    }
}

fn row_key(row: &PersonRow) -> String {
    format!("{:?}:{}", row.principal.kind, row.principal.id)
}

fn table_rows(rows: &[PersonRow], tab: &str) -> Vec<DataTableRowModel> {
    rows.iter()
        .filter(|r| on_tab(r, tab))
        .map(|r| {
            let mut cells = HashMap::new();
            cells.insert("name".into(), r.principal.label.clone());
            cells.insert(
                "groups".into(),
                names_or_none(r.groups.iter().map(|g| g.name.clone())),
            );
            cells.insert(
                "direct".into(),
                names_or_none(r.direct_permissions.iter().map(|p| p.display_name.clone())),
            );
            DataTableRowModel::from_text_cells(row_key(r), cells)
        })
        .collect()
}

/// The People page.
#[component]
pub fn PeopleIndexPage() -> impl IntoView {
    let viewer = use_viewer_access();
    view! {
        <ContentContainer max_width="1100px" data_testid="gauge-people-index">
            <Suspense fallback=|| view! { <SkeletonItem width="100%".to_string() /> }>
                {move || viewer.get().map(|v| match v {
                    Ok(v) if v.gauge_admin => view! { <PeopleList /> }.into_any(),
                    _ => view! {
                        <DeniedPage
                            heading="People is for Gauge admins"
                            text="Gauge admins see everyone's access here. Your own access is on your page."
                            action="See your access"
                            action_href="/permission/people/me"
                            parent=("Overview", "/permission")
                            current="People"
                        />
                    }.into_any(),
                })}
            </Suspense>
        </ContentContainer>
    }
}

#[component]
fn PeopleList() -> impl IntoView {
    let search = use_query_signal("q", None);
    let tab = use_query_signal("tab", None);
    let selected_tab = RwSignal::new(if tab.get_untracked() == SERVICE_USERS_TAB {
        SERVICE_USERS_TAB.to_string()
    } else {
        PEOPLE_TAB.to_string()
    });
    Effect::new(move |_| {
        let t = selected_tab.get();
        tab.set(if t == PEOPLE_TAB { String::new() } else { t });
    });
    let refresh = RwSignal::new(0u32);
    let first = Resource::new(
        move || (search.get(), refresh.get()),
        |(q, _)| async move {
            let q = q.trim().trim_start_matches('@').to_string();
            get_people((!q.is_empty()).then_some(q), None).await
        },
    );
    // Rows past the first page, from Load more.
    let more = RwSignal::new(Vec::<PersonRow>::new());
    let next_cursor = RwSignal::new(None::<String>);
    let loading_more = RwSignal::new(false);
    let more_error = RwSignal::new(None::<String>);
    let navigate = use_navigate();
    let is_narrow = use_breakpoint_down(Breakpoint::Md);

    let skeleton = || {
        view! {
            <div data-testid="gauge-people-skeleton">
                <Flex vertical=true gap=FlexGap::Small>
                    {(0..5).map(|_| view! { <SkeletonItem width="100%".to_string() /> }).collect_view()}
                </Flex>
            </div>
        }
    };

    view! {
        <Flex vertical=true gap=FlexGap::Medium>
            <Flex vertical=true gap=FlexGap::Small>
                <Title2 tag=TextTag::H1>"People"</Title2>
                <Body1>"Everyone with access in Gauge. Open a person to see what they can do and why."</Body1>
            </Flex>
            <div id="gauge-people-tabs">
                <TabList selected_value=selected_tab>
                    <Tab value=PEOPLE_TAB>"People"</Tab>
                    <Tab value=SERVICE_USERS_TAB>"Service users"</Tab>
                </TabList>
            </div>
            <div id="gauge-people-search">
                <Field label="Search">
                    <Input bind=search appearance=InputAppearance::with_placeholder("Search by name or @handle") />
                </Field>
            </div>
            <div id="gauge-people-list">
                <Transition fallback=skeleton>
                    {move || first.get().map(|result| match result {
                        Ok(page) => {
                            more.set(Vec::new());
                            next_cursor.set(page.next_cursor.clone());
                            let base = page.items;
                            let all = Signal::derive(move || {
                                let mut rows = base.clone();
                                rows.extend(more.get());
                                rows
                            });
                            let by_key = Signal::derive(move || {
                                all.get().into_iter().map(|r| (row_key(&r), r)).collect::<HashMap<_, _>>()
                            });
                            let items = RwSignal::new(Vec::new());
                            Effect::new(move |_| items.set(table_rows(&all.get(), &selected_tab.get())));
                            let navigate = navigate.clone();
                            let on_row_click = Callback::new(move |(id,): (String,)| {
                                if let Some(row) = by_key.with_untracked(|m| m.get(&id).cloned()) {
                                    navigate(&row_href(&row), NavigateOptions::default());
                                }
                            });
                            let columns = vec![
                                DataTableColumnDef::new("name", "Name").with_cell_view(Arc::new(move |r: DataRecord| {
                                    let Some(row) = by_key.with_untracked(|m| m.get(&r.id).cloned()) else {
                                        return view! { <span /> }.into_any();
                                    };
                                    let secondary = match row.principal.kind {
                                        PrincipalKind::ServiceUser => match row.principal.owner_label.as_deref().map(plain_name) {
                                            Some(owner) => format!("@{} · owned by {owner}", row.principal.label),
                                            None => format!("@{}", row.principal.label),
                                        },
                                        _ => "Person".to_string(),
                                    };
                                    view! {
                                        <Persona config=PersonaConfig::named(plain_name(&row.principal.label))>
                                            <PersonaSecondaryText slot>{secondary}</PersonaSecondaryText>
                                        </Persona>
                                    }.into_any()
                                })),
                                DataTableColumnDef::new("groups", "Groups"),
                                DataTableColumnDef::new("direct", "Direct permissions"),
                            ];
                            view! {
                                <div data-testid="gauge-people-table">
                                    <DataTable
                                        data_source=DataTableSource::Client(items)
                                        paging=PagingMode::Paged
                                        auto_row_height=true
                                        features=if is_narrow.get_untracked() { DataTableFeatures::LIST_VIEW } else { DataTableFeatures::empty() }
                                        list_view=ListViewConfig::new("name").with_secondary_fields(vec!["groups".into()])
                                        columns=columns
                                        events=DataTableEvents {
                                            on_row_click: Some(on_row_click),
                                            ..Default::default()
                                        }
                                    >
                                        <DataTableNoResultsView slot>
                                            <div data-testid="gauge-people-no-match">
                                                <EmptyState
                                                    message="No one matches your search"
                                                    description="Search looks at names and @handles. People appear here once they sign in."
                                                >
                                                    <EmptyStateCallToAction slot:call_to_action>
                                                        <Button on_click=Callback::new(move |_| search.set(String::new()))>"Clear search"</Button>
                                                    </EmptyStateCallToAction>
                                                </EmptyState>
                                            </div>
                                        </DataTableNoResultsView>
                                    </DataTable>
                                </div>
                            }.into_any()
                        }
                        Err(err) => view! {
                            <ErrorBar error=err.to_string() what="people" on_retry=Callback::new(move |()| refresh.update(|n| *n += 1)) />
                        }.into_any(),
                    })}
                </Transition>
            </div>
            <Show when=move || next_cursor.get().is_some()>
                <div id="gauge-people-more">
                    <Flex gap=FlexGap::Small>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            disabled=Signal::derive(move || loading_more.get())
                            on_click=Callback::new(move |_| {
                                let cursor = next_cursor.get_untracked();
                                let q = search.get_untracked().trim().trim_start_matches('@').to_string();
                                loading_more.set(true);
                                spawn_local_scoped(async move {
                                    match get_people((!q.is_empty()).then_some(q), cursor).await {
                                        Ok(page) => {
                                            more.update(|rows| rows.extend(page.items));
                                            next_cursor.set(page.next_cursor);
                                            more_error.set(None);
                                        }
                                        Err(e) => more_error.set(Some(e.to_string())),
                                    }
                                    loading_more.set(false);
                                });
                            })
                        >
                            "Load more"
                        </Button>
                        <Caption1>"Showing the first page of results."</Caption1>
                    </Flex>
                </div>
            </Show>
            <crate::components::error_bar::MaybeErrorBar error=more_error what="more people" />
        </Flex>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gauge::types::{GroupRef, PrincipalRefDto};

    fn row(kind: PrincipalKind, id: &str) -> PersonRow {
        PersonRow {
            principal: PrincipalRefDto {
                kind,
                id: id.into(),
                label: id.into(),
                owner_label: None,
            },
            groups: vec![GroupRef {
                id: "g".into(),
                name: "Release Captains".into(),
            }],
            direct_permissions: Vec::new(),
        }
    }

    #[test]
    fn rows_open_their_own_page_happy() {
        assert_eq!(
            row_href(&row(PrincipalKind::User, "ravi")),
            "/permission/people/ravi"
        );
        assert_eq!(
            row_href(&row(PrincipalKind::ServiceUser, "su1")),
            "/permission/service-users/su1"
        );
    }

    #[test]
    fn tabs_split_people_and_service_users_happy() {
        let person = row(PrincipalKind::User, "ravi");
        let bot = row(PrincipalKind::ServiceUser, "su1");
        assert!(on_tab(&person, PEOPLE_TAB));
        assert!(!on_tab(&person, SERVICE_USERS_TAB));
        assert!(on_tab(&bot, SERVICE_USERS_TAB));
        assert_eq!(table_rows(&[person, bot], SERVICE_USERS_TAB).len(), 1);
    }

    #[test]
    fn empty_lists_say_none_sad() {
        assert_eq!(names_or_none(std::iter::empty()), "None");
    }
}
