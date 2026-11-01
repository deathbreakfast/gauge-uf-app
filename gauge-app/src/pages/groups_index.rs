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
use crate::server::list_groups;

/// Permission group list: search box, create-group entry point, and per-row open links.
#[component]
pub fn GroupsIndexPage() -> impl IntoView {
    let (search, missing) = use_index_search(crate::paths::GROUPS, |name| async move {
        list_groups(Some(name))
            .await
            .map(|rows| rows.into_iter().map(|row| (row.id, row.name)).collect())
    });
    let groups = Resource::new(
        move || search.get(),
        move |query| async move {
            let q = query.trim().to_string();
            let filter = if q.is_empty() { None } else { Some(q) };
            list_groups(filter).await
        },
    );

    view! {
        <div id="gauge-groups-page">
        <ContentContainer max_width="1100px" data_testid="gauge-groups-index">
            <Flex vertical=true gap=FlexGap::Medium>
                <Card>
                    <Flex
                        justify=FlexJustify::SpaceBetween
                        align=FlexAlign::Center
                        gap=FlexGap::Medium
                        padding=SpacingSize::Size200.inset()
                    >
                        <Flex vertical=true gap=FlexGap::Small>
                            <Title3>"Permission Groups"</Title3>
                            <Caption1>
                                "Maintain owner and membership group structures for permission control."
                            </Caption1>
                        </Flex>
                        <div id="gauge-groups-create">
                            <A href=crate::paths::CREATE_GROUP>
                                <Button appearance=ButtonAppearance::Primary>
                                    "Create Group"
                                </Button>
                            </A>
                        </div>
                    </Flex>
                </Card>

                <Card>
                    <div id="gauge-groups-search">
                        <Flex vertical=true gap=FlexGap::Small padding=SpacingSize::Size160.inset()>
                            <Field label="Search">
                                <Input
                                    bind=search
                                    appearance=InputAppearance::with_placeholder("Search by group name or description")
                                />
                            </Field>
                            <Show when=move || missing.get().is_some()>
                                <div id="gauge-groups-name-missing">
                                <MessageBar intent=MessageBarIntent::Warning>
                                    {move || format!("No group is named {}.", missing.get().unwrap_or_default())}
                                </MessageBar>
                                </div>
                            </Show>
                        </Flex>
                    </div>
                </Card>

                <Card>
                    <div id="gauge-groups-list">
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
                            match groups.get() {
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
                                                message="No groups found."
                                                description="Create a group to get started."
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
                                                        <div id="gauge-group-row-open">
                                                            <A href=format!("/permission/groups/{}", row.id)>
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
                                                {format!("Failed to load groups: {err}")}
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
        </div>
    }
}
