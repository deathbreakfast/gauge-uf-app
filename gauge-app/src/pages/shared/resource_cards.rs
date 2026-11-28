//! Cards for resource bundles: the resource card on a resource-scoped domain, and the list of
//! resources a group can reach.

use leptos::prelude::*;
use uf_product::components::{
    Body1, Caption1, Card, CardContent, CardSectionBorder, EmptyState, SkeletonItemSize, Title3,
};
use uf_product::primitives::{
    Badge, BadgeAppearance, BadgeSize, Flex, FlexAlign, FlexGap, FlexJustify, FlexWrap, Link,
    MessageBar, MessageBarIntent, SkeletonItem, Table, TableBody, TableCell, TableCellLayout,
    TableHeader, TableHeaderCell, TableRow,
};

use crate::resource_access::ResourceBundleDto;
use crate::server::{get_resource_bundle, list_group_resource_access};

/// Placeholder rows while a card's server data loads.
fn skeleton_rows() -> impl IntoView {
    view! {
        <Flex vertical=true gap=FlexGap::Small>
            {(0..3).map(|_| view! {
                <SkeletonItem size=Signal::from(SkeletonItemSize::S32) width="100%".to_string() />
            }).collect_view()}
        </Flex>
    }
}

/// The resource behind a resource-scoped domain: its name with "Open in {app}", each
/// action's permission and who holds it, and the roles the Access card offers.
#[component]
pub fn ResourceBundleCard(
    /// The resource-scoped domain's id.
    #[prop(into)]
    domain_id: Signal<String>,
) -> impl IntoView {
    let bundle = Resource::new(
        move || domain_id.get(),
        |id| async move { get_resource_bundle(id).await },
    );
    view! {
        <div data-testid="gauge-resource-bundle-card">
        <Card>
            <CardContent>
                <Suspense fallback=skeleton_rows>
                    {move || bundle.get().map(|result| match result {
                        Ok(Some(bundle)) => bundle_view(bundle).into_any(),
                        Ok(None) => view! {
                            <EmptyState message="Gauge can't tell which resource this domain belongs to." />
                        }.into_any(),
                        Err(err) => view! {
                            <MessageBar intent=MessageBarIntent::Error>
                                {format!("Failed to load the resource: {err}")}
                            </MessageBar>
                        }.into_any(),
                    })}
                </Suspense>
            </CardContent>
        </Card>
        </div>
    }
}

fn bundle_view(bundle: ResourceBundleDto) -> impl IntoView {
    let title = bundle.resource.title();
    let open_link = bundle.resource.href.clone().map(|href| {
        let app = bundle.resource.app.clone().unwrap_or_default();
        view! {
            <span data-testid="gauge-resource-open">
                <Link href=href>{format!("Open in {app}")}</Link>
            </span>
        }
    });
    let default_owner = bundle.default_owner.clone().map(|owner| {
        view! { <Caption1>{format!("The default owner is {owner}.")}</Caption1> }
    });
    let holders_visible = bundle.holders_visible;
    view! {
        <Flex vertical=true gap=FlexGap::Medium>
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center gap=FlexGap::Small wrap=FlexWrap::Wrap>
                <Flex vertical=true gap=FlexGap::Small>
                    <span data-testid="gauge-resource-title"><Title3>{title}</Title3></span>
                    {default_owner}
                </Flex>
                {open_link}
            </Flex>
            <Show when=move || !holders_visible>
                <Caption1>"Only the resource's owners and Super User can see who holds each permission."</Caption1>
            </Show>
            <div data-testid="gauge-resource-permissions">
            <Table>
                <TableHeader>
                    <TableRow>
                        <TableHeaderCell>"Permission"</TableHeaderCell>
                        <TableHeaderCell>"Held by"</TableHeaderCell>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {bundle.permissions.into_iter().map(|row| view! {
                        <TableRow>
                            <TableCell>
                                <TableCellLayout>
                                    <Link href=format!("/permission/permissions/{}", row.permission_id)>
                                        {row.permission_name}
                                    </Link>
                                </TableCellLayout>
                            </TableCell>
                            <TableCell>
                                <TableCellLayout>
                                    <Flex gap=FlexGap::Small wrap=FlexWrap::Wrap>
                                        {row.held_by.into_iter().map(|holder| view! {
                                            <Badge appearance=BadgeAppearance::Tint size=BadgeSize::Small>
                                                {holder}
                                            </Badge>
                                        }).collect_view()}
                                    </Flex>
                                </TableCellLayout>
                            </TableCell>
                        </TableRow>
                    }).collect_view()}
                </TableBody>
            </Table>
            </div>
            <Flex vertical=true gap=FlexGap::Small>
                <Title3>"Roles"</Title3>
                <Caption1>"The Access card offers these. A mix that matches no role shows as Custom."</Caption1>
            </Flex>
            <div data-testid="gauge-resource-roles">
            <Table>
                <TableHeader>
                    <TableRow>
                        <TableHeaderCell>"Role"</TableHeaderCell>
                        <TableHeaderCell>"Grants"</TableHeaderCell>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {bundle.roles.into_iter().map(|row| view! {
                        <TableRow>
                            <TableCell><TableCellLayout>{row.role}</TableCellLayout></TableCell>
                            <TableCell>
                                <TableCellLayout>
                                    {row.grants}
                                    {row.group_href.map(|href| view! {
                                        ", through "
                                        <Link href=href>"the Owners group"</Link>
                                    })}
                                </TableCellLayout>
                            </TableCell>
                        </TableRow>
                    }).collect_view()}
                </TableBody>
            </Table>
            </div>
        </Flex>
    }
}

/// The resources a group holds a role on, each with the role.
#[component]
pub fn GroupResourceAccessCard(
    /// The group's id.
    #[prop(into)]
    group_id: Signal<String>,
) -> impl IntoView {
    let rows = Resource::new(
        move || group_id.get(),
        |id| async move { list_group_resource_access(id).await },
    );
    view! {
        <div data-testid="gauge-group-resource-access">
        <Card>
            <CardContent>
                <Flex vertical=true gap=FlexGap::Medium>
                    <Flex vertical=true gap=FlexGap::Small>
                        <Title3>"Access through this group"</Title3>
                        <Caption1>"Resources where this group has a role."</Caption1>
                    </Flex>
                    <Suspense fallback=skeleton_rows>
                        {move || rows.get().map(|result| match result {
                            Ok(rows) if rows.is_empty() => view! {
                                <EmptyState message="This group has no role on any resource." />
                            }.into_any(),
                            Ok(rows) => view! {
                                <Flex vertical=true gap=FlexGap::Small>
                                    {rows.into_iter().map(|row| view! {
                                        <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center gap=FlexGap::Small>
                                            <Link href=format!("/permission/domains/{}", row.resource.domain_id)>
                                                <Body1>{row.resource.title()}</Body1>
                                            </Link>
                                            <Caption1>{row.role}</Caption1>
                                        </Flex>
                                        <CardSectionBorder />
                                    }).collect_view()}
                                </Flex>
                            }.into_any(),
                            Err(err) => view! {
                                <MessageBar intent=MessageBarIntent::Error>
                                    {format!("Failed to load this group's access: {err}")}
                                </MessageBar>
                            }.into_any(),
                        })}
                    </Suspense>
                </Flex>
            </CardContent>
        </Card>
        </div>
    }
}
