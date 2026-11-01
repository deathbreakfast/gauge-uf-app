use gauge::search_sources::PermissionSearchSourceId;
use gauge::types::{PermissionRequestTargetKind, PrincipalKind};
use leptos::prelude::*;
use leptos::task::spawn_local_scoped;
use leptos_router::hooks::{use_navigate, use_params_map, use_query_map};
use leptos_router::NavigateOptions;
use uf_integrations::SearchSourcePicker;
use uf_product::components::{
    Body1, Caption1, Card, CardContent, CardSectionBorder, EmptyState, SkeletonItemSize,
};
use uf_product::components::{ContentContainer, SpacingSize, Title3};
use uf_product::primitives::{
    Button, ButtonAppearance, Dialog, DialogActions, DialogBody, DialogContent, DialogSurface,
    DialogTitle, Field, Flex, FlexAlign, FlexGap, FlexJustify, Icon, Input, Menu, MenuItem,
    MenuTrigger, MessageBar, MessageBarIntent, SkeletonItem, Textarea,
};
use uf_search_core::{SearchSourceItem, SearchSourceKey};

use crate::pages::shared::history_dialog::HistoryDialog;
use crate::pages::shared::principal;
use crate::pages::shared::request_access::{
    requested_for_handle, wants_request_dialog, RequestAccessDialog,
};
use crate::pages::step_up::{spawn_with_fresh_totp, spawn_with_step_up};
use crate::server::{
    add_group_group, add_group_owner_user, add_group_user, delete_group, get_group,
    remove_group_group, remove_group_owner_user, remove_group_service_user, remove_group_user,
    search_principals, update_group, UpdateGroupInput, SUPER_USER_GROUP_ID,
};

/// Group detail/edit page: owners and members pickers, name/description editing,
/// delete, audit history, and a request-access flow.
#[component]
// Route page owns member/owner/history UI; split later if modules stay cohesive.
#[allow(clippy::too_many_lines)]
pub fn GroupDetailPage() -> impl IntoView {
    let navigate = use_navigate();
    let navigate_store = StoredValue::new(navigate.clone());
    let params = use_params_map();
    let group_id = Memo::new(move |_| params.read().get("id").unwrap_or_default());
    let refresh = RwSignal::new(0u64);

    let detail = Resource::new(
        move || (group_id.get(), refresh.get()),
        move |(id, _)| async move { get_group(id).await },
    );

    let name = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let picker_options = RwSignal::new(Vec::<SearchSourceItem>::new());
    let picker_request_seq = RwSignal::new(0u64);
    let owner_picker_options = RwSignal::new(Vec::<SearchSourceItem>::new());
    let owner_picker_request_seq = RwSignal::new(0u64);
    let error = RwSignal::new(None::<String>);
    let picker_error = RwSignal::new(None::<String>);
    let owner_picker_error = RwSignal::new(None::<String>);
    let confirm_remove_open = RwSignal::new(false);
    let pending_remove = RwSignal::new(None::<(String, PrincipalKind, String)>);
    let confirm_remove_owner_open = RwSignal::new(false);
    let pending_remove_owner = RwSignal::new(None::<(String, String)>);
    let request_dialog_open = RwSignal::new(false);
    let query = use_query_map();
    let preselect_for = Signal::derive(move || requested_for_handle(&query.read()));
    // `?request=1` opens Request Access once, when the actor may request.
    let request_auto_opened = StoredValue::new(false);

    Effect::new(move |_| {
        if let Some(Ok(Some(row))) = detail.get() {
            if row.can_request_access
                && !request_auto_opened.get_value()
                && wants_request_dialog(&query.read_untracked())
            {
                request_auto_opened.set_value(true);
                request_dialog_open.set(true);
            }
            name.set(row.name);
            description.set(row.description);
        }
    });

    let save = move |_| {
        let payload = UpdateGroupInput {
            id: group_id.get(),
            name: name.get(),
            description: description.get(),
        };
        spawn_local_scoped(async move {
            match update_group(payload).await {
                Ok(()) => refresh.update(|n| *n += 1),
                Err(err) => error.set(Some(err.to_string())),
            }
        });
    };

    let request_initial = Callback::new(move |sources: Vec<SearchSourceKey>| {
        let request_id = picker_request_seq.get_untracked().saturating_add(1);
        picker_request_seq.set(request_id);
        spawn_local_scoped(async move {
            match search_principals(sources, None, 10).await {
                Ok(rows) => {
                    if picker_request_seq.get_untracked() == request_id {
                        picker_options.set(rows);
                        picker_error.set(None);
                    }
                }
                Err(err) => picker_error.set(Some(err.to_string())),
            }
        });
    });

    let request_search = Callback::new(move |(sources, query): (Vec<SearchSourceKey>, String)| {
        let request_id = picker_request_seq.get_untracked().saturating_add(1);
        picker_request_seq.set(request_id);
        spawn_local_scoped(async move {
            match search_principals(sources, Some(query), 10).await {
                Ok(rows) => {
                    if picker_request_seq.get_untracked() == request_id {
                        picker_options.set(rows);
                        picker_error.set(None);
                    }
                }
                Err(err) => picker_error.set(Some(err.to_string())),
            }
        });
    });

    let on_select = Callback::new(move |item: SearchSourceItem| {
        let group_id = group_id.get();
        let finish_ok = Callback::new(move |_: ()| {
            refresh.update(|n| *n += 1);
            error.set(None);
        });
        if item.source_id == PermissionSearchSourceId::PermissionGroup.as_str() {
            spawn_with_step_up(error, finish_ok, move || {
                let group_id = group_id.clone();
                let nested_id = item.id.clone();
                async move { add_group_group(group_id, nested_id).await }
            });
        } else if group_id == SUPER_USER_GROUP_ID {
            spawn_with_fresh_totp(error, finish_ok, move |totp_code| {
                let group_id = group_id.clone();
                let user_id = item.id.clone();
                async move { add_group_user(group_id, user_id, totp_code).await }
            });
        } else {
            spawn_with_step_up(error, finish_ok, move || {
                let group_id = group_id.clone();
                let user_id = item.id.clone();
                async move { add_group_user(group_id, user_id, String::new()).await }
            });
        }
    });

    let owner_request_initial = Callback::new(move |sources: Vec<SearchSourceKey>| {
        let request_id = owner_picker_request_seq.get_untracked().saturating_add(1);
        owner_picker_request_seq.set(request_id);
        spawn_local_scoped(async move {
            match search_principals(sources, None, 10).await {
                Ok(rows) => {
                    if owner_picker_request_seq.get_untracked() == request_id {
                        owner_picker_options.set(rows);
                        owner_picker_error.set(None);
                    }
                }
                Err(err) => owner_picker_error.set(Some(err.to_string())),
            }
        });
    });

    let owner_request_search =
        Callback::new(move |(sources, query): (Vec<SearchSourceKey>, String)| {
            let request_id = owner_picker_request_seq.get_untracked().saturating_add(1);
            owner_picker_request_seq.set(request_id);
            spawn_local_scoped(async move {
                match search_principals(sources, Some(query), 10).await {
                    Ok(rows) => {
                        if owner_picker_request_seq.get_untracked() == request_id {
                            owner_picker_options.set(rows);
                            owner_picker_error.set(None);
                        }
                    }
                    Err(err) => owner_picker_error.set(Some(err.to_string())),
                }
            });
        });

    let on_select_owner = Callback::new(move |item: SearchSourceItem| {
        let group_id = group_id.get();
        if item.kind != "user" {
            error.set(Some("Only users can be owners.".to_string()));
            return;
        }
        let finish_ok = Callback::new(move |_: ()| {
            refresh.update(|n| *n += 1);
            error.set(None);
        });
        if group_id == SUPER_USER_GROUP_ID {
            spawn_with_fresh_totp(error, finish_ok, move |totp_code| {
                let group_id = group_id.clone();
                let user_id = item.id.clone();
                async move { add_group_owner_user(group_id, user_id, totp_code).await }
            });
        } else {
            spawn_with_step_up(error, finish_ok, move || {
                let group_id = group_id.clone();
                let user_id = item.id.clone();
                async move { add_group_owner_user(group_id, user_id, String::new()).await }
            });
        }
    });

    view! {
        <ContentContainer max_width="1100px">
            <Transition fallback=move || view! {
                <CardContent>
                    <Flex vertical=true gap=FlexGap::Small>
                        {(0..3).map(|_| view! {
                            <SkeletonItem
                                size=Signal::from(SkeletonItemSize::S32)
                                width="100%".to_string()
                            />
                        }).collect_view()}
                    </Flex>
                </CardContent>
            }>
                {move || {
                    detail.get().map(|result| match result {
                        Ok(Some(row)) => {
                            let owner_users = RwSignal::new(row.owner_users.clone());
                            let members = RwSignal::new(row.members.clone());
                            view! {
                                <Flex vertical=true gap=FlexGap::Medium>
                                    <div id="gauge-group-detail-header">
                                    <Card>
                                        <Flex vertical=true gap=FlexGap::Small padding=SpacingSize::Size200.inset()>
                                            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center gap=FlexGap::Small>
                                                <Flex vertical=true gap=FlexGap::Small>
                                                    <Title3>"Group Detail"</Title3>
                                                    <Caption1>{format!("ID: {}", row.id.clone())}</Caption1>
                                                </Flex>
                                                <div id="gauge-group-request-access">
                                                <Show when=move || row.can_request_access>
                                                    <Button
                                                        appearance=ButtonAppearance::Primary
                                                        on_click=Callback::new(move |_| request_dialog_open.set(true))
                                                    >
                                                        "Request Access"
                                                    </Button>
                                                </Show>
                                                </div>
                                            </Flex>
                                        </Flex>
                                    </Card>
                                    </div>

                                    <Card>
                                        <Flex vertical=true gap=FlexGap::Medium padding=SpacingSize::Size200.inset()>
                                            <Flex vertical=true gap=FlexGap::Small>
                                                <Title3>"Owners"</Title3>
                                                <Caption1>"Owners can edit this group, manage owners, and manage members."</Caption1>
                                            </Flex>
                                            <div id="gauge-group-owners-picker">
                                            <SearchSourcePicker
                                                search_sources=Signal::derive(|| {
                                                    vec![PermissionSearchSourceId::User.into()]
                                                })
                                                options=owner_picker_options
                                                on_request_initial=owner_request_initial
                                                on_search=owner_request_search
                                                on_select=on_select_owner
                                            />
                                            </div>
                                            <Show when=move || owner_picker_error.get().is_some()>
                                                <MessageBar intent=MessageBarIntent::Error>
                                                    {move || owner_picker_error.get().unwrap_or_default()}
                                                </MessageBar>
                                            </Show>
                                            <div id="gauge-group-owner-remove">
                                            <Show when=move || !owner_users.get().is_empty() fallback=move || view! {
                                                <EmptyState message="No owners are assigned to this group." />
                                            }>
                                                <Flex vertical=true gap=FlexGap::Small>
                                                    <For each=move || owner_users.get() key=|p| format!("{}:{}", p.label, p.id) let:owner>
                                                        <>
                                                            <Flex
                                                                justify=FlexJustify::SpaceBetween
                                                                align=FlexAlign::Center
                                                                gap=FlexGap::Small
                                                                padding=SpacingSize::Size120.inset()
                                                            >
                                                            {
                                                                let owner_label_text = owner.label.clone();
                                                                let owner_label_menu = owner.label.clone();
                                                                let owner_id_text = owner.id.clone();
                                                                let owner_id_action = owner.id.clone();
                                                                view! {
                                                                    <Flex vertical=true gap=FlexGap::Small>
                                                                        <Body1>{owner_label_text}</Body1>
                                                                        <Caption1>{format!("user ({owner_id_text})")}</Caption1>
                                                                    </Flex>
                                                                    <Menu
                                                                        on_select={
                                                                            let owner_label = owner_label_menu;
                                                                            move |action: &str| {
                                                                                if action == "remove_owner" {
                                                                                    pending_remove_owner.set(Some((
                                                                                        owner_id_action.clone(),
                                                                                        owner_label.clone(),
                                                                                    )));
                                                                                    confirm_remove_owner_open.set(true);
                                                                                }
                                                                            }
                                                                        }
                                                                    >
                                                                        <MenuTrigger slot>
                                                                            <Button
                                                                                appearance=ButtonAppearance::Subtle
                                                                                icon=icondata::AiMoreOutlined
                                                                                attr:aria-label="Open owner actions"
                                                                            />
                                                                        </MenuTrigger>
                                                                        <MenuItem value="remove_owner">"Remove Owner"</MenuItem>
                                                                    </Menu>
                                                                }
                                                            }
                                                            </Flex>
                                                            <CardSectionBorder />
                                                        </>
                                                    </For>
                                                </Flex>
                                            </Show>
                                            </div>
                                        </Flex>
                                    </Card>

                                    <div id="gauge-group-edit-form">
                                    <Card>
                                        <Flex vertical=true gap=FlexGap::Medium padding=SpacingSize::Size200.inset()>
                                            <Field label="Name">
                                                <Input bind=name />
                                            </Field>
                                            <Field label="Description">
                                                <Textarea bind=description />
                                            </Field>
                                            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center gap=FlexGap::Small>
                                                <HistoryDialog
                                                    subject_kind=Signal::derive(|| "permission_group".to_string())
                                                    subject_id=Signal::derive(move || group_id.get())
                                                    trigger_id="gauge-group-show-history"
                                                />
                                                <Flex gap=FlexGap::Small>
                                                    <div id="gauge-group-delete">
                                                    <Button
                                                        appearance=ButtonAppearance::Secondary
                                                        on_click=Callback::new(move |_| {
                                                            let id = group_id.get();
                                                            let finish_ok = Callback::new(move |_: ()| {
                                                                navigate_store.with_value(|nav| {
                                                                    nav(crate::paths::GROUPS, NavigateOptions::default())
                                                                });
                                                            });
                                                            spawn_with_step_up(error, finish_ok, move || {
                                                                let id = id.clone();
                                                                async move { delete_group(id).await }
                                                            });
                                                        })
                                                    >
                                                        "Delete Group"
                                                    </Button>
                                                    </div>
                                                    <div id="gauge-group-save">
                                                    <Button appearance=ButtonAppearance::Primary on_click=Callback::new(save)>
                                                        "Save Changes"
                                                    </Button>
                                                    </div>
                                                </Flex>
                                            </Flex>
                                            <Show when=move || error.get().is_some()>
                                                <MessageBar intent=MessageBarIntent::Error>
                                                    {move || error.get().unwrap_or_default()}
                                                </MessageBar>
                                            </Show>
                                        </Flex>
                                    </Card>
                                    </div>

                                    <Card>
                                        <Flex vertical=true gap=FlexGap::Medium padding=SpacingSize::Size200.inset()>
                                            <Flex vertical=true gap=FlexGap::Small>
                                                <Title3>"Members"</Title3>
                                                <Caption1>"Manage user and nested-group membership."</Caption1>
                                            </Flex>
                                            <div id="gauge-group-members-picker">
                                            <SearchSourcePicker
                                                search_sources=Signal::derive(|| vec![
                                                    PermissionSearchSourceId::User.into(),
                                                    PermissionSearchSourceId::PermissionGroup
                                                        .into(),
                                                ])
                                                options=picker_options
                                                on_request_initial=request_initial
                                                on_search=request_search
                                                on_select=on_select
                                            />
                                            </div>
                                            <Show when=move || picker_error.get().is_some()>
                                                <MessageBar intent=MessageBarIntent::Error>
                                                    {move || picker_error.get().unwrap_or_default()}
                                                </MessageBar>
                                            </Show>
                                            <div id="gauge-group-member-remove">
                                            <Show when=move || !members.get().is_empty() fallback=move || view! {
                                                <EmptyState message="No members are currently assigned to this group." />
                                            }>
                                                <Flex vertical=true gap=FlexGap::Small>
                                                    <For each=move || members.get() key=|p| format!("{}:{}", p.label, p.id) let:principal>
                                                        <>
                                                            <Flex
                                                                justify=FlexJustify::SpaceBetween
                                                                align=FlexAlign::Center
                                                                gap=FlexGap::Small
                                                                padding=SpacingSize::Size120.inset()
                                                            >
                                                            {
                                                                let principal_label_text = principal.label.clone();
                                                                let principal_label_menu = principal.label.clone();
                                                                let principal_id_action = principal.id.clone();
                                                                let kind = principal.kind.clone();
                                                                let kind_icon = principal::kind_icon(&kind);
                                                                let detail_line = principal::detail_line(&principal);
                                                                view! {
                                                                    <Flex gap=FlexGap::Small align=FlexAlign::Center>
                                                                        <Icon icon=kind_icon />
                                                                        <Flex vertical=true gap=FlexGap::Small>
                                                                            <Body1>{principal_label_text}</Body1>
                                                                            <Caption1>{detail_line}</Caption1>
                                                                        </Flex>
                                                                    </Flex>
                                                                    <Menu
                                                                        on_select={
                                                                            let principal_label = principal_label_menu;
                                                                            move |action: &str| {
                                                                                if action == "remove" {
                                                                                    pending_remove.set(Some((
                                                                                        principal_id_action.clone(),
                                                                                        kind.clone(),
                                                                                        principal_label.clone(),
                                                                                    )));
                                                                                    confirm_remove_open.set(true);
                                                                                }
                                                                            }
                                                                        }
                                                                    >
                                                                        <MenuTrigger slot>
                                                                            <Button
                                                                                appearance=ButtonAppearance::Subtle
                                                                                icon=icondata::AiMoreOutlined
                                                                                attr:aria-label="Open member actions"
                                                                            />
                                                                        </MenuTrigger>
                                                                        <MenuItem value="remove">"Remove"</MenuItem>
                                                                    </Menu>
                                                                }
                                                            }
                                                            </Flex>
                                                            <CardSectionBorder />
                                                        </>
                                                    </For>
                                                </Flex>
                                            </Show>
                                            </div>
                                        </Flex>
                                    </Card>
                                    <Dialog open=confirm_remove_open>
                                        <DialogSurface>
                                            <DialogBody>
                                                <DialogTitle>"Confirm removal"</DialogTitle>
                                                <DialogContent>
                                                    <Body1>
                                                        {move || {
                                                            pending_remove.get().map_or_else(
                                                                || "Remove selected member?".to_string(),
                                                                |(_, _, label)| format!("Remove '{label}' from this group?"),
                                                            )
                                                        }}
                                                    </Body1>
                                                </DialogContent>
                                                <DialogActions>
                                                    <Button
                                                        appearance=ButtonAppearance::Secondary
                                                        on_click=Callback::new(move |_| {
                                                            confirm_remove_open.set(false);
                                                            pending_remove.set(None);
                                                        })
                                                    >
                                                        "Cancel"
                                                    </Button>
                                                    <Button
                                                        appearance=ButtonAppearance::Primary
                                                        on_click=Callback::new(move |_| {
                                                            let pending = pending_remove.get();
                                                            let gid = group_id.get();
                                                            if let Some((principal_id, kind, _)) = pending {
                                                                spawn_local_scoped(async move {
                                                                    let result = match kind {
                                                                        PrincipalKind::Group => remove_group_group(gid, principal_id).await,
                                                                        PrincipalKind::ServiceUser => {
                                                                            remove_group_service_user(gid, principal_id, String::new()).await
                                                                        }
                                                                        _ => remove_group_user(gid, principal_id, String::new()).await,
                                                                    };
                                                                    if let Err(err) = result {
                                                                        error.set(Some(err.to_string()));
                                                                    } else {
                                                                        refresh.update(|n| *n += 1);
                                                                        confirm_remove_open.set(false);
                                                                        pending_remove.set(None);
                                                                    }
                                                                });
                                                            }
                                                        })
                                                    >
                                                        "Remove"
                                                    </Button>
                                                </DialogActions>
                                            </DialogBody>
                                        </DialogSurface>
                                    </Dialog>
                                    <Dialog open=confirm_remove_owner_open>
                                        <DialogSurface>
                                            <DialogBody>
                                                <DialogTitle>"Confirm owner removal"</DialogTitle>
                                                <DialogContent>
                                                    <Body1>
                                                        {move || {
                                                            pending_remove_owner.get().map_or_else(
                                                                || "Remove selected owner?".to_string(),
                                                                |(_, label)| format!("Remove '{label}' as an owner of this group?"),
                                                            )
                                                        }}
                                                    </Body1>
                                                </DialogContent>
                                                <DialogActions>
                                                    <Button
                                                        appearance=ButtonAppearance::Secondary
                                                        on_click=Callback::new(move |_| {
                                                            confirm_remove_owner_open.set(false);
                                                            pending_remove_owner.set(None);
                                                        })
                                                    >
                                                        "Cancel"
                                                    </Button>
                                                    <Button
                                                        appearance=ButtonAppearance::Primary
                                                        on_click=Callback::new(move |_| {
                                                            let pending = pending_remove_owner.get();
                                                            let gid = group_id.get();
                                                            if let Some((owner_user_id, _)) = pending {
                                                                spawn_local_scoped(async move {
                                                                    let result = remove_group_owner_user(gid, owner_user_id, String::new()).await;
                                                                    if let Err(err) = result {
                                                                        error.set(Some(err.to_string()));
                                                                    } else {
                                                                        refresh.update(|n| *n += 1);
                                                                        confirm_remove_owner_open.set(false);
                                                                        pending_remove_owner.set(None);
                                                                    }
                                                                });
                                                            }
                                                        })
                                                    >
                                                        "Remove Owner"
                                                    </Button>
                                                </DialogActions>
                                            </DialogBody>
                                        </DialogSurface>
                                    </Dialog>
                                    <RequestAccessDialog
                                        open=request_dialog_open
                                        target_kind=PermissionRequestTargetKind::Group
                                        target_id=group_id
                                        preselect_for=preselect_for
                                        on_submitted=Callback::new(move |()| refresh.update(|n| *n += 1))
                                    />
                                </Flex>
                            }.into_any()
                        }
                        Ok(None) => view! {
                            <EmptyState message="Group not found." />
                        }.into_any(),
                        Err(err) => view! {
                            <MessageBar intent=MessageBarIntent::Error>
                                {format!("Failed to load group: {err}")}
                            </MessageBar>
                        }.into_any(),
                    })
                }}
            </Transition>
        </ContentContainer>
    }
}
