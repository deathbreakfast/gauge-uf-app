use leptos::prelude::*;
use leptos_router::components::A;
use uf_product::components::{Body1, Caption1, Card, EmptyState, SkeletonItemSize};
use uf_product::components::{
    CardContent, CardSectionBorder, ContentContainer, SpacingHorizontal, SpacingInset, SpacingSize,
    SpacingVertical, Title3,
};
use uf_product::primitives::{
    Button, ButtonAppearance, Field, Flex, FlexAlign, FlexGap, FlexJustify, Input, InputAppearance,
    MessageBar, MessageBarIntent, SkeletonItem,
};

use crate::pages::shared::query_state::use_index_search;
use crate::server::list_permissions;

/// Permission list: search box, create-permission entry point, and per-row open links.
#[component]
pub fn PermissionsIndexPage() -> impl IntoView {
    let (search, missing) = use_index_search(crate::paths::PERMISSIONS, |name| async move {
        list_permissions(Some(name))
            .await
            .map(|rows| rows.into_iter().map(|row| (row.id, row.name)).collect())
    });
    let permissions = Resource::new(
        move || search.get(),
        move |query| async move {
            let q = query.trim().to_string();
            let filter = if q.is_empty() { None } else { Some(q) };
            list_permissions(filter).await
        },
    );

    view! {
        <ContentContainer max_width="1100px" data_testid="gauge-permissions-index">
            <Flex vertical=true gap=FlexGap::Medium>
                <Card>
                    <Flex
                        justify=FlexJustify::SpaceBetween
                        align=FlexAlign::Center
                        gap=FlexGap::Medium
                        padding=SpacingSize::Size200.inset()
                    >
                        <Flex vertical=true gap=FlexGap::Small>
                            <Title3>"Permissions"</Title3>
                            <Caption1>
                                "Manage permission definitions, owner groups, and effective access."
                            </Caption1>
                        </Flex>
                        <div id="gauge-permissions-create">
                            <A href=crate::paths::CREATE_PERMISSION>
                                <Button appearance=ButtonAppearance::Primary>
                                    "Create Permission"
                                </Button>
                            </A>
                        </div>
                    </Flex>
                </Card>

                <Card>
                    <div id="gauge-permissions-search">
                        <Flex vertical=true gap=FlexGap::Small padding=SpacingSize::Size160.inset()>
                            <Field label="Search">
                                <Input
                                    bind=search
                                    appearance=InputAppearance::with_placeholder("Search by permission name or description")
                                />
                            </Field>
                            <Show when=move || missing.get().is_some()>
                                <div id="gauge-permissions-name-missing">
                                <MessageBar intent=MessageBarIntent::Warning>
                                    {move || format!("No permission is named {}.", missing.get().unwrap_or_default())}
                                </MessageBar>
                                </div>
                            </Show>
                        </Flex>
                    </div>
                </Card>

                <Card>
                    <div id="gauge-permissions-list">
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
                            match permissions.get() {
                                None => view! {
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
                                }.into_any(),
                                Some(Ok(items)) if items.is_empty() => {
                                    view! {
                                        <CardContent>
                                            <EmptyState
                                                message="No permissions found."
                                                description="Create a permission to get started."
                                            />
                                        </CardContent>
                                    }
                                        .into_any()
                                }
                                Some(Ok(items)) => {
                                    view! {
                                        <Flex vertical=true gap=FlexGap::Small padding=SpacingInset::symmetric(SpacingHorizontal::M, SpacingVertical::S)>
                                            <For each=move || items.clone() key=|row| row.id.clone() let:row>
                                                <>
                                                    <Flex
                                                        justify=FlexJustify::SpaceBetween
                                                        align=FlexAlign::Center
                                                        gap=FlexGap::Medium
                                                        padding=SpacingSize::Size120.inset()
                                                    >
                                                        <Flex vertical=true gap=FlexGap::Small>
                                                            <Body1>{row.name.clone()}</Body1>
                                                            <Caption1>{row.description.clone()}</Caption1>
                                                        </Flex>
                                                        <div id="gauge-permission-row-open">
                                                            <A href=format!("/permission/permissions/{}", row.id)>
                                                                <Button appearance=ButtonAppearance::Subtle>
                                                                    "Open"
                                                                </Button>
                                                            </A>
                                                        </div>
                                                    </Flex>
                                                    <CardSectionBorder />
                                                </>
                                            </For>
                                        </Flex>
                                    }.into_any()
                                }
                                Some(Err(err)) => {
                                    view! {
                                        <CardContent>
                                            <MessageBar intent=MessageBarIntent::Error>
                                                {format!("Failed to load permissions: {err}")}
                                            </MessageBar>
                                        </CardContent>
                                    }
                                        .into_any()
                                }
                            }
                        }}
                    </Transition>
                    </div>
                </Card>
            </Flex>
        </ContentContainer>
    }
}
