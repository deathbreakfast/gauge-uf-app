//! One person's page: what they can do and why, their groups, what they own, their open
//! requests and recent changes about them.
//!
//! Gauge admins open anyone's page; everyone opens their own at `/permission/people/me`.
//! The page server-renders the person and their access; Recent changes loads after hydration.

use gauge::types::{RecordKind, RecordRef};
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};
use leptos_router::NavigateOptions;
use uf_product::components::{
    Body1, Body1Strong, Caption1, Card, CardContent, CardHeader, CardHeaderAction,
    CardHeaderDescription, ContentContainer, EmptyState, EmptyStateCallToAction, Persona,
    PersonaConfig, PersonaSecondaryText,
};
use uf_product::primitives::{
    use_breakpoint_down, Badge, BadgeAppearance, BadgeColor, BadgeSize, Breadcrumb,
    BreadcrumbButton, BreadcrumbDivider, BreadcrumbItem, Breakpoint, Button, ButtonAppearance,
    Code, Flex, FlexAlign, FlexGap, FlexJustify, FlexWrap, Grid, GridConfig, GridItem, Link, Menu,
    MenuItem, MenuTrigger, MessageBar, MessageBarBody, MessageBarIntent, SkeletonItem, Table,
    TableBody, TableCell, TableHeader, TableHeaderCell, TableRow,
};

use crate::components::error_bar::{ErrorBar, MaybeErrorBar};
use crate::pages::group_detail::{remove_member, spawn_member_change};
use crate::pages::shared::give_access::GiveAccessToPersonDialog;
use crate::pages::shared::local_resource;
use crate::pages::shared::remove_confirm::{
    remove_action, RemoveConfirmDialog, RemoveFrom, RemoveTarget,
};
use crate::pages::shared::reviewers::{reviewers_for, reviewers_of};
use crate::pages::shared::words::{
    date_view, first_name, path_text, plain_name, status_words, when_view,
};
use crate::server::surfaces::get_activity_window;
use crate::server::surfaces::{get_person_page, PersonPage};
use crate::server::viewer::{use_viewer_access, viewer_now};

/// How many recent changes the page lists.
const RECENT_CHANGES: usize = 5;

/// Where a record a person owns opens.
#[must_use]
pub fn record_href(record: &RecordRef) -> String {
    match record.kind {
        RecordKind::Permission => format!("/permission/permissions/{}", record.id),
        RecordKind::Group => format!("/permission/groups/{}", record.id),
        RecordKind::Domain => format!("/permission/domains/{}", record.id),
        RecordKind::Request => format!("/permission/requests/{}", record.id),
    }
}

/// "Added by Ines Okoro", or a dash when the history doesn't say.
#[must_use]
pub fn joined_text(added_by: Option<&str>) -> String {
    added_by.map_or_else(
        || "—".to_string(),
        |by| format!("Added by {}", plain_name(by)),
    )
}

/// The person page.
#[component]
pub fn PersonDetailPage() -> impl IntoView {
    let params = use_params_map();
    let id = Memo::new(move |_| params.read().get("id").unwrap_or_default());
    let refresh = RwSignal::new(0u32);
    let page = Resource::new(
        move || (id.get(), refresh.get()),
        |(id, _)| get_person_page(id),
    );
    let viewer = use_viewer_access();
    let is_admin = Signal::derive(move || viewer_now(viewer).gauge_admin);

    view! {
        <ContentContainer max_width="1100px" data_testid="gauge-person-detail">
            <Transition fallback=|| view! { <PersonSkeleton /> }>
                {move || page.get().map(|result| match result {
                    Ok(Some(p)) => view! { <PersonView page=p refresh=refresh is_admin=is_admin /> }.into_any(),
                    Ok(None) => view! { <PersonNotFound is_admin=is_admin /> }.into_any(),
                    Err(err) => view! {
                        <Flex vertical=true gap=FlexGap::Medium>
                            <PeopleCrumbs current="Person".to_string() is_admin=is_admin />
                            <ErrorBar error=err.to_string() what="their access" on_retry=Callback::new(move |()| refresh.update(|n| *n += 1)) />
                        </Flex>
                    }.into_any(),
                })}
            </Transition>
        </ContentContainer>
    }
}

/// People › {current}. Someone who can't open People gets Overview as the first crumb.
#[component]
fn PeopleCrumbs(current: String, is_admin: Signal<bool>) -> impl IntoView {
    view! {
        <Breadcrumb>
            <BreadcrumbItem>
                {move || if is_admin.get() {
                    view! { <BreadcrumbButton href="/permission/people".to_string()>"People"</BreadcrumbButton> }.into_any()
                } else {
                    view! { <BreadcrumbButton href="/permission".to_string()>"Overview"</BreadcrumbButton> }.into_any()
                }}
            </BreadcrumbItem>
            <BreadcrumbDivider />
            <BreadcrumbItem>
                <BreadcrumbButton current=true>{current}</BreadcrumbButton>
            </BreadcrumbItem>
        </Breadcrumb>
    }
}

#[component]
fn PersonSkeleton() -> impl IntoView {
    view! {
        <div data-testid="gauge-person-skeleton">
            <Flex vertical=true gap=FlexGap::Medium>
                <SkeletonItem width="240px".to_string() />
                <Card>
                    <CardHeader>"What they can do"</CardHeader>
                    <CardContent>
                        <Flex vertical=true gap=FlexGap::Small>
                            {(0..3).map(|_| view! { <SkeletonItem width="100%".to_string() /> }).collect_view()}
                        </Flex>
                    </CardContent>
                </Card>
                <Card>
                    <CardHeader>"Groups"</CardHeader>
                    <CardContent>
                        <Flex vertical=true gap=FlexGap::Small>
                            {(0..2).map(|_| view! { <SkeletonItem width="100%".to_string() /> }).collect_view()}
                        </Flex>
                    </CardContent>
                </Card>
            </Flex>
        </div>
    }
}

#[component]
fn PersonNotFound(is_admin: Signal<bool>) -> impl IntoView {
    let navigate = use_navigate();
    view! {
        <div data-testid="gauge-person-not-found">
            <Flex vertical=true gap=FlexGap::Medium>
                <PeopleCrumbs current="Not found".to_string() is_admin=is_admin />
                <EmptyState
                    message="Person not found"
                    description="It may have been deleted, or the link has a typo."
                >
                    <EmptyStateCallToAction slot:call_to_action>
                        <Button on_click=Callback::new(move |_| {
                            let to = if is_admin.get_untracked() { "/permission/people" } else { "/permission" };
                            navigate(to, NavigateOptions::default());
                        })>
                            {move || if is_admin.get() { "Back to People" } else { "Back to Overview" }}
                        </Button>
                    </EmptyStateCallToAction>
                </EmptyState>
            </Flex>
        </div>
    }
}

#[component]
fn PersonView(page: PersonPage, refresh: RwSignal<u32>, is_admin: Signal<bool>) -> impl IntoView {
    let viewer = use_viewer_access();
    let name = plain_name(&page.principal.label);
    let first = first_name(&name);
    let recent_name = StoredValue::new(name.clone());
    let principal = StoredValue::new(page.principal.clone());
    let give_open = RwSignal::new(false);
    let notice = RwSignal::new(None::<String>);
    let error = RwSignal::new(None::<String>);
    let remove_open = RwSignal::new(false);
    let remove_target = RwSignal::new(None::<RemoveTarget>);
    let is_narrow = use_breakpoint_down(Breakpoint::Md);
    let access = page.access;
    let n_perms = access.permissions.len();
    let n_groups = access.groups.len();

    let on_remove = Callback::new(move |()| {
        let Some(target) = remove_target.get() else {
            return;
        };
        remove_open.set(false);
        let RemoveFrom::Group {
            id: gid,
            name: gname,
        } = target.from.clone()
        else {
            return;
        };
        let confirm = remove_action(&target);
        let who = plain_name(&target.principal.label);
        let kind = target.principal.kind.clone();
        let pid = target.principal.id.clone();
        let done = Callback::new(move |()| {
            notice.set(Some(format!("{who} removed from {gname}")));
            refresh.update(|n| *n += 1);
        });
        spawn_member_change(
            &gid.clone(),
            &kind.clone(),
            confirm,
            error,
            done,
            move |code| remove_member(kind.clone(), gid.clone(), pid.clone(), code),
        );
    });

    let open_requests = StoredValue::new(access.open_requests.clone());
    let reviewers = reviewers_for(Signal::derive(move || open_requests.get_value()));

    view! {
        <Flex vertical=true gap=FlexGap::Large>
            <PeopleCrumbs current=name.clone() is_admin=is_admin />
            <div id="gauge-person-header">
                <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center gap=FlexGap::Medium wrap=FlexWrap::Wrap>
                    <Persona config=PersonaConfig::named(name.clone())>
                        <PersonaSecondaryText slot>"Person"</PersonaSecondaryText>
                    </Persona>
                    <Show when=move || is_admin.get()>
                        <div id="gauge-person-give">
                            <Button appearance=ButtonAppearance::Primary on_click=Callback::new(move |_| give_open.set(true))>
                                "Give access"
                            </Button>
                        </div>
                    </Show>
                </Flex>
            </div>
            <Show when=move || notice.get().is_some()>
                <div data-testid="gauge-person-notice">
                    <MessageBar intent=MessageBarIntent::Success>
                        <MessageBarBody>{move || notice.get().unwrap_or_default()}</MessageBarBody>
                    </MessageBar>
                </div>
            </Show>
            <MaybeErrorBar error=error what="their groups" />
            <div id="gauge-person-can-do">
                <Card>
                    <CardHeader>
                        {format!("What {first} can do ({n_perms})")}
                        <CardHeaderDescription slot>"Every permission check they pass today."</CardHeaderDescription>
                    </CardHeader>
                    <CardContent>
                        {if access.permissions.is_empty() {
                            view! { <EmptyState message="No permissions yet" description="Add them to a group, or give them a permission." /> }.into_any()
                        } else {
                            view! {
                                <Table>
                                    <TableHeader>
                                        <TableRow>
                                            <TableHeaderCell>"Permission"</TableHeaderCell>
                                            <TableHeaderCell>"Why they have it"</TableHeaderCell>
                                        </TableRow>
                                    </TableHeader>
                                    <TableBody>
                                        {access.permissions.clone().into_iter().map(|(perm, paths)| {
                                            let href = format!("/permission/permissions/{}", perm.id);
                                            let why = paths.iter().map(path_text).collect::<Vec<_>>().join("; ");
                                            view! {
                                                <TableRow attr:data-testid="gauge-person-permission-row">
                                                    <TableCell>
                                                        <Flex vertical=true gap=FlexGap::Size(0)>
                                                            <Link href=href><Body1Strong>{perm.display_name.clone()}</Body1Strong></Link>
                                                            <Caption1><Code text=perm.key.clone() /></Caption1>
                                                        </Flex>
                                                    </TableCell>
                                                    <TableCell>{why}</TableCell>
                                                </TableRow>
                                            }
                                        }).collect_view()}
                                    </TableBody>
                                </Table>
                            }.into_any()
                        }}
                    </CardContent>
                </Card>
            </div>
            <div id="gauge-person-groups">
                <Card>
                    <CardHeader>{format!("Groups ({n_groups})")}</CardHeader>
                    <CardContent>
                        {if access.groups.is_empty() {
                            view! { <EmptyState message="Not in any groups yet" /> }.into_any()
                        } else {
                            view! {
                                <Table>
                                    <TableHeader>
                                        <TableRow>
                                            <TableHeaderCell>"Group"</TableHeaderCell>
                                            <TableHeaderCell>"How they joined"</TableHeaderCell>
                                            <TableHeaderCell>"Since"</TableHeaderCell>
                                            <TableHeaderCell>""</TableHeaderCell>
                                        </TableRow>
                                    </TableHeader>
                                    <TableBody>
                                        {access.groups.clone().into_iter().map(|m| {
                                            let gid = m.group.id.clone();
                                            let gname = m.group.name.clone();
                                            let href = format!("/permission/groups/{gid}");
                                            let can_remove = {
                                                let gid = gid.clone();
                                                Signal::derive(move || viewer_now(viewer).can_manage_group(&gid))
                                            };
                                            let aria = format!("More actions for {gname}");
                                            let gid_menu = gid.clone();
                                            let gname_menu = gname.clone();
                                            let joined = joined_text(m.added_by_label.as_deref());
                                            let since = m.added_at.map(|at| date_view(at.to_rfc3339()));
                                            view! {
                                                <TableRow attr:data-testid="gauge-person-group-row">
                                                    <TableCell><Link href=href>{gname.clone()}</Link></TableCell>
                                                    <TableCell>{joined}</TableCell>
                                                    <TableCell>{since}</TableCell>
                                                    <TableCell>
                                                        <Show when=move || can_remove.get()>
                                                            {
                                                                let gid = gid_menu.clone();
                                                                let gname = gname_menu.clone();
                                                                let aria = aria.clone();
                                                                view! {
                                                                    <Menu on_select=move |action: &str| if action == "remove" {
                                                                        remove_target.set(Some(RemoveTarget {
                                                                            principal: principal.get_value(),
                                                                            from: RemoveFrom::Group { id: gid.clone(), name: gname.clone() },
                                                                        }));
                                                                        remove_open.set(true);
                                                                    }>
                                                                        <MenuTrigger slot>
                                                                            <Button appearance=ButtonAppearance::Subtle icon=icondata::AiMoreOutlined attr:aria-label=aria.clone() />
                                                                        </MenuTrigger>
                                                                        <MenuItem value="remove">"Remove"</MenuItem>
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
                            }.into_any()
                        }}
                    </CardContent>
                </Card>
            </div>
            // Owns and Requests side by side; one column on narrow screens.
            {move || {
                let cols = if is_narrow.get() { 1 } else { 2 };
                let owns = access.owns.clone();
                let requests = open_requests.get_value();
                view! {
                    <div data-testid="gauge-person-side-by-side">
                        <Grid config=GridConfig::with_gaps(cols, 16, 16)>
                            <GridItem>
                                <div data-testid="gauge-person-owns">
                                    <Card>
                                        <CardHeader>"Owns"</CardHeader>
                                        <CardContent>
                                            {if owns.is_empty() {
                                                view! { <Body1>"Nothing. Owners review requests for a group or permission."</Body1> }.into_any()
                                            } else {
                                                view! {
                                                    <Flex vertical=true gap=FlexGap::Small>
                                                        {owns.into_iter().map(|r| view! {
                                                            <Link href=record_href(&r)>{r.label.clone()}</Link>
                                                        }).collect_view()}
                                                    </Flex>
                                                }.into_any()
                                            }}
                                        </CardContent>
                                    </Card>
                                </div>
                            </GridItem>
                            <GridItem>
                                <div data-testid="gauge-person-requests">
                                    <Card>
                                        <CardHeader>"Requests"</CardHeader>
                                        <CardContent>
                                            {if requests.is_empty() {
                                                view! { <Body1>"No open requests."</Body1> }.into_any()
                                            } else {
                                                view! {
                                                    <Table>
                                                        <TableHeader>
                                                            <TableRow>
                                                                <TableHeaderCell>"Asking for"</TableHeaderCell>
                                                                <TableHeaderCell>"Asked"</TableHeaderCell>
                                                                <TableHeaderCell>"Status"</TableHeaderCell>
                                                            </TableRow>
                                                        </TableHeader>
                                                        <TableBody>
                                                            {requests.into_iter().map(|row| {
                                                                let href = format!("/permission/requests/{}", row.id);
                                                                let status_row = row.clone();
                                                                view! {
                                                                    <TableRow>
                                                                        <TableCell><Link href=href>{row.target_label.clone()}</Link></TableCell>
                                                                        <TableCell>{when_view(row.created_at.clone())}</TableCell>
                                                                        <TableCell>
                                                                            <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Warning size=BadgeSize::Small>
                                                                                {move || status_words(&status_row.status, reviewers_of(reviewers, &status_row).as_deref())}
                                                                            </Badge>
                                                                        </TableCell>
                                                                    </TableRow>
                                                                }
                                                            }).collect_view()}
                                                        </TableBody>
                                                    </Table>
                                                }.into_any()
                                            }}
                                        </CardContent>
                                    </Card>
                                </div>
                            </GridItem>
                        </Grid>
                    </div>
                }
            }}
            <Show when=move || is_admin.get()>
                <PersonRecentChanges name=recent_name.get_value() />
            </Show>
            <GiveAccessToPersonDialog
                open=give_open
                principal=Signal::derive(move || principal.get_value())
                on_done=Callback::new(move |text: String| {
                    notice.set(Some(text));
                    refresh.update(|n| *n += 1);
                })
            />
            <RemoveConfirmDialog open=remove_open target=remove_target on_confirm=on_remove />
        </Flex>
    }
}

/// Changes that name the person, newest first, with a link to Activity searched for them.
#[component]
fn PersonRecentChanges(name: String) -> impl IntoView {
    let query = name.clone();
    let rows = local_resource(
        move || query.clone(),
        |q| async move {
            get_activity_window(String::new(), None, q, None)
                .await
                .map(|page| {
                    page.items
                        .into_iter()
                        .take(RECENT_CHANGES)
                        .collect::<Vec<_>>()
                })
        },
    );
    let href = format!("/permission/activity?q={}", js_encode(&name));
    view! {
        <div id="gauge-person-recent">
            <Card>
                <CardHeader>
                    "Recent changes"
                    <CardHeaderAction slot>
                        <Link href=href>"All activity"</Link>
                    </CardHeaderAction>
                </CardHeader>
                <CardContent>
                    <Suspense fallback=|| view! { <SkeletonItem width="100%".to_string() /> }>
                        {move || rows.get().map(|r| match r {
                            Ok(list) if list.is_empty() => view! { <Body1>"No changes yet."</Body1> }.into_any(),
                            Ok(list) => view! {
                                <Table>
                                    <TableHeader>
                                        <TableRow>
                                            <TableHeaderCell>"When"</TableHeaderCell>
                                            <TableHeaderCell>"Who"</TableHeaderCell>
                                            <TableHeaderCell>"What changed"</TableHeaderCell>
                                        </TableRow>
                                    </TableHeader>
                                    <TableBody>
                                        {list.into_iter().map(|row| view! {
                                            <TableRow>
                                                <TableCell>{when_view(row.at.to_rfc3339())}</TableCell>
                                                <TableCell>{plain_name(&row.actor_label)}</TableCell>
                                                <TableCell>{format!("{} · {}", row.summary, row.record.label)}</TableCell>
                                            </TableRow>
                                        }).collect_view()}
                                    </TableBody>
                                </Table>
                            }.into_any(),
                            Err(err) => view! { <ErrorBar error=err.to_string() what="recent changes" /> }.into_any(),
                        })}
                    </Suspense>
                </CardContent>
            </Card>
        </div>
    }
}

/// Percent-encode a query value (spaces, `&`, `#`, `?`, `%`, `+`, `=`).
#[must_use]
pub fn js_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            ' ' => out.push_str("%20"),
            '&' => out.push_str("%26"),
            '#' => out.push_str("%23"),
            '?' => out.push_str("%3F"),
            '%' => out.push_str("%25"),
            '+' => out.push_str("%2B"),
            '=' => out.push_str("%3D"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owned_records_open_their_pages_happy() {
        let r = RecordRef {
            kind: RecordKind::Group,
            id: "g1".into(),
            label: "Release Captains".into(),
        };
        assert_eq!(record_href(&r), "/permission/groups/g1");
        assert_eq!(
            joined_text(Some("Ines Okoro (abc123)")),
            "Added by Ines Okoro"
        );
    }

    #[test]
    fn unknown_joiner_and_odd_names_sad() {
        assert_eq!(joined_text(None), "—");
        assert_eq!(js_encode("Ravi & Co"), "Ravi%20%26%20Co");
    }
}
