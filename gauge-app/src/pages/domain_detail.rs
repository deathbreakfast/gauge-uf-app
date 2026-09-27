use gauge::search_sources::PermissionSearchSourceId;
use leptos::prelude::*;
use leptos::task::spawn_local_scoped;
use leptos_router::hooks::{use_navigate, use_params_map};
use leptos_router::NavigateOptions;
use uf_integrations::SearchSourcePicker;
use uf_product::components::{Body1, Caption1, Card, EmptyState, SkeletonItemSize};
use uf_product::components::{ContentContainer, SpacingSize, Title3};
use uf_product::primitives::{
    Button, ButtonAppearance, Dialog, DialogActions, DialogBody, DialogContent, DialogSurface,
    DialogTitle, Field, Flex, FlexAlign, FlexGap, FlexJustify, Input, Menu, MenuItem, MenuTrigger,
    MessageBar, MessageBarIntent, SkeletonItem, Textarea,
};
use uf_search_core::{SearchSourceItem, SearchSourceKey};

use crate::pages::step_up::spawn_with_step_up;
use crate::server::{
    add_domain_owner_user, delete_domain, get_domain, remove_domain_owner_user, update_domain,
    UpdateDomainInput,
};

/// Domain detail/edit page: name/description, owners picker, and delete.
#[component]
#[allow(clippy::too_many_lines)]
pub fn DomainDetailPage() -> impl IntoView {
    let navigate = use_navigate();
    let navigate_store = StoredValue::new(navigate.clone());
    let params = use_params_map();
    let domain_id = Memo::new(move |_| params.read().get("id").unwrap_or_default());
    let refresh = RwSignal::new(0u64);

    let detail = Resource::new(
        move || (domain_id.get(), refresh.get()),
        move |(id, _)| async move { get_domain(id).await },
    );

    let name = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let owner_picker_options = RwSignal::new(Vec::<SearchSourceItem>::new());
    let owner_picker_request_seq = RwSignal::new(0u64);
    let error = RwSignal::new(None::<String>);
    let owner_picker_error = RwSignal::new(None::<String>);
    let confirm_remove_owner_open = RwSignal::new(false);
    let pending_remove_owner = RwSignal::new(None::<(String, String)>);
    let confirm_delete_open = RwSignal::new(false);

    Effect::new(move |_| {
        if let Some(Ok(Some(row))) = detail.get() {
            name.set(row.name);
            description.set(row.description);
        }
    });

    let save = move |_| {
        let payload = UpdateDomainInput {
            id: domain_id.get(),
            name: name.get(),
            description: description.get(),
        };
        spawn_local_scoped(async move {
            match update_domain(payload).await {
                Ok(()) => refresh.update(|n| *n += 1),
                Err(err) => error.set(Some(err.to_string())),
            }
        });
    };

    let owner_request_initial = Callback::new(move |sources: Vec<SearchSourceKey>| {
        let request_id = owner_picker_request_seq.get_untracked().saturating_add(1);
        owner_picker_request_seq.set(request_id);
        spawn_local_scoped(async move {
            match crate::server::search_principals(sources, None, 10).await {
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
                match crate::server::search_principals(sources, Some(query), 10).await {
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
        let id = domain_id.get_untracked();
        let finish_ok = Callback::new(move |_: ()| {
            refresh.update(|n| *n += 1);
            owner_picker_error.set(None);
        });
        spawn_with_step_up(owner_picker_error, finish_ok, move || {
            let id = id.clone();
            let user_id = item.id.clone();
            async move { add_domain_owner_user(id, user_id).await }
        });
    });

    view! {
        <div id="gauge-domain-detail-page">
        <ContentContainer max_width="900px">
            <Suspense fallback=move || view! {
                <Card>
                    <Flex vertical=true gap=FlexGap::Small padding=SpacingSize::Size200.inset()>
                        {(0..3).map(|_| view! {
                            <SkeletonItem
                                size=Signal::from(SkeletonItemSize::S32)
                                width="100%".to_string()
                            />
                        }).collect_view()}
                    </Flex>
                </Card>
            }>
                {move || {
                    detail.get().map(|result| match result {
                        Ok(Some(row)) => {
                            let owner_users = RwSignal::new(row.owner_users.clone());
                            let resource_scoped = row.resource_scoped;
                            view! {
                                <Flex vertical=true gap=FlexGap::Medium>
                                    <div id="gauge-domain-detail-header">
                                    <Card>
                                        <Flex vertical=true gap=FlexGap::Small padding=SpacingSize::Size200.inset()>
                                            <Title3>"Domain Detail"</Title3>
                                            <Caption1>{format!("ID: {}", row.id.clone())}</Caption1>
                                            <Show when=move || resource_scoped>
                                                <MessageBar intent=MessageBarIntent::Info>
                                                    "This domain is resource-scoped plumbing. Rename, delete, and owner changes are blocked."
                                                </MessageBar>
                                            </Show>
                                        </Flex>
                                    </Card>
                                    </div>

                                    <Show when=move || !resource_scoped>
                                        <Card>
                                            <Flex vertical=true gap=FlexGap::Medium padding=SpacingSize::Size200.inset()>
                                                <Flex vertical=true gap=FlexGap::Small>
                                                    <Title3>"Owners"</Title3>
                                                    <Caption1>"Owners can rename this domain, manage owners, and delete it."</Caption1>
                                                </Flex>
                                                <div id="gauge-domain-owners-picker">
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
                                                <div id="gauge-domain-owner-remove">
                                                <Show when=move || !owner_users.get().is_empty() fallback=move || view! {
                                                    <EmptyState message="No owners are assigned to this domain." />
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
                                                                                attr:aria-label="Open owner actions"
                                                                            >
                                                                                "…"
                                                                            </Button>
                                                                        </MenuTrigger>
                                                                            <MenuItem value="remove_owner">"Remove Owner"</MenuItem>
                                                                        </Menu>
                                                                    }
                                                                }
                                                                </Flex>
                                                            </>
                                                        </For>
                                                    </Flex>
                                                </Show>
                                                </div>
                                            </Flex>
                                        </Card>
                                    </Show>

                                    <Card>
                                        <div id="gauge-domain-detail-form">
                                        <Flex vertical=true gap=FlexGap::Medium padding=SpacingSize::Size200.inset()>
                                            <Field label="Display name">
                                                <Input bind=name />
                                            </Field>
                                            <Field label="Description">
                                                <Textarea bind=description />
                                            </Field>
                                            <Show when=move || error.get().is_some()>
                                                <MessageBar intent=MessageBarIntent::Error>
                                                    {move || error.get().unwrap_or_default()}
                                                </MessageBar>
                                            </Show>
                                            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center gap=FlexGap::Small>
                                                <Show when=move || !resource_scoped>
                                                    <div id="gauge-domain-delete">
                                                        <Button
                                                            appearance=ButtonAppearance::Secondary
                                                            on_click=Callback::new(move |_| confirm_delete_open.set(true))
                                                        >
                                                            "Delete Domain"
                                                        </Button>
                                                    </div>
                                                </Show>
                                                <Show when=move || !resource_scoped>
                                                    <Button appearance=ButtonAppearance::Primary on_click=Callback::new(save)>
                                                        "Save Changes"
                                                    </Button>
                                                </Show>
                                            </Flex>
                                        </Flex>
                                        </div>
                                    </Card>
                                </Flex>
                            }.into_any()
                        }
                        Ok(None) => view! {
                            <Card>
                                <Flex vertical=true gap=FlexGap::Small padding=SpacingSize::Size200.inset()>
                                    <Title3>"Domain not found"</Title3>
                                </Flex>
                            </Card>
                        }.into_any(),
                        Err(err) => view! {
                            <MessageBar intent=MessageBarIntent::Error>{err.to_string()}</MessageBar>
                        }.into_any(),
                    })
                }}
            </Suspense>
        </ContentContainer>

        <Dialog open=confirm_remove_owner_open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>"Remove owner?"</DialogTitle>
                    <DialogContent>
                        {move || pending_remove_owner.get().map(|(_, label)| label).unwrap_or_default()}
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
                                let Some((owner_user_id, _)) = pending_remove_owner.get() else {
                                    return;
                                };
                                let id = domain_id.get_untracked();
                                confirm_remove_owner_open.set(false);
                                pending_remove_owner.set(None);
                                let finish_ok = Callback::new(move |_: ()| {
                                    refresh.update(|n| *n += 1);
                                    error.set(None);
                                });
                                spawn_with_step_up(error, finish_ok, move || {
                                    let id = id.clone();
                                    let owner_user_id = owner_user_id.clone();
                                    async move { remove_domain_owner_user(id, owner_user_id).await }
                                });
                            })
                        >
                            "Remove"
                        </Button>
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
        </Dialog>

        <Dialog open=confirm_delete_open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>"Delete domain?"</DialogTitle>
                    <DialogContent>"This removes the taxonomy domain. Permissions that still reference it may block delete."</DialogContent>
                    <DialogActions>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            on_click=Callback::new(move |_| confirm_delete_open.set(false))
                        >
                            "Cancel"
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click=Callback::new(move |_| {
                                let id = domain_id.get_untracked();
                                confirm_delete_open.set(false);
                                let finish_ok = Callback::new(move |_: ()| {
                                    navigate_store.with_value(|nav| {
                                        nav("/permission/create-domain", NavigateOptions::default());
                                    });
                                });
                                spawn_with_step_up(error, finish_ok, move || {
                                    let id = id.clone();
                                    async move { delete_domain(id).await }
                                });
                            })
                        >
                            "Delete"
                        </Button>
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
        </Dialog>
        </div>
    }
}
