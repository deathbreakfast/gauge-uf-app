//! One service user's page: what it can do, each permission with the group it comes
//! through, and its groups.
//!
//! Only its owner and people who manage every service user can open it. Anyone else, a
//! deleted service user and a mistyped id all get the same not-found page. Tokens, pausing
//! and deleting stay in Lepton behind Manage in Lepton.

use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};
use leptos_router::NavigateOptions;
use uf_product::components::{
    Body1, Body1Strong, Caption1, Card, CardContent, CardHeader, CardHeaderDescription,
    ContentContainer, EmptyState, EmptyStateCallToAction, TextTag, Title2,
};
use uf_product::primitives::{
    Badge, BadgeAppearance, BadgeColor, BadgeSize, Breadcrumb, BreadcrumbButton, BreadcrumbDivider,
    BreadcrumbItem, Button, ButtonAppearance, Flex, FlexAlign, FlexGap, FlexJustify, FlexWrap,
    Link, MessageBar, MessageBarBody, MessageBarIntent, MessageBarLayout, MessageBarTitle,
    SkeletonItem, Table, TableBody, TableCell, TableHeader, TableHeaderCell, TableRow,
};

use crate::components::error_bar::ErrorBar;
use crate::pages::people_index::SERVICE_USERS_TAB;
use crate::pages::shared::words::plain_name;
use crate::server::surfaces::{get_service_user_page, ServiceUserPage, ServiceUserPermissionRow};

/// Lepton's page for a service user, where its tokens, pausing and deleting live.
#[must_use]
pub fn lepton_href(handle: &str) -> String {
    format!("/user/service-users/{handle}")
}

/// "@release-bot · Owned by Sol Bergström", or the handle alone with no known owner.
#[must_use]
pub fn handle_line(page: &ServiceUserPage) -> String {
    match page.owner_label.as_deref().map(plain_name) {
        Some(owner) if !owner.is_empty() => format!("@{} · Owned by {owner}", page.handle),
        _ => format!("@{}", page.handle),
    }
}

/// Why it holds a permission: "Through Release Captains", or "Granted directly".
#[must_use]
pub fn why_text(row: &ServiceUserPermissionRow) -> String {
    row.via_group.as_ref().map_or_else(
        || "Granted directly".to_string(),
        |g| format!("Through {}", g.name),
    )
}

/// Where Request access goes: the groups list with the dialog up for this service user.
#[must_use]
pub fn request_href(handle: &str) -> String {
    format!("/permission/groups?request=1&for={handle}")
}

/// The service user page.
#[component]
pub fn ServiceUserDetailPage() -> impl IntoView {
    let params = use_params_map();
    let id = Memo::new(move |_| params.read().get("id").unwrap_or_default());
    let refresh = RwSignal::new(0u32);
    let page = Resource::new(
        move || (id.get(), refresh.get()),
        |(id, _)| get_service_user_page(id),
    );
    view! {
        <ContentContainer max_width="1100px" data_testid="gauge-service-user-detail">
            <Transition fallback=|| view! { <ServiceUserSkeleton /> }>
                {move || page.get().map(|result| match result {
                    Ok(Some(p)) => view! { <ServiceUserView page=p /> }.into_any(),
                    Ok(None) => view! { <ServiceUserNotFound /> }.into_any(),
                    Err(err) => view! {
                        <Flex vertical=true gap=FlexGap::Medium>
                            <Crumbs current=None />
                            <ErrorBar error=err.to_string() what="this service user" on_retry=Callback::new(move |()| refresh.update(|n| *n += 1)) />
                        </Flex>
                    }.into_any(),
                })}
            </Transition>
        </ContentContainer>
    }
}

/// People › Service users › {name}, or People › Not found.
#[component]
fn Crumbs(current: Option<String>) -> impl IntoView {
    let found = current.is_some();
    view! {
        <div data-testid="gauge-service-user-crumbs">
            <Breadcrumb>
                <BreadcrumbItem>
                    <BreadcrumbButton href="/permission/people".to_string()>"People"</BreadcrumbButton>
                </BreadcrumbItem>
                <BreadcrumbDivider />
                <Show when=move || found>
                    <BreadcrumbItem>
                        <BreadcrumbButton href=format!("/permission/people?tab={SERVICE_USERS_TAB}")>"Service users"</BreadcrumbButton>
                    </BreadcrumbItem>
                    <BreadcrumbDivider />
                </Show>
                <BreadcrumbItem>
                    <BreadcrumbButton current=true>{current.clone().unwrap_or_else(|| "Not found".to_string())}</BreadcrumbButton>
                </BreadcrumbItem>
            </Breadcrumb>
        </div>
    }
}

#[component]
fn ServiceUserSkeleton() -> impl IntoView {
    view! {
        <div data-testid="gauge-service-user-skeleton">
            <Flex vertical=true gap=FlexGap::Medium>
                <SkeletonItem width="240px".to_string() />
                <Card>
                    <CardHeader>"What it can do"</CardHeader>
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
fn ServiceUserNotFound() -> impl IntoView {
    let navigate = use_navigate();
    view! {
        <div data-testid="gauge-service-user-not-found">
            <Flex vertical=true gap=FlexGap::Medium>
                <Crumbs current=None />
                <EmptyState
                    message="Service user not found"
                    description="It may have been deleted, or it isn't one of yours."
                >
                    <EmptyStateCallToAction slot:call_to_action>
                        <Button on_click=Callback::new(move |_| navigate("/user/service-users", NavigateOptions::default()))>
                            "Your service users"
                        </Button>
                    </EmptyStateCallToAction>
                </EmptyState>
            </Flex>
        </div>
    }
}

#[component]
fn ServiceUserView(page: ServiceUserPage) -> impl IntoView {
    let name = page.display_name.clone();
    let active = page.active;
    let n_perms = page.permissions.len();
    let n_groups = page.groups.len();
    let navigate = use_navigate();
    let request = request_href(&page.handle);
    let groups_caption = format!("{name} holds what these groups hold.");
    let paused_title = format!("{name} is paused");
    let handle_text = handle_line(&page);
    let manage_href = lepton_href(&page.handle);
    let permissions = page.permissions;
    let groups = page.groups;
    view! {
        <Flex vertical=true gap=FlexGap::Large>
            <Crumbs current=Some(name.clone()) />
            <div id="gauge-service-user-header">
                <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center gap=FlexGap::Medium wrap=FlexWrap::Wrap>
                    <Flex vertical=true gap=FlexGap::Small>
                        <Flex align=FlexAlign::Center gap=FlexGap::Small wrap=FlexWrap::Wrap>
                            <Title2 tag=TextTag::H1>{name.clone()}</Title2>
                            <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Informative size=BadgeSize::Small>"Service user"</Badge>
                            <span data-testid="gauge-service-user-status">
                                {if active {
                                    view! { <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Success size=BadgeSize::Small>"Active"</Badge> }.into_any()
                                } else {
                                    view! { <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Warning size=BadgeSize::Small>"Paused"</Badge> }.into_any()
                                }}
                            </span>
                        </Flex>
                        <Body1>{handle_text}</Body1>
                    </Flex>
                    <Link href=manage_href>"Manage in Lepton"</Link>
                </Flex>
            </div>
            {(!active).then(|| view! {
                <div data-testid="gauge-service-user-paused">
                    <MessageBar intent=MessageBarIntent::Warning layout=MessageBarLayout::Multiline>
                        <MessageBarBody>
                            <MessageBarTitle>{paused_title.clone()}</MessageBarTitle>
                            "It holds none of its groups' access until it's resumed. Resume it in Lepton."
                        </MessageBarBody>
                    </MessageBar>
                </div>
            })}
            <div id="gauge-service-user-permissions">
                <Card>
                    <CardHeader>
                        {format!("What it can do ({n_perms})")}
                        <CardHeaderDescription slot>"Every permission check it passes today."</CardHeaderDescription>
                    </CardHeader>
                    <CardContent>
                        {if permissions.is_empty() {
                            view! { <EmptyState message="It can't do anything yet" /> }.into_any()
                        } else {
                            view! {
                                <Table>
                                    <TableHeader>
                                        <TableRow>
                                            <TableHeaderCell>"Permission"</TableHeaderCell>
                                            <TableHeaderCell>"Why it has it"</TableHeaderCell>
                                        </TableRow>
                                    </TableHeader>
                                    <TableBody>
                                        {permissions.into_iter().map(|p| {
                                            let why = why_text(&p);
                                            let title = match p.id.clone() {
                                                Some(id) => view! {
                                                    <Link href=format!("/permission/permissions/{id}")><Body1Strong>{p.display_name.clone()}</Body1Strong></Link>
                                                }.into_any(),
                                                None => view! { <Body1Strong>{p.display_name.clone()}</Body1Strong> }.into_any(),
                                            };
                                            let via = p.via_group.clone();
                                            view! {
                                                <TableRow attr:data-testid="gauge-service-user-permission-row">
                                                    <TableCell>
                                                        <Flex vertical=true gap=FlexGap::Size(0)>
                                                            {title}
                                                            <Caption1>{p.description.clone()}</Caption1>
                                                        </Flex>
                                                    </TableCell>
                                                    <TableCell>
                                                        {match via {
                                                            Some(g) => view! {
                                                                <Body1>"Through " <Link href=format!("/permission/groups/{}", g.id)>{g.name}</Link></Body1>
                                                            }.into_any(),
                                                            None => view! { <Body1>{why}</Body1> }.into_any(),
                                                        }}
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
            <div id="gauge-service-user-groups">
                <Card>
                    <CardHeader>
                        {format!("Groups ({n_groups})")}
                        <CardHeaderDescription slot>{groups_caption}</CardHeaderDescription>
                    </CardHeader>
                    <CardContent>
                        {if groups.is_empty() {
                            view! {
                                <div data-testid="gauge-service-user-no-groups">
                                    <EmptyState
                                        message="Not in any groups yet"
                                        description="A group's owner can add it, or you can ask for access for it."
                                    >
                                        <EmptyStateCallToAction slot:call_to_action>
                                            <Button appearance=ButtonAppearance::Primary on_click=Callback::new(move |_| navigate(&request, NavigateOptions::default()))>
                                                "Request access"
                                            </Button>
                                        </EmptyStateCallToAction>
                                    </EmptyState>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <Table>
                                    <TableHeader>
                                        <TableRow>
                                            <TableHeaderCell>"Group"</TableHeaderCell>
                                            <TableHeaderCell>""</TableHeaderCell>
                                        </TableRow>
                                    </TableHeader>
                                    <TableBody>
                                        {groups.into_iter().map(|g| {
                                            let href = format!("/permission/groups/{}", g.id);
                                            let navigate = use_navigate();
                                            view! {
                                                <TableRow attr:data-testid="gauge-service-user-group-row">
                                                    <TableCell>
                                                        <Flex vertical=true gap=FlexGap::Size(0)>
                                                            <Body1Strong>{g.name}</Body1Strong>
                                                            <Caption1>{g.description}</Caption1>
                                                        </Flex>
                                                    </TableCell>
                                                    <TableCell>
                                                        <Button appearance=ButtonAppearance::Subtle on_click=Callback::new(move |_| navigate(&href, NavigateOptions::default()))>
                                                            "Open"
                                                        </Button>
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
        </Flex>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gauge::types::ServiceUserGroupDto;

    fn page(owner: Option<&str>) -> ServiceUserPage {
        ServiceUserPage {
            id: "su1".into(),
            handle: "release-bot".into(),
            display_name: "Release bot".into(),
            active: true,
            owner_label: owner.map(Into::into),
            groups: Vec::new(),
            permissions: Vec::new(),
        }
    }

    #[test]
    fn header_names_the_owner_happy() {
        assert_eq!(
            handle_line(&page(Some("Sol Bergström (h3jx6p)"))),
            "@release-bot · Owned by Sol Bergström"
        );
        assert_eq!(
            lepton_href("release-bot"),
            "/user/service-users/release-bot"
        );
        assert_eq!(
            request_href("release-bot"),
            "/permission/groups?request=1&for=release-bot"
        );
    }

    #[test]
    fn no_owner_and_direct_grants_sad() {
        assert_eq!(handle_line(&page(None)), "@release-bot");
        let direct = ServiceUserPermissionRow {
            id: None,
            display_name: "Approve releases".into(),
            description: String::new(),
            via_group: None,
        };
        assert_eq!(why_text(&direct), "Granted directly");
        let through = ServiceUserPermissionRow {
            via_group: Some(ServiceUserGroupDto {
                id: "g1".into(),
                name: "Release Captains".into(),
                description: String::new(),
            }),
            ..direct
        };
        assert_eq!(why_text(&through), "Through Release Captains");
    }
}
