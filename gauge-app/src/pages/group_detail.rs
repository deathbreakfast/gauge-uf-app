//! One group: its members, what they get, its owners and its activity.
//!
//! Owners and Gauge admins add and remove members; each change asks for their authenticator
//! code (a fresh one on Super User). After a removal a success bar offers Undo for a few
//! seconds. Groups another product manages (Polaron teams) list their members read-only.
//! Tabs swap a signal, so selecting one never reloads the page.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use gauge::types::{
    ActivityRow, PermissionGroupDetailDto, PermissionRequestTargetKind, PrincipalKind,
    PrincipalRefDto, RecordKind, RecordRef,
};
use lepton_shell::people::{PersonPicker, PickedPrincipal};
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};
use leptos_router::NavigateOptions;
use uf_product::components::{
    Body1, Body1Strong, Caption1, Card, CardContent, ContentContainer, EmptyState,
    EmptyStateCallToAction, MarkdownView, TextTag, Title3,
};
use uf_product::primitives::{
    Badge, BadgeAppearance, BadgeColor, BadgeSize, Breadcrumb, BreadcrumbButton, BreadcrumbDivider,
    BreadcrumbItem, Button, ButtonAppearance, Code, DataRecord, DataTable, DataTableColumnDef,
    DataTableRowModel, DataTableSource, DataTableToolbarConfig, DataValue, Dialog, DialogActions,
    DialogBody, DialogContent, DialogSurface, DialogTitle, Field, Flex, FlexAlign, FlexGap,
    FlexJustify, FlexWrap, Input, Link, MarkdownEditor, Menu, MenuItem, MenuTrigger, MessageBar,
    MessageBarActions, MessageBarBody, MessageBarIntent, MessageBarLayout, MessageBarTitle,
    PagingMode, SkeletonItem, Tab, TabList, Table, TableBody, TableCell, TableHeader,
    TableHeaderCell, TableRow,
};

use crate::components::error_bar::{ErrorBar, MaybeErrorBar};
use crate::pages::principal_pick::{act_on_pick, to_picked, use_group_search, PickTarget};
use crate::pages::shared::record_activity::RecordActivity;
use crate::pages::shared::remove_confirm::{
    remove_action, RemoveConfirmDialog, RemoveFrom, RemoveTarget,
};
use crate::pages::shared::request_access::{
    requested_for_handle, wants_request_dialog, RequestAccessDialog,
};
use crate::pages::shared::resource_cards::GroupResourceAccessCard;
use crate::pages::shared::words::{first_name, kind_word, plain_name, when_view};
use crate::pages::step_up::{spawn_with_fresh_totp_to, spawn_with_step_up_to};
use crate::resource_access::ManagedGroupDto;
use crate::server::viewer::{use_viewer_access, viewer_now};
use crate::server::{
    add_group_group, add_group_owner_user, add_group_service_user, add_group_user, delete_group,
    get_group, get_group_access, get_group_managed, get_record_activity, remove_group_group,
    remove_group_owner_user, remove_group_service_user, remove_group_user, update_group,
    UpdateGroupInput, SUPER_USER_GROUP_ID,
};

/// Tab values.
const TAB_MEMBERS: &str = "members";
const TAB_ACCESS: &str = "access";
const TAB_OWNERS: &str = "owners";
const TAB_ACTIVITY: &str = "activity";

/// How long the Undo after a removal stays up.
const UNDO_FOR: Duration = Duration::from_secs(8);

/// "Owned by Ines Okoro and Kenji Watanabe. Owners approve requests to join…".
#[must_use]
pub fn owner_caption(owners: &[String]) -> String {
    let who = match owners {
        [] => "nobody yet".to_string(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    };
    format!(
        "Owned by {who}. Owners approve requests to join and add or remove members; each change asks for their authenticator code."
    )
}

/// When and by whom each member was added, from the group's activity rows, keyed by label.
#[must_use]
pub fn added_by_label(rows: &[ActivityRow]) -> HashMap<String, (String, String)> {
    let mut out = HashMap::new();
    // Rows come newest first; keep the newest add for each label.
    for row in rows {
        if let Some(who) = row
            .summary
            .strip_prefix("Added ")
            .and_then(|r| r.strip_suffix(" to the group"))
        {
            out.entry(who.to_string())
                .or_insert_with(|| (row.at.to_rfc3339(), row.actor_label.clone()));
        }
    }
    out
}

/// Remove one member by kind; `totp_code` is only read for user and service-user members.
async fn remove_member(
    kind: PrincipalKind,
    group_id: String,
    principal_id: String,
    totp_code: String,
) -> Result<(), ServerFnError> {
    match kind {
        PrincipalKind::Group => remove_group_group(group_id, principal_id).await,
        PrincipalKind::ServiceUser => {
            remove_group_service_user(group_id, principal_id, totp_code).await
        }
        _ => remove_group_user(group_id, principal_id, totp_code).await,
    }
}

/// Add one member by kind; `totp_code` is only read for user and service-user members.
async fn add_member(
    kind: PrincipalKind,
    group_id: String,
    principal_id: String,
    totp_code: String,
) -> Result<(), ServerFnError> {
    match kind {
        PrincipalKind::Group => add_group_group(group_id, principal_id).await,
        PrincipalKind::ServiceUser => {
            add_group_service_user(group_id, principal_id, totp_code).await
        }
        _ => add_group_user(group_id, principal_id, totp_code).await,
    }
}

/// Run a member change: groups and ordinary groups use the sudo window, Super User wants a
/// fresh code for people.
fn spawn_member_change<F, Fut>(
    group_id: &str,
    kind: &PrincipalKind,
    confirm: String,
    error: RwSignal<Option<String>>,
    done: Callback<()>,
    change: F,
) where
    F: Fn(String) -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<(), ServerFnError>> + 'static,
{
    if group_id == SUPER_USER_GROUP_ID && *kind != PrincipalKind::Group {
        spawn_with_fresh_totp_to(confirm, error, done, change);
    } else {
        spawn_with_step_up_to(confirm, error, done, move || change(String::new()));
    }
}

/// The success bar after a change, with Undo when the change can be undone.
#[derive(Clone, Debug)]
struct Notice {
    title: String,
    text: String,
    undo: Option<PrincipalRefDto>,
}

/// The banner on a group another product manages: where its members come from, or that the
/// team is archived, with a link to the team.
#[component]
fn ManagedGroupBanner(managed: ManagedGroupDto, group_name: String) -> impl IntoView {
    let source = managed.source_label.clone();
    let (intent, text) = if managed.archived {
        (
            MessageBarIntent::Warning,
            format!(
                "{group_name} is archived in {source}, so this group has no members. Its grants stay, and restoring the team brings its members back."
            ),
        )
    } else {
        (
            MessageBarIntent::Info,
            format!(
                "Members come from the {group_name} team in {source} and change when the team does. Add or remove people there."
            ),
        )
    };
    let link = managed.source_href.clone().map(|href| {
        view! {
            <span data-testid="gauge-group-managed-open">
                <Link href=href>{format!("Open in {source}")}</Link>
            </span>
        }
    });
    view! {
        <div data-testid="gauge-group-managed-banner">
        <MessageBar intent=intent layout=MessageBarLayout::Multiline>
            <Flex vertical=true gap=FlexGap::Small>
                <Body1>{text}</Body1>
                {link}
            </Flex>
        </MessageBar>
        </div>
    }
}

#[component]
fn GroupCrumbs(name: String) -> impl IntoView {
    view! {
        <Breadcrumb>
            <BreadcrumbItem>
                <BreadcrumbButton href=crate::paths::GROUPS.to_string()>"Groups"</BreadcrumbButton>
            </BreadcrumbItem>
            <BreadcrumbDivider />
            <BreadcrumbItem>
                <BreadcrumbButton current=true>{name}</BreadcrumbButton>
            </BreadcrumbItem>
        </Breadcrumb>
    }
}

/// Group page.
#[component]
pub fn GroupDetailPage() -> impl IntoView {
    let params = use_params_map();
    let group_id = Memo::new(move |_| params.read().get("id").unwrap_or_default());
    let refresh = RwSignal::new(0u64);
    // Lives above the Transition, so a refetch after a change keeps the open tab.
    // `?request=1` opens Request access once per visit, not again after each refetch.
    let auto_opened = StoredValue::new(false);
    // The success bar (and its Undo) outlives the refetch a change triggers.
    let notice = RwSignal::new(None::<Notice>);
    let notice_seq = RwSignal::new(0u64);
    let tab = RwSignal::new(TAB_MEMBERS.to_string());
    let _viewer = use_viewer_access();
    let detail = Resource::new(
        move || (group_id.get(), refresh.get()),
        move |(id, _)| get_group(id),
    );
    let navigate = use_navigate();
    let skeleton = || {
        view! {
            <div data-testid="gauge-group-skeleton">
                <Flex vertical=true gap=FlexGap::Medium>
                    <SkeletonItem width="120px".to_string() />
                    <SkeletonItem width="240px".to_string() />
                    <Card><CardContent>
                        <Flex vertical=true gap=FlexGap::Small>
                            {(0..4).map(|_| view! { <SkeletonItem width="100%".to_string() /> }).collect_view()}
                        </Flex>
                    </CardContent></Card>
                </Flex>
            </div>
        }
    };
    view! {
        <ContentContainer max_width="1100px" data_testid="gauge-group-page">
            <Transition fallback=skeleton>
                {move || detail.get().map(|result| match result {
                    Ok(Some(row)) => view! { <GroupLoaded row=row refresh=refresh tab=tab auto_opened=auto_opened notice=notice notice_seq=notice_seq /> }.into_any(),
                    Ok(None) => {
                        let navigate = navigate.clone();
                        view! {
                            <div data-testid="gauge-group-not-found">
                                <Flex vertical=true gap=FlexGap::Medium>
                                    <GroupCrumbs name="Not found".to_string() />
                                    <EmptyState
                                        message="No group with that link"
                                        description="It may have been deleted, or the link has a typo."
                                        icon=icondata_core::Icon::from(icondata::AiSearchOutlined)
                                    >
                                        <EmptyStateCallToAction slot:call_to_action>
                                            <Button
                                                appearance=ButtonAppearance::Primary
                                                on_click=Callback::new(move |_| navigate(crate::paths::GROUPS, NavigateOptions::default()))
                                            >
                                                "Back to Groups"
                                            </Button>
                                        </EmptyStateCallToAction>
                                    </EmptyState>
                                </Flex>
                            </div>
                        }.into_any()
                    }
                    Err(err) => view! {
                        <Flex vertical=true gap=FlexGap::Medium>
                            <GroupCrumbs name="Group".to_string() />
                            <ErrorBar error=err.to_string() what="this group" on_retry=Callback::new(move |()| refresh.update(|n| *n += 1)) />
                        </Flex>
                    }.into_any(),
                })}
            </Transition>
        </ContentContainer>
    }
}

/// The page once the group loaded.
#[component]
fn GroupLoaded(
    row: PermissionGroupDetailDto,
    refresh: RwSignal<u64>,
    tab: RwSignal<String>,
    auto_opened: StoredValue<bool>,
    notice: RwSignal<Option<Notice>>,
    notice_seq: RwSignal<u64>,
) -> impl IntoView {
    let viewer = use_viewer_access();
    let row = StoredValue::new(row);
    let group_id = row.with_value(|r| r.id.clone());
    let group_name = row.with_value(|r| r.name.clone());
    let is_super = group_id == SUPER_USER_GROUP_ID;
    let managed_res = crate::pages::shared::local_resource(
        {
            let id = group_id.clone();
            move || (id.clone(), refresh.get())
        },
        |(id, _)| get_group_managed(id),
    );
    let managed = move || managed_res.get().and_then(Result::ok).flatten();
    let can_manage = {
        let id = group_id.clone();
        Signal::derive(move || viewer_now(viewer).can_manage_group(&id))
    };
    let is_admin = Signal::derive(move || viewer_now(viewer).gauge_admin);
    let can_request = row.with_value(|r| r.can_request_access);

    let error = RwSignal::new(None::<String>);
    let add_open = RwSignal::new(false);
    let edit_open = RwSignal::new(false);
    let delete_open = RwSignal::new(false);
    let request_open = RwSignal::new(false);
    let query = use_query_map();
    let preselect_for = Signal::derive(move || requested_for_handle(&query.read()));
    Effect::new(move |_| {
        if can_request && !auto_opened.get_value() && wants_request_dialog(&query.read_untracked())
        {
            auto_opened.set_value(true);
            request_open.set(true);
        }
    });

    let show_notice = move |n: Notice| {
        let seq = notice_seq.get_untracked().saturating_add(1);
        notice_seq.set(seq);
        let has_undo = n.undo.is_some();
        notice.set(Some(n));
        if has_undo {
            // Drop the Undo after a few seconds; the sentence stays.
            leptos::leptos_dom::helpers::set_timeout(
                move || {
                    if notice_seq.get_untracked() == seq {
                        notice.update(|n| {
                            if let Some(n) = n {
                                n.undo = None;
                            }
                        });
                    }
                },
                UNDO_FOR,
            );
        }
    };

    let undo = {
        let group_id = group_id.clone();
        let group_name = group_name.clone();
        move |_| {
            let Some(principal) = notice.get().and_then(|n| n.undo) else {
                return;
            };
            let gid = group_id.clone();
            let label = plain_name(&principal.label);
            let kind = principal.kind.clone();
            let pid = principal.id.clone();
            // Clear the bar only once the add lands: the Undo button lives in it, and
            // unmounting it first would cancel this task.
            let done = Callback::new(move |()| {
                notice.set(None);
                refresh.update(|n| *n += 1);
            });
            let kind_ref = kind.clone();
            spawn_member_change(
                &gid,
                &kind_ref,
                format!("Add {label} back to {group_name}"),
                error,
                done,
                {
                    let gid = gid.clone();
                    move |code| add_member(kind.clone(), gid.clone(), pid.clone(), code)
                },
            );
        }
    };

    let undo_cb = Callback::new(undo);
    let owners_labels = row.with_value(|r| {
        r.owner_users
            .iter()
            .map(|o| plain_name(&o.label))
            .collect::<Vec<_>>()
    });
    let member_count = row.with_value(|r| r.members.len());
    let owner_count = row.with_value(|r| r.owner_users.len());
    let record = RecordRef {
        kind: RecordKind::Group,
        id: group_id.clone(),
        label: group_name.clone(),
    };
    let record = StoredValue::new(record);
    let description = row.with_value(|r| r.description.clone());
    let name_for_banner = group_name.clone();

    view! {
        <Flex vertical=true gap=FlexGap::Medium>
            <GroupCrumbs name=group_name.clone() />
            <Show when=move || notice.get().is_some()>
                <div data-testid="gauge-group-notice">
                    <MessageBar intent=MessageBarIntent::Success layout=MessageBarLayout::Multiline>
                        <MessageBarBody>
                            <MessageBarTitle>{move || notice.get().map(|n| n.title).unwrap_or_default()}</MessageBarTitle>
                            {move || notice.get().map(|n| n.text).unwrap_or_default()}
                        </MessageBarBody>
                        <MessageBarActions>
                            <Show when=move || notice.get().is_some_and(|n| n.undo.is_some())>
                                <span data-testid="gauge-group-undo">
                                    <Button appearance=ButtonAppearance::Secondary on_click=undo_cb>"Undo"</Button>
                                </span>
                            </Show>
                        </MessageBarActions>
                    </MessageBar>
                </div>
            </Show>
            <div id="gauge-group-detail-header">
                <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Start gap=FlexGap::Medium wrap=FlexWrap::Wrap>
                    <Flex vertical=true gap=FlexGap::Small>
                        <Flex align=FlexAlign::Center gap=FlexGap::Small wrap=FlexWrap::Wrap>
                            <Title3 tag=TextTag::H1>{group_name.clone()}</Title3>
                            <Show when=move || managed().is_some()>
                                <span data-testid="gauge-group-team-badge">
                                    <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Informative size=BadgeSize::Small>"Team"</Badge>
                                </span>
                            </Show>
                            <Show when=move || managed().is_some_and(|m| m.archived)>
                                <span data-testid="gauge-group-archived-badge">
                                    <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Subtle size=BadgeSize::Small>"Archived team"</Badge>
                                </span>
                            </Show>
                        </Flex>
                        <MarkdownView markdown=description testid="gauge-group-description" />
                    </Flex>
                    <div id="gauge-group-actions">
                        <Flex gap=FlexGap::Small>
                            <div id="gauge-group-add-members">
                                <Show when=move || can_manage.get() && managed().is_none()>
                                    <Button appearance=ButtonAppearance::Primary on_click=Callback::new(move |_| add_open.set(true))>
                                        "Add members"
                                    </Button>
                                </Show>
                            </div>
                            <div id="gauge-group-request-access">
                                <Show when=move || !can_manage.get() && can_request>
                                    <Button appearance=ButtonAppearance::Primary on_click=Callback::new(move |_| request_open.set(true))>
                                        "Request access"
                                    </Button>
                                </Show>
                            </div>
                            <Show when=move || is_admin.get()>
                                <div id="gauge-group-more">
                                    <Menu on_select=move |action: &str| match action {
                                        "edit" => edit_open.set(true),
                                        "delete" => delete_open.set(true),
                                        _ => {}
                                    }>
                                        <MenuTrigger slot>
                                            <Button appearance=ButtonAppearance::Secondary icon=icondata::AiMoreOutlined attr:aria-label="More actions" />
                                        </MenuTrigger>
                                        <MenuItem value="edit">"Edit"</MenuItem>
                                        <MenuItem value="delete">"Delete group"</MenuItem>
                                    </Menu>
                                </div>
                            </Show>
                        </Flex>
                    </div>
                </Flex>
            </div>
            <Caption1 attr:data-testid="gauge-group-owner-caption">{owner_caption(&owners_labels)}</Caption1>
            {is_super.then(|| view! {
                <div data-testid="gauge-group-super-user-warning">
                    <MessageBar intent=MessageBarIntent::Warning layout=MessageBarLayout::Multiline>
                        <MessageBarBody>
                            <MessageBarTitle>"Super User passes every check"</MessageBarTitle>
                            "Members can do everything in every app, including change Gauge. Keep it to two or three people, and each change needs a fresh code."
                        </MessageBarBody>
                    </MessageBar>
                </div>
            })}
            {move || managed().map(|m| view! { <ManagedGroupBanner managed=m group_name=name_for_banner.clone() /> })}
            <MaybeErrorBar error=error what="this group" />
            <Card>
                <CardContent>
                    <div id="gauge-group-tabs">
                        <Flex vertical=true gap=FlexGap::Medium>
                            <TabList selected_value=tab>
                                <Tab value=TAB_MEMBERS>{format!("Members ({member_count})")}</Tab>
                                <Tab value=TAB_ACCESS>"Access"</Tab>
                                <Tab value=TAB_OWNERS>{format!("Owners ({owner_count})")}</Tab>
                                <Tab value=TAB_ACTIVITY>"Activity"</Tab>
                            </TabList>
                            {move || match tab.get().as_str() {
                                TAB_ACCESS => view! { <GroupAccessTab group_id=row.with_value(|r| r.id.clone()) /> }.into_any(),
                                TAB_OWNERS => view! {
                                    <OwnersTab row=row.get_value() can_manage=can_manage refresh=refresh error=error on_notice=Callback::new(show_notice) />
                                }.into_any(),
                                TAB_ACTIVITY => view! { <RecordActivity record=record.get_value() /> }.into_any(),
                                _ => view! {
                                    <MembersTab
                                        row=row.get_value()
                                        managed=Signal::derive(managed)
                                        can_manage=can_manage
                                        refresh=refresh
                                        error=error
                                        on_notice=Callback::new(show_notice)
                                    />
                                }.into_any(),
                            }}
                        </Flex>
                    </div>
                </CardContent>
            </Card>
            <GroupResourceAccessCard group_id=Signal::derive(move || row.with_value(|r| r.id.clone())) />
            <AddMembersDialog open=add_open row=row.get_value() refresh=refresh error=error />
            <Show when=move || is_admin.get()>
                <EditGroupDialog open=edit_open row=row.get_value() refresh=refresh />
                <DeleteGroupDialog open=delete_open row=row.get_value() />
            </Show>
            <RequestAccessDialog
                open=request_open
                target_kind=PermissionRequestTargetKind::Group
                target_id=Signal::derive(move || row.with_value(|r| r.id.clone()))
                preselect_for=preselect_for
                on_submitted=Callback::new(move |()| refresh.update(|n| *n += 1))
            />
        </Flex>
    }
}

fn text_of(record: &DataRecord, field: &str) -> String {
    match record.get(field) {
        Some(DataValue::Text(t)) => t.clone(),
        _ => String::new(),
    }
}

#[component]
fn MembersTab(
    row: PermissionGroupDetailDto,
    managed: Signal<Option<ManagedGroupDto>>,
    can_manage: Signal<bool>,
    refresh: RwSignal<u64>,
    error: RwSignal<Option<String>>,
    on_notice: Callback<Notice>,
) -> impl IntoView {
    let row = StoredValue::new(row);
    let remove_open = RwSignal::new(false);
    let remove_target = RwSignal::new(None::<RemoveTarget>);
    let activity = crate::pages::shared::local_resource(
        move || row.with_value(|r| r.id.clone()),
        move |id| {
            let name = row.with_value(|r| r.name.clone());
            get_record_activity(RecordRef {
                kind: RecordKind::Group,
                id,
                label: name,
            })
        },
    );
    let added = Signal::derive(move || {
        activity
            .get()
            .and_then(Result::ok)
            .map(|rows| added_by_label(&rows))
            .unwrap_or_default()
    });
    let members_by_key = StoredValue::new(row.with_value(|r| {
        r.members
            .iter()
            .map(|m| (format!("{:?}:{}", m.kind, m.id), m.clone()))
            .collect::<HashMap<_, _>>()
    }));
    let on_remove = Callback::new(move |()| {
        let Some(target) = remove_target.get() else {
            return;
        };
        remove_open.set(false);
        let RemoveFrom::Group { id: gid, name } = target.from.clone() else {
            return;
        };
        let principal = target.principal.clone();
        let confirm = remove_action(&target);
        let label = plain_name(&principal.label);
        let first = first_name(&label);
        let undo_principal = principal.clone();
        let done = Callback::new(move |()| {
            refresh.update(|n| *n += 1);
            on_notice.run(Notice {
                title: format!("{label} removed from {name}"),
                text: format!(
                    "{first} no longer gets what {name} gives. {first} got a notification."
                ),
                undo: Some(undo_principal.clone()),
            });
        });
        let kind = principal.kind.clone();
        let pid = principal.id.clone();
        spawn_member_change(
            &gid.clone(),
            &kind.clone(),
            confirm,
            error,
            done,
            move |code| remove_member(kind.clone(), gid.clone(), pid.clone(), code),
        );
    });

    view! {
        <div data-testid="gauge-group-members">
            {move || {
                {
                    if let Some(m) = managed.get() {
                        return view! { <ManagedMembers members=row.with_value(|r| r.members.clone()) managed=m /> }.into_any();
                    }
                }
                let members = row.with_value(|r| r.members.clone());
                if members.is_empty() {
                    return view! { <EmptyState message="No members yet." description="Owners add people, service users and groups with Add members." /> }.into_any();
                }
                let added_map = added.get();
                let items = RwSignal::new(members.iter().map(|m| {
                    let mut cells = HashMap::new();
                    cells.insert("member".into(), m.label.clone());
                    cells.insert("kind".into(), kind_word(&m.kind, m.owner_label.as_deref()));
                    let (at, by) = added_map.get(&m.label).cloned().unwrap_or_default();
                    cells.insert("added".into(), at);
                    cells.insert("added_by".into(), if by.is_empty() { "—".to_string() } else { by });
                    cells.insert("actions".into(), String::new());
                    DataTableRowModel::from_text_cells(format!("{:?}:{}", m.kind, m.id), cells)
                }).collect::<Vec<_>>());
                let gid = row.with_value(|r| r.id.clone());
                let gname = row.with_value(|r| r.name.clone());
                let mut columns = vec![
                    DataTableColumnDef::new("member", "Member").with_cell_view(Arc::new(|r: DataRecord| {
                        let member = text_of(&r, "member");
                        let kind = text_of(&r, "kind");
                        view! {
                            <Flex vertical=true gap=FlexGap::Size(0)>
                                <Body1Strong>{member}</Body1Strong>
                                <Caption1>{kind}</Caption1>
                            </Flex>
                        }.into_any()
                    })),
                    DataTableColumnDef::new("added", "Added").with_cell_view(Arc::new(|r: DataRecord| {
                        let at = text_of(&r, "added");
                        if at.is_empty() { view! { <Caption1>"—"</Caption1> }.into_any() } else { when_view(at).into_any() }
                    })),
                    DataTableColumnDef::new("added_by", "Added by"),
                ];
                if can_manage.get() {
                    columns.push(DataTableColumnDef::new("actions", "").with_sortable(false).with_cell_view(Arc::new(move |r: DataRecord| {
                        let member = members_by_key.with_value(|m| m.get(&r.id).cloned());
                        let Some(member) = member else { return view! { <span /> }.into_any() };
                        let gid = gid.clone();
                        let gname = gname.clone();
                        let aria = format!("Actions for {}", member.label);
                        view! {
                            <Menu on_select=move |action: &str| if action == "remove" {
                                remove_target.set(Some(RemoveTarget {
                                    principal: member.clone(),
                                    from: RemoveFrom::Group { id: gid.clone(), name: gname.clone() },
                                }));
                                remove_open.set(true);
                            }>
                                <MenuTrigger slot>
                                    <Button appearance=ButtonAppearance::Subtle icon=icondata::AiMoreOutlined attr:aria-label=aria.clone() />
                                </MenuTrigger>
                                <MenuItem value="remove">"Remove"</MenuItem>
                            </Menu>
                        }.into_any()
                    })));
                }
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
                        columns=columns
                    />
                }.into_any()
            }}
            <RemoveConfirmDialog open=remove_open target=remove_target on_confirm=on_remove />
        </div>
    }
}

/// Members of a managed group, read-only: the managing product owns the list.
#[component]
fn ManagedMembers(members: Vec<PrincipalRefDto>, managed: ManagedGroupDto) -> impl IntoView {
    let title = format!("Members ({})", members.len());
    let caption = format!("Read-only: {} manages this list.", managed.source_label);
    let empty_message = if managed.archived {
        "No members while the team is archived."
    } else {
        "No members to show."
    };
    view! {
        <div data-testid="gauge-group-managed-members">
            <Flex vertical=true gap=FlexGap::Small>
                <Body1Strong>{title}</Body1Strong>
                <Caption1>{caption}</Caption1>
                {if members.is_empty() {
                    view! { <EmptyState message=empty_message /> }.into_any()
                } else {
                    view! {
                        <Table>
                            <TableHeader><TableRow><TableHeaderCell>"Member"</TableHeaderCell></TableRow></TableHeader>
                            <TableBody>
                                {members.into_iter().map(|m| view! {
                                    <TableRow>
                                        <TableCell>
                                            <Flex vertical=true gap=FlexGap::Size(0)>
                                                <Body1Strong>{m.label.clone()}</Body1Strong>
                                                <Caption1>{kind_word(&m.kind, m.owner_label.as_deref())}</Caption1>
                                            </Flex>
                                        </TableCell>
                                    </TableRow>
                                }).collect_view()}
                            </TableBody>
                        </Table>
                    }.into_any()
                }}
            </Flex>
        </div>
    }
}

/// What every member gets from the group.
#[component]
fn GroupAccessTab(group_id: String) -> impl IntoView {
    let refresh = RwSignal::new(0u32);
    let access = crate::pages::shared::local_resource(
        move || (group_id.clone(), refresh.get()),
        |(id, _)| get_group_access(id),
    );
    view! {
        <div data-testid="gauge-group-access">
            <Flex vertical=true gap=FlexGap::Small>
                <Caption1>"What every member gets from this group. Groups inside it get the same."</Caption1>
                <Suspense fallback=|| view! { <SkeletonItem width="100%".to_string() /> }>
                    {move || access.get().map(|r| match r {
                        Ok(perms) if perms.is_empty() => view! {
                            <EmptyState message="This group gives no permissions yet." description="Add it to a permission's allow list to give its members that permission." />
                        }.into_any(),
                        Ok(perms) => view! {
                            <Table>
                                <TableHeader>
                                    <TableRow>
                                        <TableHeaderCell>"Permission"</TableHeaderCell>
                                        <TableHeaderCell>"How"</TableHeaderCell>
                                        <TableHeaderCell>""</TableHeaderCell>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    {perms.into_iter().map(|p| view! {
                                        <TableRow attr:data-testid="gauge-group-access-row">
                                            <TableCell>
                                                <Flex vertical=true gap=FlexGap::Size(0)>
                                                    <Body1Strong>{p.display_name.clone()}</Body1Strong>
                                                    <Caption1>"Key " <Code text=p.key.clone() /></Caption1>
                                                </Flex>
                                            </TableCell>
                                            <TableCell><Body1>"Allow list of the permission"</Body1></TableCell>
                                            <TableCell><Link href=format!("/permission/permissions/{}", p.id)>"Open"</Link></TableCell>
                                        </TableRow>
                                    }).collect_view()}
                                </TableBody>
                            </Table>
                        }.into_any(),
                        Err(err) => view! {
                            <ErrorBar error=err.to_string() what="what this group gives" on_retry=Callback::new(move |()| refresh.update(|n| *n += 1)) />
                        }.into_any(),
                    })}
                </Suspense>
            </Flex>
        </div>
    }
}

#[component]
fn OwnersTab(
    row: PermissionGroupDetailDto,
    can_manage: Signal<bool>,
    refresh: RwSignal<u64>,
    error: RwSignal<Option<String>>,
    on_notice: Callback<Notice>,
) -> impl IntoView {
    let row = StoredValue::new(row);
    let is_super = row.with_value(|r| r.id == SUPER_USER_GROUP_ID);
    let confirm_open = RwSignal::new(false);
    let pending = RwSignal::new(None::<PrincipalRefDto>);
    let owner_pick = RwSignal::new(Vec::<PickedPrincipal>::new());
    let on_owner_pick = act_on_pick(owner_pick, move |target| {
        let PickTarget::User(user_id) = target else {
            return;
        };
        let gid = row.with_value(|r| r.id.clone());
        let gname = row.with_value(|r| r.name.clone());
        let done = Callback::new(move |()| refresh.update(|n| *n += 1));
        let confirm = format!("Make someone an owner of {gname}");
        spawn_member_change(
            &gid.clone(),
            &PrincipalKind::User,
            confirm,
            error,
            done,
            move |code| add_group_owner_user(gid.clone(), user_id.clone(), code),
        );
    });
    let remove = move |_| {
        let Some(owner) = pending.get() else { return };
        confirm_open.set(false);
        let gid = row.with_value(|r| r.id.clone());
        let gname = row.with_value(|r| r.name.clone());
        let label = plain_name(&owner.label);
        let first = first_name(&label);
        let confirm = format!("Remove {label} as an owner of {gname}");
        let still_member = row.with_value(|r| r.members.iter().any(|m| m.id == owner.id));
        let done = Callback::new(move |()| {
            refresh.update(|n| *n += 1);
            let text = if still_member {
                format!("{first} is still a member. {first} got a notification.")
            } else {
                format!("{first} got a notification.")
            };
            on_notice.run(Notice {
                title: format!("{label} is no longer an owner of {gname}"),
                text,
                undo: None,
            });
        });
        let uid = owner.id.clone();
        spawn_member_change(
            &gid.clone(),
            &PrincipalKind::User,
            confirm,
            error,
            done,
            move |code| remove_group_owner_user(gid.clone(), uid.clone(), code),
        );
    };
    let body = move || {
        let Some(owner) = pending.get() else {
            return String::new();
        };
        let first = first_name(&owner.label);
        let gname = row.with_value(|r| r.name.clone());
        let member = row.with_value(|r| r.members.iter().any(|m| m.id == owner.id));
        let stays = if member {
            if is_super {
                format!(" {first} stays a member, so {first} still passes every permission check. Remove {first} under Members too if that should end.")
            } else {
                format!(
                    " {first} stays a member. Remove {first} under Members too if that should end."
                )
            }
        } else {
            String::new()
        };
        format!("{first} stops approving requests to join {gname} and can no longer add or remove its members.{stays}")
    };
    let caption = move || {
        let first = pending
            .get()
            .map(|o| first_name(&o.label))
            .unwrap_or_default();
        if is_super {
            format!(
                "{first} gets a notification. Super User changes need a fresh authenticator code."
            )
        } else {
            format!("{first} gets a notification. You'll confirm with your authenticator code.")
        }
    };
    view! {
        <div data-testid="gauge-group-owners">
            <Flex vertical=true gap=FlexGap::Small>
                <Caption1>"Owners approve requests to join and add or remove members."</Caption1>
                <Table>
                    <TableHeader>
                        <TableRow>
                            <TableHeaderCell>"Owner"</TableHeaderCell>
                            <TableHeaderCell>""</TableHeaderCell>
                        </TableRow>
                    </TableHeader>
                    <TableBody>
                        {move || row.with_value(|r| r.owner_users.clone()).into_iter().map(|o| {
                            let aria = format!("Actions for {}", o.label);
                            let label = o.label.clone();
                            view! {
                                <TableRow attr:data-testid="gauge-group-owner-row">
                                    <TableCell>
                                        <Flex vertical=true gap=FlexGap::Size(0)>
                                            <Body1Strong>{label}</Body1Strong>
                                            <Caption1>"Person"</Caption1>
                                        </Flex>
                                    </TableCell>
                                    <TableCell>
                                        <Show when=move || can_manage.get()>
                                            {
                                                let o = o.clone();
                                                let aria = aria.clone();
                                                view! {
                                                    <Menu on_select=move |action: &str| if action == "remove_owner" {
                                                        pending.set(Some(o.clone()));
                                                        confirm_open.set(true);
                                                    }>
                                                        <MenuTrigger slot>
                                                            <Button appearance=ButtonAppearance::Subtle icon=icondata::AiMoreOutlined attr:aria-label=aria.clone() />
                                                        </MenuTrigger>
                                                        <MenuItem value="remove_owner">"Remove as owner"</MenuItem>
                                                    </Menu>
                                                }
                                            }
                                        </Show>
                                    </TableCell>
                                </TableRow>
                            }
                        }).collect_view()}
                    </TableBody>
                </Table>
                <Show when=move || can_manage.get()>
                    <div id="gauge-group-owners-picker">
                        <PersonPicker
                            value=owner_pick
                            single=true
                            exclude=Signal::derive(move || row.with_value(|r| r.owner_users.iter().map(to_picked).collect::<Vec<_>>()))
                            label="Add an owner"
                            placeholder="Search people"
                            on_change=on_owner_pick
                        />
                    </div>
                </Show>
            </Flex>
            <Dialog open=confirm_open>
                <DialogSurface>
                    <DialogBody>
                        <DialogTitle>{move || pending.get().map(|o| format!("Remove {} as an owner of {}?", plain_name(&o.label), row.with_value(|r| r.name.clone()))).unwrap_or_default()}</DialogTitle>
                        <DialogContent>
                            <div data-testid="gauge-group-owner-remove-confirm">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Body1>{body}</Body1>
                                    <Caption1>{caption}</Caption1>
                                </Flex>
                            </div>
                        </DialogContent>
                        <DialogActions>
                            <Button appearance=ButtonAppearance::Subtle on_click=Callback::new(move |_| confirm_open.set(false))>"Cancel"</Button>
                            <Button appearance=ButtonAppearance::Danger attr:data-testid="gauge-group-owner-remove-button" on_click=Callback::new(remove)>
                                "Remove as owner"
                            </Button>
                        </DialogActions>
                    </DialogBody>
                </DialogSurface>
            </Dialog>
        </div>
    }
}

/// Add members: one picker for people, service users and groups.
#[component]
fn AddMembersDialog(
    open: RwSignal<bool>,
    row: PermissionGroupDetailDto,
    refresh: RwSignal<u64>,
    error: RwSignal<Option<String>>,
) -> impl IntoView {
    let row = StoredValue::new(row);
    let is_super = row.with_value(|r| r.id == SUPER_USER_GROUP_ID);
    let member_pick = RwSignal::new(Vec::<PickedPrincipal>::new());
    let query = RwSignal::new(String::new());
    // A group can't contain itself, so the picker leaves it out.
    let groups = use_group_search(
        query,
        Signal::derive(move || Some(row.with_value(|r| r.id.clone()))),
    );
    let on_pick = act_on_pick(member_pick, move |target| {
        let gid = row.with_value(|r| r.id.clone());
        let gname = row.with_value(|r| r.name.clone());
        let (kind, id) = match target {
            PickTarget::Group(id) => (PrincipalKind::Group, id),
            PickTarget::User(id) => (PrincipalKind::User, id),
            PickTarget::ServiceUser(id) => (PrincipalKind::ServiceUser, id),
        };
        open.set(false);
        let done = Callback::new(move |()| refresh.update(|n| *n += 1));
        let confirm = format!("Add a member to {gname}");
        let k = kind.clone();
        spawn_member_change(&gid.clone(), &kind, confirm, error, done, move |code| {
            add_member(k.clone(), gid.clone(), id.clone(), code)
        });
    });
    view! {
        <Dialog open=open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>{move || format!("Add members to {}", row.with_value(|r| r.name.clone()))}</DialogTitle>
                    <DialogContent>
                        <div id="gauge-group-members-picker">
                            <PersonPicker
                                value=member_pick
                                single=true
                                // Gauge never lets a service user into the Super User group.
                                service_users=!is_super
                                query=query
                                exclude=Signal::derive(move || row.with_value(|r| r.members.iter().map(to_picked).collect::<Vec<_>>()))
                                extra_groups=groups
                                label="Person, service user or group"
                                placeholder="Search people, service users or groups"
                                on_change=on_pick
                            />
                        </div>
                    </DialogContent>
                    <DialogActions>
                        <Button appearance=ButtonAppearance::Subtle on_click=Callback::new(move |_| open.set(false))>"Close"</Button>
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
        </Dialog>
    }
}

/// Edit the group's name and description (Gauge admins).
#[component]
fn EditGroupDialog(
    open: RwSignal<bool>,
    row: PermissionGroupDetailDto,
    refresh: RwSignal<u64>,
) -> impl IntoView {
    let id = row.id.clone();
    let name = RwSignal::new(row.name.clone());
    let description = RwSignal::new(row.description.clone());
    let error = RwSignal::new(None::<String>);
    let save = move |_| {
        let payload = UpdateGroupInput {
            id: id.clone(),
            name: name.get(),
            description: description.get(),
        };
        let done = Callback::new(move |()| {
            open.set(false);
            refresh.update(|n| *n += 1);
        });
        spawn_with_step_up_to(
            format!("Save changes to {}", payload.name),
            error,
            done,
            move || update_group(payload.clone()),
        );
    };
    view! {
        <Dialog open=open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>"Edit group"</DialogTitle>
                    <DialogContent>
                        <div id="gauge-group-edit-form">
                            <Flex vertical=true gap=FlexGap::Medium>
                                <Field label="Name"><Input bind=name /></Field>
                                <Field label="Description">
                                    <MarkdownEditor value=description aria_label="Description" />
                                </Field>
                                <MaybeErrorBar error=error what="this group" />
                            </Flex>
                        </div>
                    </DialogContent>
                    <DialogActions>
                        <Button appearance=ButtonAppearance::Subtle on_click=Callback::new(move |_| open.set(false))>"Cancel"</Button>
                        <div id="gauge-group-save">
                            <Button appearance=ButtonAppearance::Primary on_click=Callback::new(save)>"Save changes"</Button>
                        </div>
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
        </Dialog>
    }
}

/// Delete group asks first and says who loses what.
#[component]
fn DeleteGroupDialog(open: RwSignal<bool>, row: PermissionGroupDetailDto) -> impl IntoView {
    let navigate = use_navigate();
    let error = RwSignal::new(None::<String>);
    let id = row.id.clone();
    let name = row.name.clone();
    let members = row.members.len();
    let sentence = match members {
        0 => format!("{name} has no members, so nobody loses access."),
        1 => {
            format!("Its 1 member loses everything {name} gives, unless they hold it another way.")
        }
        n => format!(
            "Its {n} members lose everything {name} gives, unless they hold it another way."
        ),
    };
    let title = format!("Delete {name}?");
    let delete = move |_| {
        let id = id.clone();
        let navigate = navigate.clone();
        let done =
            Callback::new(move |()| navigate(crate::paths::GROUPS, NavigateOptions::default()));
        spawn_with_step_up_to(format!("Delete {name}"), error, done, move || {
            delete_group(id.clone())
        });
    };
    view! {
        <Dialog open=open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>{title}</DialogTitle>
                    <DialogContent>
                        <div data-testid="gauge-group-delete-confirm">
                            <Flex vertical=true gap=FlexGap::Small>
                                <Body1>{sentence}</Body1>
                                <Body1>"Permissions and groups it owns need a new owner first."</Body1>
                                <MaybeErrorBar error=error what="this group" />
                            </Flex>
                        </div>
                    </DialogContent>
                    <DialogActions>
                        <Button appearance=ButtonAppearance::Subtle on_click=Callback::new(move |_| open.set(false))>"Cancel"</Button>
                        <div id="gauge-group-delete">
                            <Button appearance=ButtonAppearance::Danger on_click=Callback::new(delete)>"Delete group"</Button>
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
    use gauge::types::ActivityKind;

    fn activity(summary: &str, actor: &str, at: i64) -> ActivityRow {
        ActivityRow {
            at: chrono::DateTime::from_timestamp(at, 0).unwrap_or_default(),
            actor_label: actor.into(),
            summary: summary.into(),
            record: RecordRef {
                kind: RecordKind::Group,
                id: "g".into(),
                label: "G".into(),
            },
            kind: ActivityKind::Membership,
        }
    }

    #[test]
    fn owner_caption_lists_owners_happy() {
        assert!(
            owner_caption(&["Ines Okoro".into(), "Kenji Watanabe".into()])
                .starts_with("Owned by Ines Okoro and Kenji Watanabe. Owners approve")
        );
        assert!(owner_caption(&[]).starts_with("Owned by nobody yet."));
    }

    #[test]
    fn newest_add_wins_and_removals_ignored_sad() {
        let rows = vec![
            activity("Added Ravi Menon to the group", "Sol", 1_793_000_000),
            activity("Removed Ravi Menon from the group", "Sol", 1_792_900_000),
            activity("Added Ravi Menon to the group", "Ines", 1_790_000_000),
        ];
        let map = added_by_label(&rows);
        assert_eq!(map.len(), 1);
        assert_eq!(map["Ravi Menon"].1, "Sol");
    }
}
