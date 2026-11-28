//! The Domains list: each domain with its description, how many permissions it holds and its
//! owners. Gauge admins get Create domain.

use std::collections::HashMap;
use std::sync::Arc;

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;
use uf_product::components::{
    Body1, Body1Strong, Caption1, ContentContainer, EmptyState, EmptyStateCallToAction, TextTag,
    Title2,
};
use uf_product::primitives::{
    Button, ButtonAppearance, DataRecord, DataTable, DataTableColumnDef, DataTableEvents,
    DataTableRowModel, DataTableSource, Flex, FlexAlign, FlexGap, FlexJustify, FlexWrap,
    PagingMode, SkeletonItem,
};

use crate::components::error_bar::ErrorBar;
use crate::server::surfaces::{list_domain_rows, DomainRow};
use crate::server::viewer::{use_viewer_access, viewer_now};

/// The Owners column: names, or who manages the domain when none are set.
#[must_use]
pub fn owners_text(owners: &[String]) -> String {
    if owners.is_empty() {
        "Gauge admins".to_string()
    } else {
        owners.join(", ")
    }
}

/// The Domains page.
#[component]
pub fn DomainsIndexPage() -> impl IntoView {
    let viewer = use_viewer_access();
    let is_admin = Signal::derive(move || viewer_now(viewer).gauge_admin);
    let refresh = RwSignal::new(0u32);
    let rows = Resource::new(move || refresh.get(), |_| list_domain_rows());
    let navigate = use_navigate();
    let create = Callback::new({
        let navigate = navigate.clone();
        move |_| navigate("/permission/create-domain", NavigateOptions::default())
    });

    view! {
        <ContentContainer max_width="1100px" data_testid="gauge-domains-index">
            <Flex vertical=true gap=FlexGap::Medium>
                <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center gap=FlexGap::Medium wrap=FlexWrap::Wrap>
                    <Flex vertical=true gap=FlexGap::Small>
                        <Title2 tag=TextTag::H1>"Domains"</Title2>
                        <Body1>"Domains group permissions by area, so each app's permissions sit together."</Body1>
                    </Flex>
                    <Suspense fallback=|| ()>
                        <Show when=move || is_admin.get()>
                            <div id="gauge-domains-create">
                                <Button appearance=ButtonAppearance::Primary on_click=create>"Create domain"</Button>
                            </div>
                        </Show>
                    </Suspense>
                </Flex>
                <div id="gauge-domains-list">
                    <Transition fallback=|| view! {
                        <div data-testid="gauge-domains-skeleton">
                            <Flex vertical=true gap=FlexGap::Small>
                                {(0..4).map(|_| view! { <SkeletonItem width="100%".to_string() /> }).collect_view()}
                            </Flex>
                        </div>
                    }>
                        {move || rows.get().map(|result| match result {
                            Ok(list) if list.is_empty() => view! {
                                <div data-testid="gauge-domains-empty">
                                    <EmptyState message="No domains yet" description="Create one before your first permission.">
                                        <EmptyStateCallToAction slot:call_to_action>
                                            <Show when=move || is_admin.get()>
                                                <Button appearance=ButtonAppearance::Primary on_click=create>"Create domain"</Button>
                                            </Show>
                                        </EmptyStateCallToAction>
                                    </EmptyState>
                                </div>
                            }.into_any(),
                            Ok(list) => {
                                let by_id = StoredValue::new(
                                    list.iter().map(|r| (r.id.clone(), r.clone())).collect::<HashMap<String, DomainRow>>(),
                                );
                                let items = RwSignal::new(list.iter().map(|r| {
                                    let mut cells = HashMap::new();
                                    cells.insert("domain".into(), format!("{} {}", r.name, r.description));
                                    cells.insert("permissions".into(), r.permissions.to_string());
                                    cells.insert("owners".into(), owners_text(&r.owners));
                                    DataTableRowModel::from_text_cells(r.id.clone(), cells)
                                }).collect::<Vec<_>>());
                                let navigate = navigate.clone();
                                let on_row_click = Callback::new(move |(id,): (String,)| {
                                    navigate(&format!("/permission/domains/{id}"), NavigateOptions::default());
                                });
                                let columns = vec![
                                    DataTableColumnDef::new("domain", "Domain").with_cell_view(Arc::new(move |r: DataRecord| {
                                        let Some(row) = by_id.with_value(|m| m.get(&r.id).cloned()) else {
                                            return view! { <span /> }.into_any();
                                        };
                                        view! {
                                            <Flex vertical=true gap=FlexGap::Size(0)>
                                                <Body1Strong>{row.name}</Body1Strong>
                                                <Caption1>{row.description}</Caption1>
                                            </Flex>
                                        }.into_any()
                                    })),
                                    DataTableColumnDef::new("permissions", "Permissions"),
                                    DataTableColumnDef::new("owners", "Owners"),
                                ];
                                view! {
                                    <div data-testid="gauge-domains-table">
                                        <DataTable
                                            data_source=DataTableSource::Client(items)
                                            paging=PagingMode::Paged
                                            auto_row_height=true
                                            columns=columns
                                            events=DataTableEvents {
                                                on_row_click: Some(on_row_click),
                                                ..Default::default()
                                            }
                                        />
                                    </div>
                                }.into_any()
                            }
                            Err(err) => view! {
                                <ErrorBar error=err.to_string() what="domains" on_retry=Callback::new(move |()| refresh.update(|n| *n += 1)) />
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

    #[test]
    fn owners_list_names_happy() {
        assert_eq!(owners_text(&["Sol".into(), "Ines".into()]), "Sol, Ines");
    }

    #[test]
    fn no_owners_falls_back_to_admins_sad() {
        assert_eq!(owners_text(&[]), "Gauge admins");
    }
}
