//! Gauge's landing page.
//!
//! Everyone sees Waiting for you (requests they can answer) and Your access (what they can do,
//! or, with nothing yet, their open requests and who reviews them). Gauge admins also get the
//! counts and Recent changes. Each card loads on its own after hydration, so a slow read
//! holds up only its card.

use gauge::types::{AccessPath, PermissionRequestRowDto, PermissionRequestStatusDto};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;
use uf_product::components::{
    AutoGrid, Body1, Body1Strong, Caption1, Card, CardContent, CardHeader, CardHeaderAction,
    ContentContainer, EmptyState, EmptyStateCallToAction, Persona, PersonaConfig,
    PersonaSecondaryText, StatCard, TextTag, Title2,
};
use uf_product::primitives::{
    Button, ButtonAppearance, Divider, Flex, FlexAlign, FlexGap, FlexJustify, FlexWrap, Link,
    SkeletonItem, Table, TableBody, TableCell, TableHeader, TableHeaderCell, TableRow,
};

use crate::components::error_bar::ErrorBar;
use crate::pages::shared::local_resource;
use crate::pages::shared::reviewers::{reviewers_for, reviewers_of};
use crate::pages::shared::words::{asks_for, plain_name, status_words, when_view};
use crate::server::surfaces::get_overview_stats;
use crate::server::viewer::{use_viewer_access, viewer_now};
use crate::server::{
    get_activity, get_my_access, list_my_permission_requests, list_review_permission_requests,
};

/// How many recent changes the Overview lists.
const RECENT_CHANGES: usize = 5;

/// A count for a stat card, with "+" when it stopped at its cap.
#[must_use]
pub fn count_text(n: usize, capped: bool) -> String {
    if capped {
        format!("{n}+")
    } else {
        n.to_string()
    }
}

/// One sentence about what someone holds: "Super User: you pass every permission check. Also
/// in Metrics readers." or "You can do 3 things through Release Captains and Metrics readers."
/// `None` when they hold nothing and belong to no group.
#[must_use]
pub fn access_sentence(access: &gauge::types::PrincipalAccess) -> Option<String> {
    let super_user = access
        .permissions
        .iter()
        .any(|(_, paths)| paths.contains(&AccessPath::SuperUser));
    let groups: Vec<String> = access
        .groups
        .iter()
        .map(|g| g.group.name.clone())
        .filter(|name| !(super_user && name == "Super User"))
        .collect();
    let groups_text = crate::server::surfaces::or_list(&groups).replace(" or ", " and ");
    if super_user {
        let also = if groups.is_empty() {
            String::new()
        } else {
            format!(" Also in {groups_text}.")
        };
        return Some(format!(
            "Super User: you pass every permission check.{also}"
        ));
    }
    let n = access.permissions.len();
    if n == 0 && groups.is_empty() {
        return None;
    }
    let things = if n == 1 {
        "1 thing".to_string()
    } else {
        format!("{n} things")
    };
    Some(if groups.is_empty() {
        format!("You can do {things}, each granted to you directly.")
    } else {
        format!("You can do {things}. You're in {groups_text}.")
    })
}

fn skeleton_lines(n: usize) -> impl IntoView {
    view! {
        <Flex vertical=true gap=FlexGap::Small>
            {(0..n).map(|_| view! { <SkeletonItem width="100%".to_string() /> }).collect_view()}
        </Flex>
    }
}

/// The Overview page.
#[component]
pub fn OverviewPage() -> impl IntoView {
    let viewer = use_viewer_access();
    let is_admin = Signal::derive(move || viewer_now(viewer).gauge_admin);
    let refresh = RwSignal::new(0u32);
    let failed = RwSignal::new(None::<String>);
    let subtitle = move || {
        if is_admin.get() {
            "Who can do what across your apps, and the requests waiting for an answer."
        } else {
            "What you can do across your apps, and how to ask for more."
        }
    };

    view! {
        <ContentContainer max_width="1100px" data_testid="gauge-overview">
            <Flex vertical=true gap=FlexGap::Large>
                <div id="gauge-overview-header">
                    <Flex vertical=true gap=FlexGap::Small>
                        <Title2 tag=TextTag::H1>"Gauge"</Title2>
                        <Body1>{subtitle}</Body1>
                    </Flex>
                </div>
                <Show when=move || failed.get().is_some()>
                    <ErrorBar
                        error=Signal::derive(move || failed.get().unwrap_or_default())
                        what="the overview"
                        on_retry=Callback::new(move |()| {
                            failed.set(None);
                            refresh.update(|n| *n += 1);
                        })
                    />
                </Show>
                <Show when=move || is_admin.get()>
                    <StatCards refresh=refresh failed=failed />
                </Show>
                <WaitingForYou refresh=refresh failed=failed />
                <Show when=move || is_admin.get()>
                    <RecentChanges refresh=refresh failed=failed />
                </Show>
                <YourAccess refresh=refresh failed=failed />
            </Flex>
        </ContentContainer>
    }
}

#[component]
fn StatCards(refresh: RwSignal<u32>, failed: RwSignal<Option<String>>) -> impl IntoView {
    let stats = local_resource(move || refresh.get(), |_| get_overview_stats());
    // `capped` covers the counts that page (people, changes); groups and permissions are exact.
    let value = move |pick: fn(&crate::server::surfaces::OverviewStats) -> (usize, bool)| {
        Signal::derive(move || match stats.get() {
            Some(Ok(s)) => {
                let (n, capped) = pick(&s);
                count_text(n, capped)
            }
            _ => "–".to_string(),
        })
    };
    Effect::new(move |_| {
        if let Some(Err(e)) = stats.get() {
            failed.set(Some(e.to_string()));
        }
    });
    view! {
        <div data-testid="gauge-overview-stats">
            <AutoGrid min="160px">
                <StatCard label="People" value=value(|s| (s.people, s.capped)) />
                <StatCard label="Groups" value=value(|s| (s.groups, false)) />
                <StatCard label="Permissions" value=value(|s| (s.permissions, false)) />
                <StatCard label="Changes this week" value=value(|s| (s.changes_this_week, s.capped)) />
            </AutoGrid>
        </div>
    }
}

#[component]
fn WaitingForYou(refresh: RwSignal<u32>, failed: RwSignal<Option<String>>) -> impl IntoView {
    let rows = local_resource(
        move || refresh.get(),
        |_| async move {
            list_review_permission_requests().await.map(|rows| {
                rows.into_iter()
                    .filter(|r| r.status == PermissionRequestStatusDto::Pending)
                    .collect::<Vec<_>>()
            })
        },
    );
    Effect::new(move |_| {
        if let Some(Err(e)) = rows.get() {
            failed.set(Some(e.to_string()));
        }
    });
    // Local resources only load in the browser; don't read one while server rendering.
    let count = move || match (!cfg!(feature = "ssr")).then(|| rows.get()).flatten() {
        Some(Ok(list)) if !list.is_empty() => format!("Waiting for you ({})", list.len()),
        _ => "Waiting for you".to_string(),
    };
    view! {
        <div id="gauge-overview-waiting">
            <Card>
                <CardHeader>
                    {count}
                    <CardHeaderAction slot>
                        <Link href="/permission/requests">"All requests"</Link>
                    </CardHeaderAction>
                </CardHeader>
                <CardContent>
                    <Suspense fallback=|| skeleton_lines(2)>
                        {move || rows.get().map(|result| match result {
                            Ok(list) if list.is_empty() => view! {
                                <div data-testid="gauge-overview-waiting-empty">
                                    <EmptyState
                                        message="Nothing is waiting for you"
                                        description="Requests for the groups and permissions you own show up here."
                                    />
                                </div>
                            }.into_any(),
                            Ok(list) => view! {
                                <Flex vertical=true gap=FlexGap::Small>
                                    {list.into_iter().map(waiting_row).collect_view()}
                                </Flex>
                            }.into_any(),
                            Err(_) => skeleton_lines(2).into_any(),
                        })}
                    </Suspense>
                </CardContent>
            </Card>
        </div>
    }
}

fn waiting_row(row: PermissionRequestRowDto) -> impl IntoView {
    let navigate = use_navigate();
    let href = format!("/permission/requests/{}", row.id);
    let name = plain_name(&row.requester_label);
    let mut what = asks_for(&row);
    if let Some(su) = &row.on_behalf_of {
        what = format!("{what} {}", crate::pages::shared::words::for_line(su));
    }
    view! {
        <div data-testid="gauge-overview-waiting-row">
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center gap=FlexGap::Medium wrap=FlexWrap::Wrap>
                <Persona config=PersonaConfig::named(name)>
                    <PersonaSecondaryText slot>{what}</PersonaSecondaryText>
                </Persona>
                <Flex align=FlexAlign::Center gap=FlexGap::Medium>
                    <Caption1>{when_view(row.created_at.clone())}</Caption1>
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=Callback::new(move |_| navigate(&href, NavigateOptions::default()))
                    >
                        "Review"
                    </Button>
                </Flex>
            </Flex>
            <Divider />
        </div>
    }
}

#[component]
fn RecentChanges(refresh: RwSignal<u32>, failed: RwSignal<Option<String>>) -> impl IntoView {
    let rows = local_resource(
        move || refresh.get(),
        |_| async move {
            get_activity(gauge::types::ActivityFilter::default(), None)
                .await
                .map(|page| {
                    page.items
                        .into_iter()
                        .take(RECENT_CHANGES)
                        .collect::<Vec<_>>()
                })
        },
    );
    Effect::new(move |_| {
        if let Some(Err(e)) = rows.get() {
            failed.set(Some(e.to_string()));
        }
    });
    view! {
        <div id="gauge-overview-recent">
            <Card>
                <CardHeader>
                    "Recent changes"
                    <CardHeaderAction slot>
                        <Link href="/permission/activity">"All activity"</Link>
                    </CardHeaderAction>
                </CardHeader>
                <CardContent>
                    <Suspense fallback=|| skeleton_lines(3)>
                        {move || rows.get().map(|result| match result {
                            Ok(list) if list.is_empty() => view! {
                                <EmptyState message="No changes yet" />
                            }.into_any(),
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
                            Err(_) => skeleton_lines(3).into_any(),
                        })}
                    </Suspense>
                </CardContent>
            </Card>
        </div>
    }
}

#[component]
fn YourAccess(refresh: RwSignal<u32>, failed: RwSignal<Option<String>>) -> impl IntoView {
    let mine = local_resource(move || refresh.get(), |_| get_my_access());
    let requests = local_resource(
        move || refresh.get(),
        |_| async move {
            list_my_permission_requests().await.map(|rows| {
                rows.into_iter()
                    .filter(|r| r.status == PermissionRequestStatusDto::Pending)
                    .collect::<Vec<_>>()
            })
        },
    );
    let open_rows = Signal::derive(move || requests.get().and_then(Result::ok).unwrap_or_default());
    let reviewers = reviewers_for(open_rows);
    Effect::new(move |_| {
        if let Some(Err(e)) = mine.get() {
            failed.set(Some(e.to_string()));
        }
    });
    let navigate = use_navigate();
    let navigate_rows = navigate.clone();
    view! {
        <div id="gauge-overview-access">
            <Card>
                <CardHeader>"Your access"</CardHeader>
                <CardContent>
                    <Suspense fallback=|| skeleton_lines(2)>
                        {move || mine.get().map(|result| match result {
                            Ok(access) => match access_sentence(&access) {
                                Some(sentence) => view! {
                                    <div data-testid="gauge-overview-access-summary">
                                        <Flex vertical=true gap=FlexGap::Small>
                                            <Body1>{sentence}</Body1>
                                            <Link href="/permission/people/me">"See what you can do"</Link>
                                        </Flex>
                                    </div>
                                }.into_any(),
                                None => {
                                    let navigate = navigate.clone();
                                    view! {
                                        <div data-testid="gauge-overview-access-none">
                                            <EmptyState
                                                message="You aren't in any groups yet"
                                                description="Find a group or permission and ask for access. Its owner reviews the request."
                                            >
                                                <EmptyStateCallToAction slot:call_to_action>
                                                    <Button on_click=Callback::new(move |_| navigate("/permission/groups", NavigateOptions::default()))>
                                                        "Browse groups"
                                                    </Button>
                                                </EmptyStateCallToAction>
                                            </EmptyState>
                                        </div>
                                    }.into_any()
                                }
                            },
                            Err(_) => skeleton_lines(2).into_any(),
                        })}
                    </Suspense>
                    <Suspense fallback=|| ()>
                        {move || {
                            let rows = open_rows.get();
                            let navigate = navigate_rows.clone();
                            (!rows.is_empty()).then(|| view! {
                                <div data-testid="gauge-overview-open-requests">
                                    <Flex vertical=true gap=FlexGap::Small>
                                        {rows.into_iter().map(|row| {
                                            let href = format!("/permission/requests/{}", row.id);
                                            let navigate = navigate.clone();
                                            let title = match row.target_kind {
                                                gauge::types::PermissionRequestTargetKind::Group => format!("You asked to join {}", row.target_label),
                                                gauge::types::PermissionRequestTargetKind::Permission => format!("You asked for {}", row.target_label),
                                            };
                                            let row_for_status = row.clone();
                                            view! {
                                                <Divider />
                                                <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center gap=FlexGap::Medium wrap=FlexWrap::Wrap>
                                                    <Flex vertical=true gap=FlexGap::Size(0)>
                                                        <Body1Strong>{title}</Body1Strong>
                                                        <Caption1>
                                                            {move || status_words(&row_for_status.status, reviewers_of(reviewers, &row_for_status).as_deref())}
                                                            ". Asked " {when_view(row.created_at.clone())} "."
                                                        </Caption1>
                                                    </Flex>
                                                    <Button on_click=Callback::new(move |_| navigate(&href, NavigateOptions::default()))>
                                                        "Open request"
                                                    </Button>
                                                </Flex>
                                            }
                                        }).collect_view()}
                                    </Flex>
                                </div>
                            })
                        }}
                    </Suspense>
                </CardContent>
            </Card>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gauge::types::{GroupMembership, GroupRef, PermissionRef, PrincipalAccess};

    fn access(perms: Vec<Vec<AccessPath>>, groups: &[&str]) -> PrincipalAccess {
        PrincipalAccess {
            permissions: perms
                .into_iter()
                .enumerate()
                .map(|(i, paths)| {
                    (
                        PermissionRef {
                            id: format!("p{i}"),
                            key: format!("K{i}"),
                            display_name: format!("P{i}"),
                        },
                        paths,
                    )
                })
                .collect(),
            groups: groups
                .iter()
                .map(|g| GroupMembership {
                    group: GroupRef {
                        id: g.to_lowercase(),
                        name: (*g).to_string(),
                    },
                    added_by_label: None,
                    added_at: None,
                })
                .collect(),
            owns: Vec::new(),
            open_requests: Vec::new(),
        }
    }

    #[test]
    fn super_user_and_groups_read_as_a_sentence_happy() {
        let su = access(
            vec![vec![AccessPath::SuperUser]],
            &["Super User", "Metrics readers"],
        );
        assert_eq!(
            access_sentence(&su).as_deref(),
            Some("Super User: you pass every permission check. Also in Metrics readers.")
        );
        let member = access(vec![vec![AccessPath::Direct]], &["Release Captains"]);
        assert_eq!(
            access_sentence(&member).as_deref(),
            Some("You can do 1 thing. You're in Release Captains.")
        );
    }

    #[test]
    fn nothing_held_has_no_sentence_sad() {
        assert_eq!(access_sentence(&access(Vec::new(), &[])), None);
        assert_eq!(count_text(3, true), "3+");
        assert_eq!(count_text(3, false), "3");
    }
}
