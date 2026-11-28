use leptos::prelude::*;
use leptos_router::components::A;
use uf_product::components::{Body1, Caption1, Card, EmptyState, SkeletonItemSize};
use uf_product::components::{
    CardContent, CardSectionBorder, ContentContainer, SpacingHorizontal, SpacingInset, SpacingSize,
    SpacingVertical, Title3,
};
use uf_product::primitives::{
    Badge, BadgeAppearance, BadgeColor, BadgeSize, Button, ButtonAppearance, Field, Flex,
    FlexAlign, FlexGap, FlexJustify, Input, InputAppearance, MessageBar, MessageBarIntent,
    SkeletonItem, Tab, TabList,
};

use crate::pages::shared::query_state::use_index_search;
use crate::resource_access::GroupRowDto;
use crate::server::{list_group_rows, list_groups};

/// Tab value for every group.
const ALL_TAB: &str = "all";
/// Tab value for groups another product manages (Polaron teams).
const TEAMS_TAB: &str = "teams";

/// The line under a group's name: its description, or where a team group's members come from.
fn group_caption(row: &GroupRowDto) -> String {
    match &row.managed {
        Some(managed) if managed.archived => {
            format!(
                "Members come from {} · archived, no members",
                managed.source_label
            )
        }
        // Gauge lists members only to the group's owners and Super User, so an empty list
        // may just be hidden.
        Some(managed) if row.group.members.is_empty() => {
            format!("Members come from {}", managed.source_label)
        }
        Some(managed) => {
            let n = row.group.members.len();
            let members = if n == 1 {
                "1 member".to_string()
            } else {
                format!("{n} members")
            };
            format!("Members come from {} · {members}", managed.source_label)
        }
        None => row.group.description.clone(),
    }
}

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
            list_group_rows(filter).await
        },
    );
    // Switching tabs filters the loaded rows; it doesn't refetch.
    let tab = RwSignal::new(ALL_TAB.to_string());

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
                            <div data-testid="gauge-groups-tabs">
                                <TabList selected_value=tab>
                                    <Tab value=ALL_TAB>"All groups"</Tab>
                                    <Tab value=TEAMS_TAB>"Teams"</Tab>
                                </TabList>
                            </div>
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
                                    let teams_tab = move || tab.get() == TEAMS_TAB;
                                    let items = StoredValue::new(items);
                                    let has_teams = items.with_value(|rows| rows.iter().any(|row| row.managed.is_some()));
                                    let visible = move || {
                                        items.with_value(|rows| rows
                                            .iter()
                                            .filter(|row| !teams_tab() || row.managed.is_some())
                                            .cloned()
                                            .collect::<Vec<_>>())
                                    };
                                    view! {
                                        <Flex vertical=true gap=FlexGap::Small padding=SpacingInset::symmetric(SpacingHorizontal::M, SpacingVertical::S)>
                                            <Show when=teams_tab>
                                                <div data-testid="gauge-groups-teams-caption">
                                                <Caption1>
                                                    "These groups follow a team in another product, such as Polaron. Grant them access here or on any resource; change their members in that product."
                                                </Caption1>
                                                </div>
                                            </Show>
                                            <Show when=move || teams_tab() && !has_teams>
                                                <EmptyState message="No team groups yet." />
                                            </Show>
                                            <For each=visible key=|row| row.group.id.clone() let:row>
                                                {
                                                let is_managed = row.managed.is_some();
                                                let archived = row.managed.as_ref().is_some_and(|m| m.archived);
                                                let caption = group_caption(&row);
                                                let href = format!("/permission/groups/{}", row.group.id);
                                                let name = row.group.name.clone();
                                                view! {
                                                <>
                                                    <Flex
                                                        justify=FlexJustify::SpaceBetween
                                                        align=FlexAlign::Center
                                                        gap=FlexGap::Medium
                                                        padding=SpacingSize::Size120.inset()
                                                    >
                                                        <Flex vertical=true gap=FlexGap::Small>
                                                            <Flex align=FlexAlign::Center gap=FlexGap::Small>
                                                                <Show when=move || is_managed>
                                                                    <span data-testid="gauge-group-team-badge">
                                                                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Informative size=BadgeSize::Small>
                                                                            "Team"
                                                                        </Badge>
                                                                    </span>
                                                                </Show>
                                                                <Body1>{name}</Body1>
                                                                <Show when=move || archived>
                                                                    <span data-testid="gauge-group-archived-badge">
                                                                        <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Subtle size=BadgeSize::Small>
                                                                            "Archived team"
                                                                        </Badge>
                                                                    </span>
                                                                </Show>
                                                            </Flex>
                                                            <Caption1>{caption}</Caption1>
                                                        </Flex>
                                                        <div id="gauge-group-row-open">
                                                            <A href=href>
                                                                <Button appearance=ButtonAppearance::Subtle>
                                                                    "Open"
                                                                </Button>
                                                            </A>
                                                        </div>
                                                    </Flex>
                                                    <CardSectionBorder />
                                                </>
                                                }
                                                }
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
