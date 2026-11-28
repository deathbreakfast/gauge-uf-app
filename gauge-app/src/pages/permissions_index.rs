//! The permissions list: names people read, with the key as code, the domain, the owners,
//! and for Gauge admins how many pass each check.
//!
//! Search is the table's quick search, kept in the URL (`?q=`) and run on the loaded rows,
//! so typing never refetches. A search with no match says so, apart from an empty catalog.
//! Create permission shows to Gauge admins only.

use std::collections::HashMap;
use std::sync::Arc;

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;
use uf_product::components::{
    Body1Strong, Caption1, ContentContainer, EmptyState, EmptyStateCallToAction, TextTag, Title3,
};
use uf_product::primitives::{
    use_breakpoint_down, Breakpoint, Button, ButtonAppearance, Code, DataRecord, DataTable,
    DataTableColumnDef, DataTableEvents, DataTableFeatures, DataTableNoResultsView,
    DataTableRowModel, DataTableSource, DataTableToolbarConfig, Flex, FlexAlign, FlexGap,
    FlexJustify, FlexWrap, ListViewConfig, MessageBar, MessageBarBody, MessageBarIntent,
    PagingMode, SkeletonItem,
};

use crate::components::error_bar::ErrorBar;
use crate::pages::shared::query_state::use_index_search;
use crate::server::viewer::{use_viewer_access, viewer_now};
use crate::server::{list_permission_rows, list_permissions, PermissionListRow};

/// "N people" for the Who has it column.
#[must_use]
pub fn holders_text(n: Option<usize>) -> String {
    match n {
        None => String::new(),
        Some(1) => "1 person".to_string(),
        Some(n) => format!("{n} people"),
    }
}

/// The text quick search looks through for one row: name, description and key.
#[must_use]
pub fn search_text(row: &PermissionListRow) -> String {
    format!("{} {} {}", row.display_name, row.description, row.key)
}

fn table_rows(rows: &[PermissionListRow]) -> Vec<DataTableRowModel> {
    rows.iter()
        .map(|r| {
            let mut cells = HashMap::new();
            cells.insert("permission".into(), search_text(r));
            cells.insert("domain".into(), r.domain_name.clone());
            cells.insert("owned_by".into(), r.owned_by.clone());
            cells.insert("holders".into(), holders_text(r.holders));
            DataTableRowModel::from_text_cells(r.id.clone(), cells)
        })
        .collect()
}

/// Permissions list page.
#[component]
pub fn PermissionsIndexPage() -> impl IntoView {
    let (search, missing) = use_index_search(crate::paths::PERMISSIONS, |name| async move {
        list_permissions(Some(name))
            .await
            .map(|rows| rows.into_iter().map(|row| (row.id, row.name)).collect())
    });
    let viewer = use_viewer_access();
    let is_admin = Signal::derive(move || viewer_now(viewer).gauge_admin);
    let refresh = RwSignal::new(0u32);
    let rows = Resource::new(move || refresh.get(), |_| list_permission_rows());
    let navigate = use_navigate();
    let navigate_create = navigate.clone();
    let is_narrow = use_breakpoint_down(Breakpoint::Md);
    let skeleton = || {
        view! {
            <div data-testid="gauge-permissions-skeleton">
                <Flex vertical=true gap=FlexGap::Small>
                    {(0..5).map(|_| view! { <SkeletonItem width="100%".to_string() /> }).collect_view()}
                </Flex>
            </div>
        }
    };

    view! {
        <ContentContainer max_width="1100px" data_testid="gauge-permissions-index">
            <Flex vertical=true gap=FlexGap::Medium>
                <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center gap=FlexGap::Medium wrap=FlexWrap::Wrap>
                    <Flex vertical=true gap=FlexGap::Small>
                        <Title3 tag=TextTag::H1>"Permissions"</Title3>
                        <Caption1>"What people and service users can do. Open one to see who has it and why."</Caption1>
                    </Flex>
                    <Suspense fallback=|| ()>
                        <Show when=move || is_admin.get()>
                            <div id="gauge-permissions-create">
                                <Button
                                    appearance=ButtonAppearance::Primary
                                    on_click=Callback::new({
                                        let navigate = navigate_create.clone();
                                        move |_| navigate(crate::paths::CREATE_PERMISSION, NavigateOptions::default())
                                    })
                                >
                                    "Create permission"
                                </Button>
                            </div>
                        </Show>
                    </Suspense>
                </Flex>
                <Show when=move || missing.get().is_some()>
                    <div id="gauge-permissions-name-missing">
                        <MessageBar intent=MessageBarIntent::Warning>
                            <MessageBarBody>{move || format!("No permission has the key {}.", missing.get().unwrap_or_default())}</MessageBarBody>
                        </MessageBar>
                    </div>
                </Show>
                <div id="gauge-permissions-list">
                    <Transition fallback=skeleton>
                        {move || rows.get().map(|result| match result {
                            Ok(list) if list.is_empty() => view! {
                                <div data-testid="gauge-permissions-empty">
                                    <EmptyState
                                        message="No permissions yet"
                                        description="Gauge admins create permissions for each app's actions."
                                    />
                                </div>
                            }.into_any(),
                            Ok(list) => {
                                let by_id = StoredValue::new(
                                    list.iter().map(|r| (r.id.clone(), r.clone())).collect::<HashMap<_, _>>(),
                                );
                                let items = RwSignal::new(table_rows(&list));
                                let navigate = navigate.clone();
                                let on_row_click = Callback::new(move |(id,): (String,)| {
                                    navigate(&format!("/permission/permissions/{id}"), NavigateOptions::default());
                                });
                                let columns = vec![
                                    DataTableColumnDef::new("permission", "Permission").with_cell_view(Arc::new(move |r: DataRecord| {
                                        let Some(row) = by_id.with_value(|m| m.get(&r.id).cloned()) else {
                                            return view! { <span /> }.into_any();
                                        };
                                        view! {
                                            <Flex vertical=true gap=FlexGap::Size(0)>
                                                <Body1Strong>{row.display_name}</Body1Strong>
                                                <Caption1>{row.description}</Caption1>
                                                <Caption1>"Key " <Code text=row.key /></Caption1>
                                            </Flex>
                                        }.into_any()
                                    })),
                                    DataTableColumnDef::new("domain", "Domain"),
                                    DataTableColumnDef::new("owned_by", "Owned by"),
                                ];
                                // Viewer access loads after the rows, so the admin column
                                // joins the table when it arrives.
                                let columns = StoredValue::new(columns);
                                view! {
                                    <div data-testid="gauge-permissions-table">
                                        {move || view! {
                                            <DataTable
                                                data_source=DataTableSource::Client(items)
                                                paging=PagingMode::Paged
                                                auto_row_height=true
                                                quick_search=search
                                                features=if is_narrow.get() { DataTableFeatures::LIST_VIEW } else { DataTableFeatures::empty() }
                                                list_view=ListViewConfig::new("permission").with_secondary_fields(vec!["domain".into(), "owned_by".into()])
                                                toolbar_config=DataTableToolbarConfig {
                                                    quick_search: true,
                                                    filter_panel: false,
                                                    column_picker: false,
                                                    pivot: false,
                                                    export_menu: false,
                                                }
                                                columns={
                                                    let mut cols = columns.get_value();
                                                    if is_admin.get() {
                                                        cols.push(DataTableColumnDef::new("holders", "Who has it"));
                                                    }
                                                    cols
                                                }
                                                events=DataTableEvents {
                                                    on_row_click: Some(on_row_click),
                                                    ..Default::default()
                                                }
                                            >
                                                <DataTableNoResultsView slot>
                                                    <div data-testid="gauge-permissions-no-match">
                                                        <EmptyState
                                                            message="No permission matches your search"
                                                            description="Search looks at names, keys and descriptions."
                                                        >
                                                            <EmptyStateCallToAction slot:call_to_action>
                                                                <Button on_click=Callback::new(move |_| search.set(String::new()))>"Clear search"</Button>
                                                            </EmptyStateCallToAction>
                                                        </EmptyState>
                                                    </div>
                                                </DataTableNoResultsView>
                                            </DataTable>
                                        }}
                                    </div>
                                }.into_any()
                            }
                            Err(err) => view! {
                                <ErrorBar error=err.to_string() what="permissions" on_retry=Callback::new(move |()| refresh.update(|n| *n += 1)) />
                            }.into_any(),
                        })}
                    </Transition>
                </div>
            </Flex>
        </ContentContainer>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> PermissionListRow {
        PermissionListRow {
            id: "p1".into(),
            display_name: "Approve releases".into(),
            key: "ApproveRelease".into(),
            description: "Sign off a release".into(),
            domain_name: "Release tooling".into(),
            owned_by: "Release Captains".into(),
            holders: Some(5),
        }
    }

    #[test]
    fn search_covers_name_key_and_description_happy() {
        let text = search_text(&row());
        assert!(text.contains("Approve releases"));
        assert!(text.contains("ApproveRelease"));
        assert!(text.contains("Sign off"));
    }

    #[test]
    fn holder_count_hidden_without_admin_sad() {
        assert_eq!(holders_text(None), "");
        assert_eq!(holders_text(Some(1)), "1 person");
        assert_eq!(holders_text(Some(5)), "5 people");
    }
}
