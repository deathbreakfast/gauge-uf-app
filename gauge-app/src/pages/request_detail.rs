//! One access request: who asked for what and why, what approving gives, what they hold now,
//! and who reviews it. Reviewers approve, or deny with a reason the requester sees; the
//! requester can withdraw. A decided request says who decided and when.

use gauge::types::{
    PermissionRef, PermissionRequestDecision, PermissionRequestDecisionInput,
    PermissionRequestRowDto, PermissionRequestStatusDto, PermissionRequestTargetKind, PrincipalKey,
    PrincipalKind, PrincipalRefDto,
};
use leptos::prelude::*;
use leptos::task::spawn_local_scoped;
use leptos_router::hooks::use_params_map;
use uf_product::components::{
    Body1, Caption1, Caption1Strong, Card, CardContent, CardHeader, ContentContainer, EmptyState,
    Persona, PersonaConfig, TextTag, Title2,
};
use uf_product::primitives::{
    Badge, BadgeAppearance, BadgeColor, BadgeSize, Breadcrumb, BreadcrumbButton, BreadcrumbDivider,
    BreadcrumbItem, Button, ButtonAppearance, Dialog, DialogActions, DialogBody, DialogContent,
    DialogSurface, DialogTitle, Field, Flex, FlexAlign, FlexGap, FlexJustify, FlexWrap, Link,
    MessageBar, MessageBarBody, MessageBarIntent, MessageBarLayout, MessageBarTitle, SkeletonItem,
    Textarea,
};

use crate::components::error_bar::{ErrorBar, MaybeErrorBar};
use crate::pages::shared::local_resource;
use crate::pages::shared::words::{first_name, plain_name, status_words, when_view};
use crate::pages::step_up::spawn_with_step_up;
use crate::server::surfaces::{get_request_reviewers, get_service_user_page, review_target};
use crate::server::viewer::{use_viewer_access, viewer_now};
use crate::server::{
    decide_permission_request, deny_request, get_group_access, get_permission_request,
    get_principal_access, get_request_resource, withdraw_request,
};

/// Links from a resource request to its resource: in its app when the host knows the
/// route, and to the resource's permissions in Gauge. Nothing for other requests.
#[component]
fn RequestResourceLinks(
    target_kind: PermissionRequestTargetKind,
    target_id: String,
) -> impl IntoView {
    let resource = Resource::new(
        move || (target_kind.clone(), target_id.clone()),
        |(kind, id)| async move { get_request_resource(kind, id).await },
    );
    view! {
        <Suspense fallback=|| ()>
            {move || resource.get().and_then(Result::ok).flatten().map(|link| {
                let open = link.href.clone().map(|href| {
                    let app = link.app.clone().unwrap_or_default();
                    view! {
                        <span data-testid="gauge-request-resource-open">
                            <Link href=href>{format!("Open in {app}")}</Link>
                        </span>
                    }
                });
                view! {
                    <div data-testid="gauge-request-resource">
                    <Flex gap=FlexGap::Medium>
                        {open}
                        <Link href=format!("/permission/domains/{}", link.domain_id)>
                            {format!("Permissions for {}", link.title())}
                        </Link>
                    </Flex>
                    </div>
                }
            })}
        </Suspense>
    }
}

/// The page title: "Noor Haddad asks to join Release Captains", or for a service user "Sol
/// Bergström asks for @release-bot to join Metrics readers".
#[must_use]
pub fn request_title(row: &PermissionRequestRowDto) -> String {
    let who = plain_name(&row.requester_label);
    let target = &row.target_label;
    match (&row.on_behalf_of, &row.target_kind) {
        (Some(su), PermissionRequestTargetKind::Group) => {
            format!("{who} asks for @{} to join {target}", su.label)
        }
        (Some(su), PermissionRequestTargetKind::Permission) => {
            format!("{who} asks for {target} for @{}", su.label)
        }
        (None, PermissionRequestTargetKind::Group) => format!("{who} asks to join {target}"),
        (None, PermissionRequestTargetKind::Permission) => format!("{who} asks for {target}"),
    }
}

/// The For fact's second line: "Release bot (service user), owned by Sol Bergström".
#[must_use]
pub fn for_caption(su: &PrincipalRefDto) -> String {
    match su.owner_label.as_deref().map(plain_name) {
        Some(owner) if !owner.is_empty() => format!("Service user, owned by {owner}"),
        _ => "Service user".to_string(),
    }
}

/// What someone holds now, in a sentence: "No groups and no permissions", or
/// "Approve releases and Roll back releases, through Release Captains".
#[must_use]
pub fn holds_now(access: &gauge::types::PrincipalAccess) -> String {
    if access.permissions.is_empty() && access.groups.is_empty() {
        return "No groups and no permissions".to_string();
    }
    let perms: Vec<String> = access
        .permissions
        .iter()
        .map(|(p, _)| p.display_name.clone())
        .collect();
    let groups: Vec<String> = access.groups.iter().map(|g| g.group.name.clone()).collect();
    let and = |names: &[String]| crate::server::surfaces::or_list(names).replace(" or ", " and ");
    match (perms.is_empty(), groups.is_empty()) {
        (true, _) => format!("In {}, with no permissions", and(&groups)),
        (false, true) => and(&perms),
        (false, false) => format!("{}, through {}", and(&perms), and(&groups)),
    }
}

/// Whether two user ids name the same person, ignoring a `table:` prefix on either.
#[must_use]
pub fn same_user(a: &str, b: &str) -> bool {
    let bare = |id: &str| id.rsplit(':').next().unwrap_or(id).to_string();
    !a.is_empty() && bare(a) == bare(b)
}

/// A label and value in the facts grid.
#[component]
fn Fact(#[prop(into)] label: String, children: Children) -> impl IntoView {
    view! {
        <Flex vertical=true gap=FlexGap::Size(0)>
            <Caption1Strong>{label}</Caption1Strong>
            <Body1>{children()}</Body1>
        </Flex>
    }
}

/// Access request page: who asked for what and why, with Approve and Deny for reviewers,
/// Withdraw for the requester, and who decided once it's decided.
#[component]
pub fn RequestDetailPage() -> impl IntoView {
    let params = use_params_map();
    let request_id = Memo::new(move |_| params.read().get("id").unwrap_or_default());
    let refresh = RwSignal::new(0u64);
    let request = Resource::new(
        move || (request_id.get(), refresh.get()),
        move |(id, _)| async move { get_permission_request(id).await },
    );
    view! {
        <ContentContainer max_width="1100px" data_testid="gauge-request-detail">
            <Transition fallback=move || view! { <RequestSkeleton /> }>
                {move || request.get().map(|result| match result {
                    Ok(Some(row)) => view! { <RequestView row=row refresh=refresh /> }.into_any(),
                    Ok(None) => view! {
                        <div data-testid="gauge-request-not-found">
                            <Flex vertical=true gap=FlexGap::Medium>
                                <RequestCrumbs current="Not found".to_string() />
                                <EmptyState message="Request not found" description="It may have been withdrawn and removed, or the link has a typo." />
                            </Flex>
                        </div>
                    }.into_any(),
                    Err(err) => view! {
                        <Flex vertical=true gap=FlexGap::Medium>
                            <RequestCrumbs current="Request".to_string() />
                            <ErrorBar error=err.to_string() what="this request" on_retry=Callback::new(move |()| refresh.update(|n| *n += 1)) />
                        </Flex>
                    }.into_any(),
                })}
            </Transition>
        </ContentContainer>
    }
}

#[component]
fn RequestCrumbs(current: String) -> impl IntoView {
    view! {
        <Breadcrumb>
            <BreadcrumbItem>
                <BreadcrumbButton href="/permission/requests".to_string()>"Requests"</BreadcrumbButton>
            </BreadcrumbItem>
            <BreadcrumbDivider />
            <BreadcrumbItem>
                <BreadcrumbButton current=true>{current}</BreadcrumbButton>
            </BreadcrumbItem>
        </Breadcrumb>
    }
}

#[component]
fn RequestSkeleton() -> impl IntoView {
    view! {
        <div data-testid="gauge-request-skeleton">
            <Flex vertical=true gap=FlexGap::Medium>
                <SkeletonItem width="320px".to_string() />
                <Card>
                    <CardContent>
                        <Flex vertical=true gap=FlexGap::Small>
                            {(0..4).map(|_| view! { <SkeletonItem width="100%".to_string() /> }).collect_view()}
                        </Flex>
                    </CardContent>
                </Card>
                <Card>
                    <CardHeader>"Reason"</CardHeader>
                    <CardContent><SkeletonItem width="100%".to_string() /></CardContent>
                </Card>
            </Flex>
        </div>
    }
}

#[component]
// One request's page: its resources, actions and dialogs share the row and refresh signals.
// Splitting it into facts, actions, decision and withdraw pieces is follow-up
// gau-ux-surfaces-fu5.
#[allow(clippy::too_many_lines)]
fn RequestView(row: PermissionRequestRowDto, refresh: RwSignal<u64>) -> impl IntoView {
    let viewer = use_viewer_access();
    let error = RwSignal::new(None::<String>);
    let deny_open = RwSignal::new(false);
    let deny_reason = RwSignal::new(String::new());
    let deny_error = RwSignal::new(None::<String>);
    let withdraw_open = RwSignal::new(false);
    let row = StoredValue::new(row);
    let id = row.with_value(|r| r.id.clone());
    let is_pending = row.with_value(|r| r.status == PermissionRequestStatusDto::Pending);
    let can_review = row.with_value(|r| r.can_review) && is_pending;
    let requester_id = row.with_value(|r| r.requestor_user_id.clone());
    let is_requester = {
        let requester_id = requester_id.clone();
        Signal::derive(move || same_user(&viewer_now(viewer).user_id, &requester_id))
    };
    let is_admin = Signal::derive(move || viewer_now(viewer).gauge_admin);
    let name = row.with_value(|r| plain_name(&r.requester_label));
    let first = first_name(&name);

    let reviewers = local_resource(
        move || row.with_value(|r| review_target(&r.target_kind, &r.target_id)),
        |target| async move {
            get_request_reviewers(vec![target])
                .await
                .ok()
                .and_then(|mut lines| lines.pop())
                .unwrap_or_default()
        },
    );
    let reviewers_text = Signal::derive(move || {
        reviewers
            .get()
            .filter(|r| !r.is_empty())
            .unwrap_or_else(|| "its owners".to_string())
    });
    let members_get = local_resource(
        move || {
            row.with_value(|r| {
                (
                    r.target_kind.clone(),
                    r.target_id.clone(),
                    r.target_label.clone(),
                )
            })
        },
        |(kind, id, label)| async move {
            match kind {
                PermissionRequestTargetKind::Group => get_group_access(id).await.ok(),
                PermissionRequestTargetKind::Permission => Some(vec![PermissionRef {
                    id: id.clone(),
                    key: String::new(),
                    display_name: label,
                }]),
            }
        },
    );
    // What they hold now: the service user's access for a request on its behalf, otherwise
    // the requester's (Gauge admins and the requester only; hidden from other reviewers).
    let has_now = local_resource(
        move || row.with_value(|r| (r.requestor_user_id.clone(), r.on_behalf_of.clone())),
        |(user_id, su)| async move {
            match su {
                Some(su) => get_service_user_page(su.id).await.ok().flatten().map(|p| {
                    let perms: Vec<String> = p
                        .permissions
                        .iter()
                        .map(|x| x.display_name.clone())
                        .collect();
                    let groups: Vec<String> = p.groups.iter().map(|g| g.name.clone()).collect();
                    let and = |names: &[String]| {
                        crate::server::surfaces::or_list(names).replace(" or ", " and ")
                    };
                    match (perms.is_empty(), groups.is_empty()) {
                        (true, true) => "No groups and no permissions".to_string(),
                        (true, false) => format!("In {}, with no permissions", and(&groups)),
                        (false, true) => and(&perms),
                        (false, false) => format!("{}, through {}", and(&perms), and(&groups)),
                    }
                }),
                None => get_principal_access(PrincipalKey {
                    kind: PrincipalKind::User,
                    id: user_id,
                })
                .await
                .ok()
                .map(|a| holds_now(&a)),
            }
        },
    );

    let approve = {
        let id = id.clone();
        Callback::new(move |()| {
            let request_id = id.clone();
            let finish_ok = Callback::new(move |_: PermissionRequestRowDto| {
                refresh.update(|n| *n += 1);
                error.set(None);
            });
            spawn_with_step_up(error, finish_ok, move || {
                let request_id = request_id.clone();
                async move {
                    decide_permission_request(PermissionRequestDecisionInput {
                        request_id,
                        decision: PermissionRequestDecision::Approve,
                    })
                    .await
                }
            });
        })
    };
    let deny = {
        let id = id.clone();
        Callback::new(move |()| {
            let reason = deny_reason.get_untracked();
            if reason.trim().is_empty() {
                deny_error.set(Some(
                    "validation: Say why, so they know what to do next.".to_string(),
                ));
                return;
            }
            let id = id.clone();
            spawn_local_scoped(async move {
                match deny_request(id, reason).await {
                    Ok(_) => {
                        deny_open.set(false);
                        deny_error.set(None);
                        refresh.update(|n| *n += 1);
                    }
                    Err(e) => deny_error.set(Some(e.to_string())),
                }
            });
        })
    };
    let withdraw = {
        let id = id.clone();
        Callback::new(move |()| {
            let id = id.clone();
            spawn_local_scoped(async move {
                match withdraw_request(id).await {
                    Ok(_) => {
                        withdraw_open.set(false);
                        refresh.update(|n| *n += 1);
                    }
                    Err(e) => {
                        withdraw_open.set(false);
                        error.set(Some(e.to_string()));
                    }
                }
            });
        })
    };

    let r = row.get_value();
    let crumb = match &r.on_behalf_of {
        Some(su) => format!("@{} · {}", su.label, r.target_label),
        None => format!("{name} · {}", r.target_label),
    };
    let status = r.status.clone();
    let badge_color = match status {
        PermissionRequestStatusDto::Pending => BadgeColor::Warning,
        PermissionRequestStatusDto::Approved => BadgeColor::Success,
        PermissionRequestStatusDto::Denied => BadgeColor::Danger,
        PermissionRequestStatusDto::Withdrawn => BadgeColor::Subtle,
    };
    let status_for_badge = status.clone();
    let decided_by = r
        .decided_by_label
        .as_deref()
        .map(plain_name)
        .unwrap_or_else(|| "a reviewer".to_string());
    let decided_at = r.decided_at.map(|at| at.to_rfc3339());
    let decision = match status {
        PermissionRequestStatusDto::Approved => Some((
            MessageBarIntent::Success,
            format!("Approved by {decided_by}"),
            match r.target_kind {
                PermissionRequestTargetKind::Group => format!(
                    "{first} is now a member of {} and got a notification.",
                    r.target_label
                ),
                PermissionRequestTargetKind::Permission => {
                    format!("{first} now has {} and got a notification.", r.target_label)
                }
            },
        )),
        PermissionRequestStatusDto::Denied => Some((
            MessageBarIntent::Warning,
            format!("Denied by {decided_by}"),
            format!(
                "\u{201c}{}\u{201d} {first} sees this reason.",
                r.deny_reason.clone().unwrap_or_default()
            ),
        )),
        PermissionRequestStatusDto::Withdrawn => Some((
            MessageBarIntent::Info,
            format!("Withdrawn by {decided_by}"),
            "Nobody needs to review it now.".to_string(),
        )),
        PermissionRequestStatusDto::Pending => None,
    };
    let target_kind = r.target_kind.clone();
    let target_label = r.target_label.clone();
    let withdraw_title = match target_kind {
        PermissionRequestTargetKind::Group => {
            format!("Withdraw your request to join {target_label}?")
        }
        PermissionRequestTargetKind::Permission => {
            format!("Withdraw your request for {target_label}?")
        }
    };
    let person_href = format!("/permission/people/{requester_id}");
    let title = request_title(&r);
    let created_at = r.created_at.clone();
    let reason_text = r.reason.clone();
    let for_principal = r.on_behalf_of.clone();
    let resource_kind = r.target_kind.clone();
    let resource_id = r.target_id.clone();

    view! {
        <Flex vertical=true gap=FlexGap::Large>
            <RequestCrumbs current=crumb />
            <div id="gauge-request-detail-header">
                <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center gap=FlexGap::Medium wrap=FlexWrap::Wrap>
                    <Flex align=FlexAlign::Center gap=FlexGap::Small wrap=FlexWrap::Wrap>
                        <Title2 tag=TextTag::H1>{title}</Title2>
                        <span data-testid="gauge-request-status">
                            <Badge appearance=BadgeAppearance::Tint color=badge_color size=BadgeSize::Medium>
                                {move || status_words(&status_for_badge, Some(&reviewers_text.get()))}
                            </Badge>
                        </span>
                    </Flex>
                    <Flex gap=FlexGap::Small>
                        {can_review.then(|| view! {
                            <div id="gauge-request-approve">
                                <Button appearance=ButtonAppearance::Primary on_click=Callback::new(move |_| approve.run(()))>"Approve"</Button>
                            </div>
                            <div id="gauge-request-deny">
                                <Button appearance=ButtonAppearance::Secondary on_click=Callback::new(move |_| deny_open.set(true))>"Deny"</Button>
                            </div>
                        })}
                        <Show when=move || is_pending && !can_review && is_requester.get()>
                            <div id="gauge-request-withdraw">
                                <Button appearance=ButtonAppearance::Secondary on_click=Callback::new(move |_| withdraw_open.set(true))>"Withdraw request"</Button>
                            </div>
                        </Show>
                    </Flex>
                </Flex>
            </div>
            <MaybeErrorBar error=error what="this request" />
            {decision.map(|(intent, title, text)| view! {
                <div data-testid="gauge-request-decision">
                    <MessageBar intent=intent layout=MessageBarLayout::Multiline>
                        <MessageBarBody>
                            <MessageBarTitle>{title} {decided_at.clone().map(|at| view! { ", " {when_view(at)} })}</MessageBarTitle>
                            {text}
                        </MessageBarBody>
                    </MessageBar>
                </div>
            })}
            <Show when=move || is_pending && !can_review && is_requester.get()>
                <div data-testid="gauge-request-who-reviews">
                    <MessageBar intent=MessageBarIntent::Info layout=MessageBarLayout::Multiline>
                        <MessageBarBody>
                            <MessageBarTitle>{move || format!("{} reviews this", reviewers_text.get())}</MessageBarTitle>
                            "They got a notification when you asked. You'll get one when it's answered."
                        </MessageBarBody>
                    </MessageBar>
                </div>
            </Show>
            <div id="gauge-request-detail-summary">
                <Card>
                    <CardContent>
                        <Flex gap=FlexGap::Large wrap=FlexWrap::Wrap>
                            <Fact label="Asked by">
                                <Persona config=PersonaConfig::named(name.clone()) />
                            </Fact>
                            <Fact label="Asked">{when_view(created_at)}</Fact>
                            {for_principal.map(|su| {
                                let caption = for_caption(&su);
                                view! {
                                    <div id="gauge-request-detail-for">
                                        <Fact label="For">
                                            <Flex vertical=true gap=FlexGap::Size(0)>
                                                <Link href=format!("/permission/service-users/{}", su.id)>{format!("@{}", su.label)}</Link>
                                                <Caption1>{caption}</Caption1>
                                            </Flex>
                                        </Fact>
                                    </div>
                                }
                            })}
                            <Fact label="Members get">
                                <span data-testid="gauge-request-members-get">
                                    <Suspense fallback=|| view! { <SkeletonItem width="160px".to_string() /> }>
                                        {move || members_get.get().map(|perms| match perms {
                                            Some(perms) if !perms.is_empty() => perms.into_iter().map(|p| p.display_name).collect::<Vec<_>>().join(", "),
                                            Some(_) => "No permissions yet".to_string(),
                                            None => "Hidden from you".to_string(),
                                        })}
                                    </Suspense>
                                </span>
                            </Fact>
                            <Suspense fallback=|| ()>
                                {move || has_now.get().flatten().map(|text| {
                                    let label = row.with_value(|r| match &r.on_behalf_of {
                                        Some(su) => format!("@{} has now", su.label),
                                        None => format!("{} has now", first_name(&plain_name(&r.requester_label))),
                                    });
                                    let person_href = person_href.clone();
                                    view! {
                                        <div data-testid="gauge-request-has-now">
                                            <Fact label=label>
                                                <Flex vertical=true gap=FlexGap::Size(0)>
                                                    {text}
                                                    <Show when=move || is_admin.get() && row.with_value(|r| r.on_behalf_of.is_none())>
                                                        <Link href=person_href.clone()>"Open their page"</Link>
                                                    </Show>
                                                </Flex>
                                            </Fact>
                                        </div>
                                    }
                                })}
                            </Suspense>
                            <Fact label="Reviewed by">{move || reviewers_text.get()}</Fact>
                        </Flex>
                        <RequestResourceLinks target_kind=resource_kind target_id=resource_id />
                    </CardContent>
                </Card>
            </div>
            <Card>
                <CardHeader>"Reason"</CardHeader>
                <CardContent>
                    <Body1>{reason_text}</Body1>
                </CardContent>
            </Card>
            <Dialog open=deny_open>
                <DialogSurface>
                    <DialogBody>
                        <DialogTitle>{format!("Deny {first}'s request?")}</DialogTitle>
                        <DialogContent>
                            <div data-testid="gauge-request-deny-dialog">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Body1>{format!("{first} sees your reason in the notification and on the request.")}</Body1>
                                    <Field label="Reason">
                                        <Textarea bind=deny_reason />
                                    </Field>
                                    <MaybeErrorBar error=deny_error what="the request" />
                                </Flex>
                            </div>
                        </DialogContent>
                        <DialogActions>
                            <Button appearance=ButtonAppearance::Secondary on_click=Callback::new(move |_| { deny_open.set(false); deny_error.set(None); })>"Cancel"</Button>
                            <Button appearance=ButtonAppearance::Primary on_click=Callback::new(move |_| deny.run(()))>"Deny request"</Button>
                        </DialogActions>
                    </DialogBody>
                </DialogSurface>
            </Dialog>
            <Dialog open=withdraw_open>
                <DialogSurface>
                    <DialogBody>
                        <DialogTitle>{withdraw_title}</DialogTitle>
                        <DialogContent>
                            <Body1>{move || format!("{} stops seeing it in Waiting for you. You can ask again later.", reviewers_text.get())}</Body1>
                        </DialogContent>
                        <DialogActions>
                            <Button appearance=ButtonAppearance::Secondary on_click=Callback::new(move |_| withdraw_open.set(false))>"Keep it"</Button>
                            <div id="gauge-request-withdraw-confirm">
                                <Button appearance=ButtonAppearance::Primary on_click=Callback::new(move |_| withdraw.run(()))>"Withdraw request"</Button>
                            </div>
                        </DialogActions>
                    </DialogBody>
                </DialogSurface>
            </Dialog>
        </Flex>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gauge::types::{GroupMembership, GroupRef, PrincipalAccess};

    fn row(for_handle: Option<&str>) -> PermissionRequestRowDto {
        PermissionRequestRowDto {
            id: "r1".into(),
            target_kind: PermissionRequestTargetKind::Group,
            target_id: "g1".into(),
            target_label: "Release Captains".into(),
            requestor_user_id: "noor".into(),
            approver_user_id: None,
            reason: "covering shifts".into(),
            status: PermissionRequestStatusDto::Pending,
            created_at: String::new(),
            updated_at: String::new(),
            can_review: false,
            requester_label: "Noor Haddad (abc123)".into(),
            decided_by: None,
            decided_by_label: None,
            decided_at: None,
            deny_reason: None,
            on_behalf_of: for_handle.map(|h| bot(h, Some("Sol Bergström"))),
        }
    }

    fn bot(handle: &str, owner: Option<&str>) -> PrincipalRefDto {
        PrincipalRefDto {
            kind: PrincipalKind::ServiceUser,
            id: "su1".into(),
            label: handle.into(),
            owner_label: owner.map(Into::into),
        }
    }

    #[test]
    fn titles_say_who_asks_for_what_happy() {
        assert_eq!(
            request_title(&row(None)),
            "Noor Haddad asks to join Release Captains"
        );
        assert_eq!(
            request_title(&row(Some("release-bot"))),
            "Noor Haddad asks for @release-bot to join Release Captains"
        );
        assert_eq!(
            for_caption(&bot("release-bot", Some("Sol Bergström"))),
            "Service user, owned by Sol Bergström"
        );
    }

    #[test]
    fn same_user_ignores_table_prefix_happy() {
        assert!(same_user("user:noor", "noor"));
        assert!(!same_user("", ""));
        assert!(!same_user("noor", "sol"));
    }

    #[test]
    fn holds_now_reads_as_a_sentence_sad() {
        let none = PrincipalAccess {
            permissions: Vec::new(),
            groups: Vec::new(),
            owns: Vec::new(),
            open_requests: Vec::new(),
        };
        assert_eq!(holds_now(&none), "No groups and no permissions");
        let some = PrincipalAccess {
            permissions: vec![(
                PermissionRef {
                    id: "p".into(),
                    key: "K".into(),
                    display_name: "Approve releases".into(),
                },
                Vec::new(),
            )],
            groups: vec![GroupMembership {
                group: GroupRef {
                    id: "g".into(),
                    name: "Release Captains".into(),
                },
                added_by_label: None,
                added_at: None,
            }],
            ..none
        };
        assert_eq!(
            holds_now(&some),
            "Approve releases, through Release Captains"
        );
        assert_eq!(for_caption(&bot("bot", None)), "Service user");
    }
}
