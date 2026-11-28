use lepton_shell::people::{PersonPicker, PickedPrincipal};
use leptos::prelude::*;
use leptos::task::spawn_local_scoped;
use leptos_router::hooks::{use_navigate, use_params_map};
use leptos_router::NavigateOptions;
use uf_product::components::EmptyStateCallToAction;
use uf_product::components::{Body1, Caption1, Card, EmptyState, SkeletonItemSize};
use uf_product::components::{ContentContainer, SpacingSize, Title3};
use uf_product::primitives::{
    Breadcrumb, BreadcrumbButton, BreadcrumbDivider, BreadcrumbItem, Button, ButtonAppearance,
    Dialog, DialogActions, DialogBody, DialogContent, DialogSurface, DialogTitle, Field, Flex,
    FlexAlign, FlexGap, FlexJustify, Input, MarkdownEditor, Menu, MenuItem, MenuTrigger,
    MessageBar, MessageBarIntent, SkeletonItem, Table, TableBody, TableCell, TableHeader,
    TableHeaderCell, TableRow,
};

use crate::components::error_bar::ErrorBar;
use crate::pages::principal_pick::{act_on_pick, to_picked, PickTarget};
use crate::pages::shared::resource_cards::ResourceBundleCard;
use crate::pages::step_up::spawn_with_step_up;
use crate::server::surfaces::{domain_display_name, list_domain_permissions};
use crate::server::{
    add_domain_owner_user, delete_domain, get_domain, remove_domain_owner_user, update_domain,
    UpdateDomainInput,
};

/// Domains › {current}.
#[component]
fn DomainCrumbs(#[prop(into)] current: String) -> impl IntoView {
    view! {
        <div data-testid="gauge-domain-crumbs">
            <Breadcrumb>
                <BreadcrumbItem>
                    <BreadcrumbButton href="/permission/domains".to_string()>"Domains"</BreadcrumbButton>
                </BreadcrumbItem>
                <BreadcrumbDivider />
                <BreadcrumbItem>
                    <BreadcrumbButton current=true>{current}</BreadcrumbButton>
                </BreadcrumbItem>
            </Breadcrumb>
        </div>
    }
}

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
    let owner_pick = RwSignal::new(Vec::<PickedPrincipal>::new());
    let error = RwSignal::new(None::<String>);
    let owner_picker_error = RwSignal::new(None::<String>);
    let confirm_remove_owner_open = RwSignal::new(false);
    let pending_remove_owner = RwSignal::new(None::<(String, String)>);
    let confirm_delete_open = RwSignal::new(false);
    // The permissions still in the domain, read when Delete opens; any left block the delete.
    let blocking = crate::pages::shared::local_resource(
        move || confirm_delete_open.get().then(|| domain_id.get()),
        |id| async move {
            match id {
                Some(id) => list_domain_permissions(id).await,
                None => Ok(Vec::new()),
            }
        },
    );
    let shown_name = Memo::new(move |_| {
        detail
            .get()
            .and_then(Result::ok)
            .flatten()
            .map(|d| domain_display_name(&d.name))
            .unwrap_or_default()
    });

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

    let on_owner_pick = act_on_pick(owner_pick, move |target| {
        // The owners picker searches people only.
        let PickTarget::User(user_id) = target else {
            return;
        };
        let id = domain_id.get_untracked();
        let finish_ok = Callback::new(move |_: ()| {
            refresh.update(|n| *n += 1);
            owner_picker_error.set(None);
        });
        spawn_with_step_up(owner_picker_error, finish_ok, move || {
            let id = id.clone();
            let user_id = user_id.clone();
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
                            let title = domain_display_name(&row.name);
                            view! {
                                <Flex vertical=true gap=FlexGap::Medium>
                                    <DomainCrumbs current=title.clone() />
                                    <div id="gauge-domain-detail-header">
                                    <Card>
                                        <Flex vertical=true gap=FlexGap::Small padding=SpacingSize::Size200.inset()>
                                            <Title3>{title}</Title3>
                                            <Show when=move || resource_scoped>
                                                <MessageBar intent=MessageBarIntent::Info>
                                                    "This domain is resource-scoped plumbing. Rename, delete, and owner changes are blocked."
                                                </MessageBar>
                                            </Show>
                                        </Flex>
                                    </Card>
                                    </div>

                                    <Show when=move || resource_scoped>
                                        <ResourceBundleCard domain_id=domain_id />
                                    </Show>

                                    <Show when=move || !resource_scoped>
                                        <Card>
                                            <Flex vertical=true gap=FlexGap::Medium padding=SpacingSize::Size200.inset()>
                                                <Flex vertical=true gap=FlexGap::Small>
                                                    <Title3>"Owners"</Title3>
                                                    <Caption1>"Owners can rename this domain, manage owners, and delete it."</Caption1>
                                                </Flex>
                                                <div id="gauge-domain-owners-picker">
                                                <PersonPicker
                                                    value=owner_pick
                                                    single=true
                                                    exclude=Signal::derive(move || {
                                                        owner_users.get().iter().map(to_picked).collect::<Vec<_>>()
                                                    })
                                                    label="Add an owner"
                                                    placeholder="Search people"
                                                    on_change=on_owner_pick
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
                                                <MarkdownEditor value=description aria_label="Description" />
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
                            <div data-testid="gauge-domain-not-found">
                                <Flex vertical=true gap=FlexGap::Medium>
                                    <DomainCrumbs current="Not found" />
                                    <EmptyState
                                        message="Domain not found"
                                        description="It may have been deleted, or the link has a typo."
                                    >
                                        <EmptyStateCallToAction slot:call_to_action>
                                            <Button on_click=Callback::new(move |_| navigate_store.with_value(|nav| nav("/permission/domains", NavigateOptions::default())))>
                                                "Back to Domains"
                                            </Button>
                                        </EmptyStateCallToAction>
                                    </EmptyState>
                                </Flex>
                            </div>
                        }.into_any(),
                        Err(err) => view! {
                            <Flex vertical=true gap=FlexGap::Medium>
                                <DomainCrumbs current="Domain" />
                                <ErrorBar error=err.to_string() what="this domain" on_retry=Callback::new(move |()| refresh.update(|n| *n += 1)) />
                            </Flex>
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
                    {move || match blocking.get() {
                        Some(Ok(perms)) if !perms.is_empty() => {
                            let name = shown_name.get();
                            let n = perms.len();
                            let count = if n == 1 { "1 permission".to_string() } else { format!("{n} permissions") };
                            view! {
                                <DialogTitle>{format!("{name} can't be deleted yet")}</DialogTitle>
                                <DialogContent>
                                    <div data-testid="gauge-domain-delete-blocked">
                                        <Flex vertical=true gap=FlexGap::Medium>
                                            <Body1>{format!("{name} still has {count}. Move or delete them first.")}</Body1>
                                            <Table>
                                                <TableHeader>
                                                    <TableRow>
                                                        <TableHeaderCell>"Permission"</TableHeaderCell>
                                                        <TableHeaderCell>"Key"</TableHeaderCell>
                                                    </TableRow>
                                                </TableHeader>
                                                <TableBody>
                                                    {perms.into_iter().map(|p| view! {
                                                        <TableRow>
                                                            <TableCell>{p.display_name}</TableCell>
                                                            <TableCell>{p.key}</TableCell>
                                                        </TableRow>
                                                    }).collect_view()}
                                                </TableBody>
                                            </Table>
                                        </Flex>
                                    </div>
                                </DialogContent>
                                <DialogActions>
                                    <Button appearance=ButtonAppearance::Primary on_click=Callback::new(move |_| confirm_delete_open.set(false))>
                                        "Close"
                                    </Button>
                                </DialogActions>
                            }.into_any()
                        }
                        Some(_) => view! {
                            <DialogTitle>{move || format!("Delete {}?", shown_name.get())}</DialogTitle>
                            <DialogContent>"It holds no permissions, so nothing else changes."</DialogContent>
                            <DialogActions>
                                <Button
                                    appearance=ButtonAppearance::Secondary
                                    on_click=Callback::new(move |_| confirm_delete_open.set(false))
                                >
                                    "Cancel"
                                </Button>
                                <div id="gauge-domain-delete-confirm">
                                <Button
                                    appearance=ButtonAppearance::Primary
                                    on_click=Callback::new(move |_| {
                                        let id = domain_id.get_untracked();
                                        confirm_delete_open.set(false);
                                        let finish_ok = Callback::new(move |_: ()| {
                                            navigate_store.with_value(|nav| {
                                                nav("/permission/domains", NavigateOptions::default());
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
                                </div>
                            </DialogActions>
                        }.into_any(),
                        None => view! {
                            <DialogTitle>"Checking the domain"</DialogTitle>
                            <DialogContent><SkeletonItem width="100%".to_string() /></DialogContent>
                        }.into_any(),
                    }}
                </DialogBody>
            </DialogSurface>
        </Dialog>
        </div>
    }
}
