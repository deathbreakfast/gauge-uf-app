use gauge::search_sources::PermissionSearchSourceId;
use gauge::types::{PermissionRequestCreateInput, PermissionRequestTargetKind, PrincipalKind};
use leptos::prelude::*;
use leptos::task::spawn_local_scoped;
use leptos_router::hooks::{use_navigate, use_params_map};
use leptos_router::NavigateOptions;
use uf_integrations::SearchSourcePicker;
use uf_product::components::{
    Body1, Caption1, Card, CardContent, CardSectionBorder, EmptyState, SkeletonItemSize,
};
use uf_product::components::{ContentContainer, SpacingSize, Title3};
use uf_product::primitives::{
    Button, ButtonAppearance, Dialog, DialogActions, DialogBody, DialogContent, DialogSurface,
    DialogTitle, Field, Flex, FlexAlign, FlexGap, FlexJustify, Input, Menu, MenuItem, MenuTrigger,
    MessageBar, MessageBarIntent, Select, SelectAppearance, SkeletonItem, Textarea,
};
use uf_search_core::{SearchSourceItem, SearchSourceKey};

use crate::pages::shared::history_dialog::HistoryDialog;
use crate::pages::step_up::spawn_with_step_up;
use crate::server::{
    add_permission_group, add_permission_user, create_permission_request, delete_permission,
    get_permission, list_domains, remove_permission_group, remove_permission_user,
    search_principals, update_permission, UpdatePermissionInput,
};

/// Permission detail/edit page: name/description/owners/domain editing, allow-list
/// picker, delete, audit history, and a request-access flow.
#[component]
// Route page owns grant/revoke/history UI; split later if modules stay cohesive.
#[allow(clippy::too_many_lines)]
pub fn PermissionDetailPage() -> impl IntoView {
    let navigate = use_navigate();
    let navigate_store = StoredValue::new(navigate.clone());
    let params = use_params_map();
    let permission_id = Memo::new(move |_| params.read().get("id").unwrap_or_default());
    let refresh = RwSignal::new(0u64);

    let detail = Resource::new(
        move || (permission_id.get(), refresh.get()),
        move |(id, _)| async move { get_permission(id).await },
    );

    let name = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let owners_group_id = RwSignal::new(String::new());
    let domain_id = RwSignal::new(String::new());
    let domains = Resource::new(|| (), |()| async move { list_domains(None).await });
    let picker_options = RwSignal::new(Vec::<SearchSourceItem>::new());
    let picker_request_seq = RwSignal::new(0u64);
    let error = RwSignal::new(None::<String>);
    let picker_error = RwSignal::new(None::<String>);
    let confirm_remove_open = RwSignal::new(false);
    let pending_remove = RwSignal::new(None::<(String, bool, String)>);
    let request_dialog_open = RwSignal::new(false);
    let request_reason = RwSignal::new(String::new());
    let request_error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        if let Some(Ok(Some(row))) = detail.get() {
            name.set(row.name);
            description.set(row.description);
            owners_group_id.set(row.owners_group_id);
            domain_id.set(row.domain_id);
        }
    });

    let save = move |_| {
        let payload = UpdatePermissionInput {
            id: permission_id.get(),
            name: name.get(),
            description: description.get(),
            owners_group_id: owners_group_id.get(),
            domain_id: domain_id.get(),
        };
        let finish_ok = Callback::new(move |_: ()| {
            refresh.update(|n| *n += 1);
            error.set(None);
        });
        spawn_with_step_up(error, finish_ok, move || {
            let payload = payload.clone();
            async move { update_permission(payload).await }
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
        let permission_id = permission_id.get();
        let finish_ok = Callback::new(move |_: ()| {
            refresh.update(|n| *n += 1);
            error.set(None);
        });
        if item.source_id == PermissionSearchSourceId::PermissionGroup.as_str() {
            spawn_with_step_up(error, finish_ok, move || {
                let permission_id = permission_id.clone();
                let group_id = item.id.clone();
                async move { add_permission_group(permission_id, group_id).await }
            });
        } else {
            spawn_with_step_up(error, finish_ok, move || {
                let permission_id = permission_id.clone();
                let user_id = item.id.clone();
                async move { add_permission_user(permission_id, user_id).await }
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
                            let allow_list = RwSignal::new(row.allow_list.clone());
                            view! {
                                <Flex vertical=true gap=FlexGap::Medium>
                                    <div id="gauge-perm-detail-header">
                                    <Card>
                                        <Flex vertical=true gap=FlexGap::Small padding=SpacingSize::Size200.inset()>
                                            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center gap=FlexGap::Small>
                                                <Flex vertical=true gap=FlexGap::Small>
                                                    <Title3>"Permission Detail"</Title3>
                                                    <Caption1>{format!("ID: {}", row.id.clone())}</Caption1>
                                                </Flex>
                                                <div id="gauge-perm-request-access">
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

                                    <div id="gauge-perm-edit-form">
                                    <Card>
                                        <Flex vertical=true gap=FlexGap::Medium padding=SpacingSize::Size200.inset()>
                                            <Field label="Name">
                                                <Input bind=name />
                                            </Field>
                                            <Field label="Description">
                                                <Textarea bind=description />
                                            </Field>
                                            <Field label="Owners Group Id">
                                                <Input bind=owners_group_id />
                                            </Field>
                                            <Field label="Domain">
                                                <Select
                                                    bind=domain_id
                                                    appearance=SelectAppearance {
                                                        disabled: Signal::derive(move || {
                                                            domains.get().is_none()
                                                                || matches!(domains.get(), Some(Err(_)))
                                                        }),
                                                        ..Default::default()
                                                    }
                                                >
                                                    <option value="">"Select a domain"</option>
                                                    {move || {
                                                        domains
                                                            .get()
                                                            .and_then(std::result::Result::ok)
                                                            .unwrap_or_default()
                                                            .into_iter()
                                                            .map(|row| {
                                                                view! {
                                                                    <option value={row.id}>{row.name}</option>
                                                                }
                                                            })
                                                            .collect_view()
                                                    }}
                                                </Select>
                                                <Show when=move || domains.get().is_none()>
                                                    <Caption1>"Loading domains..."</Caption1>
                                                </Show>
                                                <Show
                                                    when=move || {
                                                        matches!(domains.get(), Some(Ok(items)) if items.is_empty())
                                                    }
                                                >
                                                    <Caption1>"No domains available. Create one first."</Caption1>
                                                </Show>
                                                <Show when=move || matches!(domains.get(), Some(Err(_)))>
                                                    <MessageBar intent=MessageBarIntent::Error>
                                                        {move || {
                                                            domains
                                                                .get()
                                                                .and_then(std::result::Result::err)
                                                                .map(|e| e.to_string())
                                                                .unwrap_or_default()
                                                        }}
                                                    </MessageBar>
                                                </Show>
                                            </Field>
                                            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center gap=FlexGap::Small>
                                                <HistoryDialog
                                                    subject_kind=Signal::derive(|| "permission".to_string())
                                                    subject_id=Signal::derive(move || permission_id.get())
                                                    trigger_id="gauge-show-history"
                                                />
                                                <Flex gap=FlexGap::Small>
                                                    <div id="gauge-perm-delete">
                                                    <Button
                                                        appearance=ButtonAppearance::Secondary
                                                        on_click=Callback::new(move |_| {
                                                            let id = permission_id.get();
                                                            let finish_ok = Callback::new(move |_: ()| {
                                                                navigate_store.with_value(|nav| {
                                                                    nav(crate::paths::PERMISSIONS, NavigateOptions::default())
                                                                });
                                                            });
                                                            spawn_with_step_up(error, finish_ok, move || {
                                                                let id = id.clone();
                                                                async move { delete_permission(id).await }
                                                            });
                                                        })
                                                    >
                                                        "Delete Permission"
                                                    </Button>
                                                    </div>
                                                    <div id="gauge-perm-save">
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
                                                <Title3>"Allow List"</Title3>
                                                <Caption1>"Add users or groups that should inherit this permission."</Caption1>
                                            </Flex>
                                            <div id="gauge-perm-allow-picker">
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
                                            <div id="gauge-perm-allow-remove">
                                            <Show when=move || !allow_list.get().is_empty() fallback=move || view! {
                                                <EmptyState message="No principals are currently in the allow list." />
                                            }>
                                                <Flex vertical=true gap=FlexGap::Small>
                                                    <For each=move || allow_list.get() key=|p| format!("{}:{}", p.label, p.id) let:principal>
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
                                                                let principal_id_text = principal.id.clone();
                                                                let principal_id_action = principal.id.clone();
                                                                let is_group = principal.kind == PrincipalKind::Group;
                                                                let principal_kind_label = if is_group { "group" } else { "user" };
                                                                view! {
                                                                    <Flex vertical=true gap=FlexGap::Small>
                                                                        <Body1>{principal_label_text}</Body1>
                                                                        <Caption1>{format!("{principal_kind_label} ({principal_id_text})")}</Caption1>
                                                                    </Flex>
                                                                    <Menu
                                                                        on_select={
                                                                            let principal_label = principal_label_menu;
                                                                            move |action: &str| {
                                                                                if action == "remove" {
                                                                                    pending_remove.set(Some((
                                                                                        principal_id_action.clone(),
                                                                                        is_group,
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
                                                                || "Remove selected principal?".to_string(),
                                                                |(_, _, label)| format!("Remove '{label}' from this permission allow list?"),
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
                                                            let pid = permission_id.get();
                                                            if let Some((principal_id, is_group, _)) = pending {
                                                                spawn_local_scoped(async move {
                                                                    let result = if is_group {
                                                                        remove_permission_group(pid, principal_id).await
                                                                    } else {
                                                                        remove_permission_user(pid, principal_id).await
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
                                    <Dialog open=request_dialog_open>
                                        <DialogSurface>
                                            <DialogBody>
                                                <DialogTitle>"Request Access"</DialogTitle>
                                                <DialogContent>
                                                    <Flex vertical=true gap=FlexGap::Small>
                                                        <Field label="Reason">
                                                            <Textarea bind=request_reason />
                                                        </Field>
                                                        <Show when=move || request_error.get().is_some()>
                                                            <MessageBar intent=MessageBarIntent::Error>
                                                                {move || request_error.get().unwrap_or_default()}
                                                            </MessageBar>
                                                        </Show>
                                                    </Flex>
                                                </DialogContent>
                                                <DialogActions>
                                                    <Button
                                                        appearance=ButtonAppearance::Secondary
                                                        on_click=Callback::new(move |_| {
                                                            request_dialog_open.set(false);
                                                            request_error.set(None);
                                                        })
                                                    >
                                                        "Cancel"
                                                    </Button>
                                                    <Button
                                                        appearance=ButtonAppearance::Primary
                                                        on_click=Callback::new(move |_| {
                                                            let reason = request_reason.get();
                                                            if reason.trim().is_empty() {
                                                                request_error.set(Some("Reason is required.".to_string()));
                                                                return;
                                                            }
                                                            let input = PermissionRequestCreateInput {
                                                                target_kind: PermissionRequestTargetKind::Permission,
                                                                target_id: permission_id.get(),
                                                                reason,
                                                            };
                                                            spawn_local_scoped(async move {
                                                                match create_permission_request(input).await {
                                                                    Ok(_) => {
                                                                        request_dialog_open.set(false);
                                                                        request_error.set(None);
                                                                        request_reason.set(String::new());
                                                                        refresh.update(|n| *n += 1);
                                                                    }
                                                                    Err(err) => request_error.set(Some(err.to_string())),
                                                                }
                                                            });
                                                        })
                                                    >
                                                        "Submit Request"
                                                    </Button>
                                                </DialogActions>
                                            </DialogBody>
                                        </DialogSurface>
                                    </Dialog>
                                </Flex>
                            }.into_any()
                        }
                        Ok(None) => view! {
                            <EmptyState message="Permission not found." />
                        }.into_any(),
                        Err(err) => view! {
                            <MessageBar intent=MessageBarIntent::Error>
                                {format!("Failed to load permission: {err}")}
                            </MessageBar>
                        }.into_any(),
                    })
                }}
            </Transition>
        </ContentContainer>
    }
}
