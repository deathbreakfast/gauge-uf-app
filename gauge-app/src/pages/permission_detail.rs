//! One permission: what it is, who has it and why, its allow list and its activity.
//!
//! Editors (the permission's owners and Gauge admins) get the Who has access, Allow list and
//! Activity tabs, Give access, and a More menu. Anyone else sees what it is, who owns it and
//! how to get it, with Request access. Switching tabs swaps a signal; no route or page
//! resource changes, so the page never falls back to its skeleton.

use std::collections::HashMap;
use std::sync::Arc;

use gauge::search_sources::PermissionSearchSourceId;
use gauge::types::{
    HolderRow, PermissionDetailDto, PermissionRequestTargetKind, PrincipalKind, PrincipalRefDto,
    RecordKind, RecordRef,
};
use leptos::prelude::*;
use leptos::task::spawn_local_scoped;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};
use leptos_router::NavigateOptions;
use uf_integrations::SearchSourcePicker;
use uf_product::components::{
    Body1, Body1Strong, Caption1, Card, CardContent, ContentContainer, EmptyState,
    EmptyStateCallToAction, MarkdownView, Title3,
};
use uf_product::primitives::{
    Badge, BadgeAppearance, BadgeColor, BadgeSize, Breadcrumb, BreadcrumbButton, BreadcrumbDivider,
    BreadcrumbItem, Button, ButtonAppearance, Code, DataRecord, DataTable, DataTableColumnDef,
    DataTableRowModel, DataTableSource, DataTableToolbarConfig, DataValue, Dialog, DialogActions,
    DialogBody, DialogContent, DialogSurface, DialogTitle, Field, Flex, FlexAlign, FlexGap,
    FlexJustify, FlexWrap, Input, InputAppearance, Link, MarkdownEditor, Menu, MenuItem,
    MenuTrigger, MessageBar, MessageBarBody, MessageBarIntent, MessageBarLayout, MessageBarTitle,
    PagingMode, Select, SelectAppearance, SkeletonItem, Tab, TabList, Table, TableBody, TableCell,
    TableHeader, TableHeaderCell, TableRow,
};
use uf_search_core::{SearchSourceItem, SearchSourceKey};

use crate::components::error_bar::{ErrorBar, MaybeErrorBar};
use crate::pages::shared::give_access::{GiveAccessDialog, GivePermission};
use crate::pages::shared::record_activity::RecordActivity;
use crate::pages::shared::remove_confirm::{
    remove_action, RemoveConfirmDialog, RemoveFrom, RemoveTarget,
};
use crate::pages::shared::request_access::{
    requested_for_handle, wants_request_dialog, RequestAccessDialog,
};
use crate::pages::shared::words::{date_view, kind_word, paths_text, plain_name};
use crate::pages::step_up::spawn_with_step_up_to;
use crate::server::viewer::{use_viewer_access, viewer_now};
use crate::server::{
    delete_permission, get_effective_holders, get_group, get_my_access, get_permission,
    list_domains, remove_permission_group, remove_permission_user, search_principals,
    update_permission, UpdatePermissionInput,
};

/// Tab values.
const TAB_WHO: &str = "who";
const TAB_ALLOW: &str = "allow";
const TAB_ACTIVITY: &str = "activity";

/// The name people read: the display name, or the key for older rows.
fn display_name(row: &PermissionDetailDto) -> String {
    if row.display_name.trim().is_empty() {
        row.name.clone()
    } else {
        row.display_name.clone()
    }
}

/// "5 people and service users", "1 person".
#[must_use]
pub fn holder_count_text(n: usize) -> String {
    match n {
        1 => "1 person or service user".to_string(),
        n => format!("{n} people and service users"),
    }
}

/// What deleting the permission takes away, in one sentence.
#[must_use]
pub fn delete_loses_sentence(holders: usize, name: &str, groups: &[String]) -> String {
    let who = if holders == 1 {
        format!("1 person or service user loses {name}")
    } else {
        format!("{holders} people and service users lose {name}")
    };
    if groups.is_empty() {
        format!("{who}.")
    } else {
        format!("{who}, including everyone in {}.", groups.join(", "))
    }
}

/// Permission page.
#[component]
pub fn PermissionDetailPage() -> impl IntoView {
    let params = use_params_map();
    let permission_id = Memo::new(move |_| params.read().get("id").unwrap_or_default());
    let refresh = RwSignal::new(0u64);
    // Lives above the Transition, so a refetch after a change keeps the open tab.
    let tab = RwSignal::new(TAB_WHO.to_string());
    let viewer = use_viewer_access();
    let detail = Resource::new(
        move || (permission_id.get(), refresh.get()),
        move |(id, _)| get_permission(id),
    );

    view! {
        <ContentContainer max_width="1100px" data_testid="gauge-permission-page">
            <Transition fallback=PermissionSkeleton>
                {move || detail.get().map(|result| match result {
                    Ok(Some(row)) => view! {
                        <PermissionLoaded row=row viewer_ready=Signal::derive(move || viewer.get().is_some()) refresh=refresh tab=tab />
                    }.into_any(),
                    Ok(None) => view! { <PermissionNotFound /> }.into_any(),
                    Err(err) => view! {
                        <Flex vertical=true gap=FlexGap::Medium>
                            <PermissionCrumbs name="Permission".to_string() />
                            <ErrorBar
                                error=err.to_string()
                                what="this permission"
                                on_retry=Callback::new(move |()| refresh.update(|n| *n += 1))
                            />
                        </Flex>
                    }.into_any(),
                })}
            </Transition>
        </ContentContainer>
    }
}

#[component]
fn PermissionSkeleton() -> impl IntoView {
    view! {
        <div data-testid="gauge-permission-skeleton">
            <Flex vertical=true gap=FlexGap::Medium>
                <SkeletonItem width="120px".to_string() />
                <SkeletonItem width="280px".to_string() />
                <Card>
                    <CardContent>
                        <Flex vertical=true gap=FlexGap::Small>
                            {(0..4).map(|_| view! { <SkeletonItem width="100%".to_string() /> }).collect_view()}
                        </Flex>
                    </CardContent>
                </Card>
            </Flex>
        </div>
    }
}

#[component]
fn PermissionCrumbs(name: String) -> impl IntoView {
    view! {
        <Breadcrumb>
            <BreadcrumbItem>
                <BreadcrumbButton href=crate::paths::PERMISSIONS.to_string()>"Permissions"</BreadcrumbButton>
            </BreadcrumbItem>
            <BreadcrumbDivider />
            <BreadcrumbItem>
                <BreadcrumbButton current=true>{name}</BreadcrumbButton>
            </BreadcrumbItem>
        </Breadcrumb>
    }
}

#[component]
fn PermissionNotFound() -> impl IntoView {
    let navigate = use_navigate();
    view! {
        <div data-testid="gauge-permission-not-found">
            <Flex vertical=true gap=FlexGap::Medium>
                <PermissionCrumbs name="Not found".to_string() />
                <EmptyState
                    message="No permission with that link"
                    description="It may have been deleted, or the link has a typo."
                    icon=icondata_core::Icon::from(icondata::AiSearchOutlined)
                >
                    <EmptyStateCallToAction slot:call_to_action>
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click=Callback::new(move |_| navigate(crate::paths::PERMISSIONS, NavigateOptions::default()))
                        >
                            "Back to Permissions"
                        </Button>
                    </EmptyStateCallToAction>
                </EmptyState>
            </Flex>
        </div>
    }
}

/// The page once the permission loaded.
#[component]
fn PermissionLoaded(
    row: PermissionDetailDto,
    viewer_ready: Signal<bool>,
    refresh: RwSignal<u64>,
    tab: RwSignal<String>,
) -> impl IntoView {
    let viewer = use_viewer_access();
    let row = StoredValue::new(row);
    let id = row.with_value(|r| r.id.clone());
    let can_edit = {
        let id = id.clone();
        Signal::derive(move || viewer_now(viewer).can_manage_permission(&id))
    };
    let is_admin = Signal::derive(move || viewer_now(viewer).gauge_admin);
    let name = row.with_value(display_name);
    let owners_group_id = row.with_value(|r| r.owners_group_id.clone());
    let owners = Resource::new(
        {
            let g = owners_group_id.clone();
            move || g.clone()
        },
        get_group,
    );
    let owners_name = Signal::derive(move || {
        owners
            .get()
            .and_then(Result::ok)
            .flatten()
            .map(|g| g.name)
            .unwrap_or_default()
    });
    let owner_people = Signal::derive(move || {
        owners
            .get()
            .and_then(Result::ok)
            .flatten()
            .map(|g| {
                g.owner_users
                    .iter()
                    .map(|o| plain_name(&o.label))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    });

    let request_open = RwSignal::new(false);
    let query = use_query_map();
    let preselect_for = Signal::derive(move || requested_for_handle(&query.read()));
    let auto_opened = StoredValue::new(false);
    let can_request = row.with_value(|r| r.can_request_access);
    Effect::new(move |_| {
        if can_request && !auto_opened.get_value() && wants_request_dialog(&query.read_untracked())
        {
            auto_opened.set_value(true);
            request_open.set(true);
        }
    });

    let notice = RwSignal::new(None::<String>);
    let give_open = RwSignal::new(false);
    let edit_open = RwSignal::new(false);
    let delete_open = RwSignal::new(false);

    let record = RecordRef {
        kind: RecordKind::Permission,
        id: id.clone(),
        label: name.clone(),
    };
    let key = row.with_value(|r| r.name.clone());
    let domain_name = row.with_value(|r| r.domain_name.clone());
    let domain_badge = domain_name.clone();
    let domain_href = row.with_value(|r| format!("/permission/domains/{}", r.domain_id));
    let group_href = format!("/permission/groups/{owners_group_id}");
    let description = row.with_value(|r| r.description.clone());

    view! {
        <Flex vertical=true gap=FlexGap::Medium>
            <PermissionCrumbs name=name.clone() />
            <Show when=move || notice.get().is_some()>
                <div data-testid="gauge-permission-notice">
                    <MessageBar intent=MessageBarIntent::Success>
                        <MessageBarBody>{move || notice.get().unwrap_or_default()}</MessageBarBody>
                    </MessageBar>
                </div>
            </Show>
            <div id="gauge-perm-detail-header">
                <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Start gap=FlexGap::Medium wrap=FlexWrap::Wrap>
                    <Flex vertical=true gap=FlexGap::Small>
                        <Flex align=FlexAlign::Center gap=FlexGap::Small wrap=FlexWrap::Wrap>
                            <Title3 tag=uf_product::components::TextTag::H1>{name.clone()}</Title3>
                            <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand size=BadgeSize::Small>
                                {domain_badge}
                            </Badge>
                        </Flex>
                        <MarkdownView markdown=description testid="gauge-perm-description" />
                        <Caption1>"Key " <Code text=key.clone() /></Caption1>
                    </Flex>
                    <div id="gauge-perm-actions">
                        <Flex gap=FlexGap::Small>
                            <div id="gauge-perm-request-access">
                                <Show when=move || viewer_ready.get() && !can_edit.get() && can_request>
                                    <Button appearance=ButtonAppearance::Primary on_click=Callback::new(move |_| request_open.set(true))>
                                        "Request access"
                                    </Button>
                                </Show>
                            </div>
                            <div id="gauge-perm-give">
                                <Show when=move || viewer_ready.get() && can_edit.get()>
                                    <Button appearance=ButtonAppearance::Primary on_click=Callback::new(move |_| give_open.set(true))>
                                        "Give access"
                                    </Button>
                                </Show>
                            </div>
                            <Show when=move || is_admin.get()>
                                <div id="gauge-perm-more">
                                    <Menu on_select=move |action: &str| match action {
                                        "edit" => edit_open.set(true),
                                        "delete" => delete_open.set(true),
                                        _ => {}
                                    }>
                                        <MenuTrigger slot>
                                            <Button appearance=ButtonAppearance::Secondary icon=icondata::AiMoreOutlined attr:aria-label="More actions" />
                                        </MenuTrigger>
                                        <MenuItem value="edit">"Edit"</MenuItem>
                                        <MenuItem value="delete">"Delete"</MenuItem>
                                    </Menu>
                                </div>
                            </Show>
                        </Flex>
                    </div>
                </Flex>
            </div>
            <div data-testid="gauge-perm-facts">
                <Flex gap=FlexGap::Large wrap=FlexWrap::Wrap>
                    <Fact label="Owned by">
                        <Link href=group_href.clone()>{move || owners_name.get()}</Link>
                    </Fact>
                    <Fact label="Domain">
                        <Link href=domain_href.clone()>{domain_name.clone()}</Link>
                    </Fact>
                    <Show when=move || can_edit.get()>
                        <Fact label="Who has it">
                            <HolderCount permission_id=row.with_value(|r| r.id.clone()) />
                        </Fact>
                    </Show>
                </Flex>
            </div>
            <div id="gauge-perm-access">
            <Show
                when=move || can_edit.get()
                fallback=move || view! {
                    <NotEditorView
                        permission_id=row.with_value(|r| r.id.clone())
                        name=row.with_value(display_name)
                        owners_name=owners_name
                        owner_people=owner_people
                    />
                }
            >
                <EditorTabs row=row.get_value() record=record.clone() refresh=refresh notice=notice tab=tab />
            </Show>
            </div>
            <GiveAccessDialog
                open=give_open
                service_users=true
                permission=Signal::derive(move || give_permission(&row.get_value(), &owners_name.get(), viewer_now(viewer).can_manage_group(&row.with_value(|r| r.owners_group_id.clone()))))
                on_done=Callback::new(move |text: String| {
                    notice.set(Some(text));
                    refresh.update(|n| *n += 1);
                })
            />
            <Show when=move || is_admin.get()>
                <EditPermissionDialog open=edit_open row=row.get_value() refresh=refresh />
                <DeletePermissionDialog open=delete_open row=row.get_value() />
            </Show>
            <RequestAccessDialog
                open=request_open
                target_kind=PermissionRequestTargetKind::Permission
                target_id=Signal::derive(move || row.with_value(|r| r.id.clone()))
                preselect_for=preselect_for
                on_submitted=Callback::new(move |()| refresh.update(|n| *n += 1))
            />
        </Flex>
    }
}

/// The Give access dialog's view of this permission: groups on the allow list that the
/// viewer may add members to come first as suggestions.
fn give_permission(
    row: &PermissionDetailDto,
    _owners_name: &str,
    _owns_owner_group: bool,
) -> GivePermission {
    let viewer_groups =
        use_context::<Resource<Result<crate::server::viewer::ViewerAccess, ServerFnError>>>()
            .map(viewer_now)
            .unwrap_or_default();
    GivePermission {
        id: row.id.clone(),
        name: display_name(row),
        key: row.name.clone(),
        suggest_groups: row
            .allow_list
            .iter()
            .filter(|p| p.kind == PrincipalKind::Group && viewer_groups.can_manage_group(&p.id))
            .map(|p| (p.id.clone(), p.label.clone()))
            .collect(),
        allow_list: row.allow_list.clone(),
    }
}

#[component]
fn Fact(#[prop(into)] label: String, children: Children) -> impl IntoView {
    view! {
        <Flex vertical=true gap=FlexGap::Size(0)>
            <Caption1>{label}</Caption1>
            <Body1>{children()}</Body1>
        </Flex>
    }
}

#[component]
fn HolderCount(permission_id: String) -> impl IntoView {
    let holders = Resource::new(move || permission_id.clone(), get_effective_holders);
    view! {
        <Suspense fallback=|| view! { <SkeletonItem width="80px".to_string() /> }>
            {move || holders.get().map(|r| match r {
                Ok(rows) => holder_count_text(rows.len()),
                Err(_) => "Hidden".to_string(),
            })}
        </Suspense>
    }
}

/// What someone who can't edit the permission sees in place of the tabs.
#[component]
fn NotEditorView(
    permission_id: String,
    name: String,
    owners_name: Signal<String>,
    owner_people: Signal<Vec<String>>,
) -> impl IntoView {
    let mine = Resource::new(|| (), |()| get_my_access());
    let holds = {
        let id = permission_id.clone();
        Signal::derive(move || {
            mine.get()
                .and_then(Result::ok)
                .and_then(|a| a.permissions.into_iter().find(|(p, _)| p.id == id))
                .map(|(_, paths)| paths_text(&paths))
        })
    };
    let name = StoredValue::new(name);
    view! {
        <Suspense fallback=|| view! { <SkeletonItem width="100%".to_string() /> }>
            <div data-testid="gauge-perm-why-not">
                {move || match holds.get() {
                    Some(how) => view! {
                        <MessageBar intent=MessageBarIntent::Success layout=MessageBarLayout::Multiline>
                            <MessageBarBody>
                                <MessageBarTitle>"You have this permission"</MessageBarTitle>
                                {format!("You hold it from: {how}.")}
                            </MessageBarBody>
                        </MessageBar>
                    }.into_any(),
                    None => view! {
                        <MessageBar intent=MessageBarIntent::Info layout=MessageBarLayout::Multiline>
                            <MessageBarBody>
                                <MessageBarTitle>"You don't have this permission"</MessageBarTitle>
                                {format!(
                                    "{owners} owns it. Ask for access and its owners decide, or ask to join a group that holds {name}.",
                                    owners = owners_name.get(),
                                    name = name.get_value(),
                                )}
                            </MessageBarBody>
                        </MessageBar>
                    }.into_any(),
                }}
            </div>
        </Suspense>
        <Card>
            <CardContent>
                <div data-testid="gauge-perm-owner-line">
                    <Flex vertical=true gap=FlexGap::Small>
                        <Body1Strong>"Who has access"</Body1Strong>
                        <Body1>{move || {
                            let people = owner_people.get();
                            let ask = if people.is_empty() {
                                format!("Ask the owners of {}", owners_name.get())
                            } else {
                                format!("Ask {}, who owns {},", people.join(" or "), owners_name.get())
                            };
                            format!("Only the owners of this permission and Gauge admins can see who holds it. {ask} if you need to know.")
                        }}</Body1>
                    </Flex>
                </div>
            </CardContent>
        </Card>
    }
}

/// Who has access, Allow list and Activity, for editors.
#[component]
fn EditorTabs(
    row: PermissionDetailDto,
    record: RecordRef,
    refresh: RwSignal<u64>,
    notice: RwSignal<Option<String>>,
    tab: RwSignal<String>,
) -> impl IntoView {
    let row = StoredValue::new(row);
    let record = StoredValue::new(record);
    let allow_count = row.with_value(|r| r.allow_list.len());
    let holders = Resource::new(
        move || (row.with_value(|r| r.id.clone()), refresh.get()),
        |(id, _)| get_effective_holders(id),
    );
    let holder_total = Signal::derive(move || holders.get().and_then(Result::ok).map(|h| h.len()));
    view! {
        <Card>
            <CardContent>
                <div id="gauge-perm-tabs">
                    <Flex vertical=true gap=FlexGap::Medium>
                        <TabList selected_value=tab>
                            <Tab value=TAB_WHO>{move || match holder_total.get() {
                                Some(n) => format!("Who has access ({n})"),
                                None => "Who has access".to_string(),
                            }}</Tab>
                            <Tab value=TAB_ALLOW>{format!("Allow list ({allow_count})")}</Tab>
                            <Tab value=TAB_ACTIVITY>"Activity"</Tab>
                        </TabList>
                        {move || match tab.get().as_str() {
                            TAB_ALLOW => view! { <AllowListTab row=row.get_value() holders=holders refresh=refresh notice=notice /> }.into_any(),
                            TAB_ACTIVITY => view! { <RecordActivity record=record.get_value() /> }.into_any(),
                            _ => view! { <WhoHasAccessTab key=row.with_value(|r| r.name.clone()) holders=holders refresh=refresh /> }.into_any(),
                        }}
                    </Flex>
                </div>
            </CardContent>
        </Card>
    }
}

fn holder_rows(rows: &[HolderRow]) -> Vec<DataTableRowModel> {
    rows.iter()
        .map(|h| {
            let mut cells = HashMap::new();
            cells.insert("who".into(), h.label.clone());
            cells.insert("kind".into(), kind_word(&h.kind, h.owner_label.as_deref()));
            cells.insert("why".into(), paths_text(&h.paths));
            cells.insert(
                "since".into(),
                h.since.map(|s| s.to_rfc3339()).unwrap_or_default(),
            );
            DataTableRowModel::from_text_cells(format!("{:?}:{}", h.kind, h.principal.id), cells)
        })
        .collect()
}

fn text_of(record: &DataRecord, field: &'static str) -> String {
    match record.get(field) {
        Some(DataValue::Text(t)) => t.clone(),
        _ => String::new(),
    }
}

#[component]
fn WhoHasAccessTab(
    key: String,
    holders: Resource<Result<Vec<HolderRow>, ServerFnError>>,
    refresh: RwSignal<u64>,
) -> impl IntoView {
    let caption =
        format!("Everyone who passes the {key} check today, and the path that gives it to them.");
    view! {
        <div data-testid="gauge-perm-who">
            <Flex vertical=true gap=FlexGap::Small>
                <Caption1>{caption}</Caption1>
                <Suspense fallback=|| view! { <SkeletonItem width="100%".to_string() /> }>
                    {move || holders.get().map(|r| match r {
                        Ok(rows) if rows.is_empty() => view! {
                            <EmptyState message="Nobody has this permission yet." description="Give access to a person or a group to start." />
                        }.into_any(),
                        Ok(rows) => {
                            let items = RwSignal::new(holder_rows(&rows));
                            view! {
                                <DataTable
                                    data_source=DataTableSource::Client(items)
                                    paging=PagingMode::None
                                    auto_row_height=true
                                    toolbar_config=DataTableToolbarConfig {
                                        quick_search: true,
                                        filter_panel: false,
                                        column_picker: false,
                                        pivot: false,
                                        export_menu: false,
                                    }
                                    columns=vec![
                                        DataTableColumnDef::new("who", "Who").with_cell_view(Arc::new(|r: DataRecord| {
                                            let who = text_of(&r, "who");
                                            let kind = text_of(&r, "kind");
                                            view! {
                                                <Flex vertical=true gap=FlexGap::Size(0)>
                                                    <Body1Strong>{who}</Body1Strong>
                                                    <Caption1>{kind}</Caption1>
                                                </Flex>
                                            }.into_any()
                                        })),
                                        DataTableColumnDef::new("why", "Why they have it"),
                                        DataTableColumnDef::new("since", "Since").with_cell_view(Arc::new(|r: DataRecord| {
                                            let since = text_of(&r, "since");
                                            if since.is_empty() {
                                                view! { <Caption1>"—"</Caption1> }.into_any()
                                            } else {
                                                date_view(since).into_any()
                                            }
                                        })),
                                    ]
                                />
                            }.into_any()
                        }
                        Err(err) => view! {
                            <ErrorBar error=err.to_string() what="who has this permission" on_retry=Callback::new(move |()| refresh.update(|n| *n += 1)) />
                        }.into_any(),
                    })}
                </Suspense>
            </Flex>
        </div>
    }
}

#[component]
fn AllowListTab(
    row: PermissionDetailDto,
    holders: Resource<Result<Vec<HolderRow>, ServerFnError>>,
    refresh: RwSignal<u64>,
    notice: RwSignal<Option<String>>,
) -> impl IntoView {
    let row = StoredValue::new(row);
    let remove_open = RwSignal::new(false);
    let remove_target = RwSignal::new(None::<RemoveTarget>);
    let add_open = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let since = Signal::derive(move || {
        holders
            .get()
            .and_then(Result::ok)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|h| h.since.map(|s| (h.principal.id, s.to_rfc3339())))
            .collect::<HashMap<_, _>>()
    });
    let on_remove = Callback::new(move |()| {
        let Some(target) = remove_target.get() else {
            return;
        };
        let permission_id = row.with_value(|r| r.id.clone());
        let principal = target.principal.clone();
        let confirm = remove_action(&target);
        let label = plain_name(&principal.label);
        let name = target.from.name().to_string();
        remove_open.set(false);
        let done = Callback::new(move |()| {
            notice.set(Some(format!("{label} is off the allow list of {name}.")));
            refresh.update(|n| *n += 1);
        });
        spawn_with_step_up_to(confirm, error, done, move || {
            let permission_id = permission_id.clone();
            let principal = principal.clone();
            async move {
                if principal.kind == PrincipalKind::Group {
                    remove_permission_group(permission_id, principal.id).await
                } else {
                    remove_permission_user(permission_id, principal.id).await
                }
            }
        });
    });
    let open_remove = move |principal: PrincipalRefDto| {
        remove_target.set(Some(RemoveTarget {
            principal,
            from: RemoveFrom::AllowList {
                id: row.with_value(|r| r.id.clone()),
                name: row.with_value(display_name),
            },
        }));
        remove_open.set(true);
    };
    view! {
        <div data-testid="gauge-perm-allow">
            <Flex vertical=true gap=FlexGap::Small>
                <Caption1>"The people, service users and groups given this permission directly. Removing one takes away only what comes from this list."</Caption1>
                {move || {
                    let list = row.with_value(|r| r.allow_list.clone());
                    if list.is_empty() {
                        view! { <EmptyState message="Nobody is on the allow list." /> }.into_any()
                    } else {
                        view! {
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        <TableHeaderCell>"On the allow list"</TableHeaderCell>
                                        <TableHeaderCell>"Since"</TableHeaderCell>
                                        <TableHeaderCell>""</TableHeaderCell>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    {list.into_iter().map(|p| {
                                        let removable = p.kind != PrincipalKind::ServiceUser;
                                        let when = since.get().get(&p.id).cloned();
                                        let kind = kind_word(&p.kind, p.owner_label.as_deref());
                                        let label = p.label.clone();
                                        let menu_label = format!("Actions for {label}");
                                        view! {
                                            <TableRow attr:data-testid="gauge-perm-allow-row">
                                                <TableCell>
                                                    <Flex vertical=true gap=FlexGap::Size(0)>
                                                        <Body1Strong>{label}</Body1Strong>
                                                        <Caption1>{kind}</Caption1>
                                                    </Flex>
                                                </TableCell>
                                                <TableCell>{match when {
                                                    Some(w) => date_view(w).into_any(),
                                                    None => view! { <Caption1>"—"</Caption1> }.into_any(),
                                                }}</TableCell>
                                                <TableCell>
                                                    {removable.then(|| view! {
                                                        <Menu on_select=move |action: &str| if action == "remove" { open_remove(p.clone()) }>
                                                            <MenuTrigger slot>
                                                                <Button appearance=ButtonAppearance::Subtle icon=icondata::AiMoreOutlined attr:aria-label=menu_label.clone() />
                                                            </MenuTrigger>
                                                            <MenuItem value="remove">"Remove"</MenuItem>
                                                        </Menu>
                                                    })}
                                                </TableCell>
                                            </TableRow>
                                        }
                                    }).collect_view()}
                                </TableBody>
                            </Table>
                        }.into_any()
                    }
                }}
                <div id="gauge-perm-allow-add">
                    <Button appearance=ButtonAppearance::Secondary on_click=Callback::new(move |_| add_open.set(true))>
                        "Add to allow list"
                    </Button>
                </div>
                <MaybeErrorBar error=error what="the allow list" />
            </Flex>
            <RemoveConfirmDialog open=remove_open target=remove_target on_confirm=on_remove />
            <GiveAccessDialog
                open=add_open
                service_users=false
                permission=Signal::derive(move || {
                    let r = row.get_value();
                    GivePermission {
                        id: r.id.clone(),
                        name: display_name(&r),
                        key: r.name.clone(),
                        suggest_groups: Vec::new(),
                        allow_list: r.allow_list.clone(),
                    }
                })
                on_done=Callback::new(move |text: String| {
                    notice.set(Some(text));
                    refresh.update(|n| *n += 1);
                })
            />
        </div>
    }
}

/// Edit name, description, owners and domain (Gauge admins).
#[component]
fn EditPermissionDialog(
    open: RwSignal<bool>,
    row: PermissionDetailDto,
    refresh: RwSignal<u64>,
) -> impl IntoView {
    let id = row.id.clone();
    let key = row.name.clone();
    let name = RwSignal::new(display_name(&row));
    let description = RwSignal::new(row.description.clone());
    let owners_group_id = RwSignal::new(row.owners_group_id.clone());
    let domain_id = RwSignal::new(row.domain_id.clone());
    let domains = Resource::new(|| (), |()| list_domains(None));
    let owners_options = RwSignal::new(Vec::<SearchSourceItem>::new());
    let owners_seq = RwSignal::new(0u64);
    let error = RwSignal::new(None::<String>);
    let search = move |query: Option<String>| {
        let seq = owners_seq.get_untracked().saturating_add(1);
        owners_seq.set(seq);
        spawn_local_scoped(async move {
            let sources: Vec<SearchSourceKey> =
                vec![PermissionSearchSourceId::PermissionGroup.into()];
            match search_principals(sources, query, 10).await {
                Ok(rows) if owners_seq.get_untracked() == seq => owners_options.set(rows),
                Ok(_) => {}
                Err(err) => error.set(Some(err.to_string())),
            }
        });
    };
    let save = move |_| {
        let payload = UpdatePermissionInput {
            id: id.clone(),
            name: key.clone(),
            display_name: name.get(),
            description: description.get(),
            owners_group_id: owners_group_id.get(),
            domain_id: domain_id.get(),
        };
        let confirm = format!("Save changes to {}", payload.display_name);
        let done = Callback::new(move |()| {
            open.set(false);
            refresh.update(|n| *n += 1);
        });
        spawn_with_step_up_to(confirm, error, done, move || {
            update_permission(payload.clone())
        });
    };
    view! {
        <Dialog open=open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>"Edit permission"</DialogTitle>
                    <DialogContent>
                        <div id="gauge-perm-edit-form">
                            <Flex vertical=true gap=FlexGap::Medium>
                                <Field label="Name"><Input bind=name /></Field>
                                <Field label="Description">
                                    <MarkdownEditor value=description aria_label="Description" />
                                </Field>
                                <Field label="Owned by">
                                    <div id="gauge-perm-owners-group-picker">
                                        <SearchSourcePicker
                                            search_sources=Signal::derive(|| vec![PermissionSearchSourceId::PermissionGroup.into()])
                                            options=owners_options
                                            placeholder="Search permission groups"
                                            on_request_initial=Callback::new(move |_| search(None))
                                            on_search=Callback::new(move |(_, q): (Vec<SearchSourceKey>, String)| search(Some(q)))
                                            on_select=Callback::new(move |item: SearchSourceItem| owners_group_id.set(item.id))
                                        />
                                    </div>
                                </Field>
                                <Field label="Domain">
                                    <Select bind=domain_id appearance=SelectAppearance {
                                        disabled: Signal::derive(move || !matches!(domains.get(), Some(Ok(_)))),
                                        ..Default::default()
                                    }>
                                        {move || domains.get().and_then(Result::ok).unwrap_or_default().into_iter().map(|d| view! {
                                            <option value=d.id>{d.name}</option>
                                        }).collect_view()}
                                    </Select>
                                </Field>
                                <MaybeErrorBar error=error what="this permission" />
                            </Flex>
                        </div>
                    </DialogContent>
                    <DialogActions>
                        <Button appearance=ButtonAppearance::Subtle on_click=Callback::new(move |_| open.set(false))>"Cancel"</Button>
                        <div id="gauge-perm-save">
                            <Button appearance=ButtonAppearance::Primary on_click=Callback::new(save)>"Save changes"</Button>
                        </div>
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
        </Dialog>
    }
}

/// Delete asks first, says who loses it and what code will do, and wants the key typed.
#[component]
fn DeletePermissionDialog(open: RwSignal<bool>, row: PermissionDetailDto) -> impl IntoView {
    let navigate = use_navigate();
    let typed = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let row = StoredValue::new(row);
    let holders = Resource::new(
        move || open.get().then(|| row.with_value(|r| r.id.clone())),
        |id| async move {
            match id {
                Some(id) => get_effective_holders(id).await.map(Some),
                None => Ok(None),
            }
        },
    );
    let loses = move || {
        let r = row.get_value();
        let groups: Vec<String> = r
            .allow_list
            .iter()
            .filter(|p| p.kind == PrincipalKind::Group)
            .map(|p| p.label.clone())
            .collect();
        match holders.get() {
            Some(Ok(Some(h))) => delete_loses_sentence(h.len(), &display_name(&r), &groups),
            _ => format!("Everyone who holds {} loses it.", display_name(&r)),
        }
    };
    let matches = Signal::derive(move || typed.get().trim() == row.with_value(|r| r.name.clone()));
    let delete = move |_| {
        if !matches.get() {
            return;
        }
        let id = row.with_value(|r| r.id.clone());
        let confirm = format!("Delete {}", row.with_value(display_name));
        let navigate = navigate.clone();
        let done = Callback::new(move |()| {
            navigate(crate::paths::PERMISSIONS, NavigateOptions::default());
        });
        spawn_with_step_up_to(confirm, error, done, move || delete_permission(id.clone()));
    };
    view! {
        <Dialog open=open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>{move || format!("Delete {}?", row.with_value(display_name))}</DialogTitle>
                    <DialogContent>
                        <div data-testid="gauge-perm-delete-confirm">
                            <Flex vertical=true gap=FlexGap::Small>
                                <Suspense fallback=|| ()>
                                    <Body1>{loses}</Body1>
                                </Suspense>
                                <Body1>{move || format!(
                                    "Code that checks {} will refuse everyone except Super User until a permission with that key exists again.",
                                    row.with_value(|r| r.name.clone())
                                )}</Body1>
                                <Field label="Type the key to confirm">
                                    <Input bind=typed appearance=InputAppearance::with_placeholder(row.with_value(|r| r.name.clone())) />
                                </Field>
                                <MaybeErrorBar error=error what="this permission" />
                            </Flex>
                        </div>
                    </DialogContent>
                    <DialogActions>
                        <Button appearance=ButtonAppearance::Subtle on_click=Callback::new(move |_| open.set(false))>"Cancel"</Button>
                        <div id="gauge-perm-delete">
                            <Button
                                appearance=ButtonAppearance::Danger
                                disabled=Signal::derive(move || !matches.get())
                                on_click=Callback::new(delete)
                            >
                                "Delete permission"
                            </Button>
                        </div>
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
        </Dialog>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delete_sentence_names_groups_happy() {
        assert_eq!(
            delete_loses_sentence(5, "Approve releases", &["Release Captains".into()]),
            "5 people and service users lose Approve releases, including everyone in Release Captains."
        );
    }

    #[test]
    fn delete_sentence_without_groups_sad() {
        assert_eq!(
            delete_loses_sentence(1, "Deploy", &[]),
            "1 person or service user loses Deploy."
        );
        assert_eq!(
            delete_loses_sentence(0, "Deploy", &[]),
            "0 people and service users lose Deploy."
        );
    }
}
