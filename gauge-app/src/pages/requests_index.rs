use gauge::types::{
    PermissionRequestRowDto, PermissionRequestStatusDto, PermissionRequestTargetKind,
};
use leptos::prelude::*;
use leptos_router::components::A;
use uf_product::components::{Body1, Caption1, Card, EmptyState, SkeletonItemSize};
use uf_product::components::{CardSectionBorder, ContentContainer, SpacingSize, Title3};
use uf_product::primitives::{
    Button, ButtonAppearance, Field, Flex, FlexAlign, FlexGap, FlexJustify, FlexWrap, Input,
    InputAppearance, MessageBar, MessageBarIntent, Select, SkeletonItem,
};

use crate::pages::shared::query_state::use_query_signal;
use crate::server::{list_my_permission_requests, list_review_permission_requests};

const fn request_target_label(kind: &PermissionRequestTargetKind) -> &'static str {
    match kind {
        PermissionRequestTargetKind::Permission => "Permission",
        PermissionRequestTargetKind::Group => "Group",
    }
}

const fn request_status_label(status: &PermissionRequestStatusDto) -> &'static str {
    match status {
        PermissionRequestStatusDto::Pending => "PENDING",
        PermissionRequestStatusDto::Approved => "APPROVED",
        PermissionRequestStatusDto::Denied => "DENIED",
    }
}

/// Requests-list filters, each kept in the URL (`q`, `status`, `for`). Empty means "any".
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct RequestFilters {
    /// Matches the requester, the target label, or the service user's handle.
    search: String,
    /// `pending`, `approved` or `denied`.
    status: String,
    /// `people` (no service user) or `service_users`.
    for_kind: String,
}

/// Handle of the service user a request is for, when it names one.
fn on_behalf_of_handle(row: &PermissionRequestRowDto) -> Option<&str> {
    row.on_behalf_of
        .as_ref()
        .map(|principal| principal.label.as_str())
}

fn request_matches(row: &PermissionRequestRowDto, filters: &RequestFilters) -> bool {
    let status_ok = match filters.status.as_str() {
        "pending" => row.status == PermissionRequestStatusDto::Pending,
        "approved" => row.status == PermissionRequestStatusDto::Approved,
        "denied" => row.status == PermissionRequestStatusDto::Denied,
        _ => true,
    };
    let for_ok = match filters.for_kind.as_str() {
        "people" => row.on_behalf_of.is_none(),
        "service_users" => row.on_behalf_of.is_some(),
        _ => true,
    };
    let needle = filters.search.trim().to_lowercase();
    let search_ok = needle.is_empty()
        || [
            Some(row.requestor_user_id.as_str()),
            Some(row.target_label.as_str()),
            on_behalf_of_handle(row),
        ]
        .into_iter()
        .flatten()
        .any(|field| field.to_lowercase().contains(&needle));
    status_ok && for_ok && search_ok
}

/// Caption under a request's target: kind, status, and "for @handle" when it names a service user.
fn request_caption(row: &PermissionRequestRowDto) -> String {
    let mut caption = format!(
        "{} - {}",
        request_target_label(&row.target_kind),
        request_status_label(&row.status)
    );
    if let Some(handle) = on_behalf_of_handle(row) {
        caption.push_str(&format!(" - for @{handle}"));
    }
    caption
}

/// One inbox's rows narrowed by `filters`, each with an Open link.
#[component]
fn RequestRows(
    rows: Vec<PermissionRequestRowDto>,
    filters: Signal<RequestFilters>,
    open_id: &'static str,
) -> impl IntoView {
    let visible = Signal::derive(move || {
        let filters = filters.get();
        rows.iter()
            .filter(|row| request_matches(row, &filters))
            .cloned()
            .collect::<Vec<_>>()
    });
    view! {
        <Show
            when=move || !visible.with(Vec::is_empty)
            fallback=|| view! { <EmptyState message="No requests match these filters." /> }
        >
            <Flex vertical=true gap=FlexGap::Small>
                <For each=move || visible.get() key=|row| row.id.clone() let:row>
                    {
                    let caption = request_caption(&row);
                    let href = format!("/permission/requests/{}", row.id);
                    view! {
                    <>
                        <Flex
                            justify=FlexJustify::SpaceBetween
                            align=FlexAlign::Center
                            gap=FlexGap::Small
                            padding=SpacingSize::Size120.inset()
                        >
                            <Flex vertical=true gap=FlexGap::Small>
                                <Body1>{row.target_label}</Body1>
                                <Caption1>{caption}</Caption1>
                            </Flex>
                            <div id=open_id>
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
        </Show>
    }
}

/// Access request inbox: a "Needs Review" queue for eligible reviewers alongside
/// the current actor's own submitted requests.
#[component]
// Dual inbox + decision actions live here; split later if modules stay cohesive.
#[allow(clippy::too_many_lines)]
pub fn RequestsIndexPage() -> impl IntoView {
    let search = use_query_signal("q", None);
    let status = use_query_signal("status", None);
    let for_kind = use_query_signal("for", None);
    let filters = Signal::derive(move || RequestFilters {
        search: search.get(),
        status: status.get(),
        for_kind: for_kind.get(),
    });
    let my_requests = Resource::new(
        || (),
        |()| async move { list_my_permission_requests().await },
    );
    let review_requests = Resource::new(
        || (),
        |()| async move { list_review_permission_requests().await },
    );

    view! {
        <div id="gauge-requests-page">
        <ContentContainer max_width="1100px" data_testid="gauge-requests-index">
            <Flex vertical=true gap=FlexGap::Medium>
                <Card>
                    <Flex vertical=true gap=FlexGap::Small padding=SpacingSize::Size200.inset()>
                        <Title3>"Permission Requests"</Title3>
                        <Caption1>
                            "Review incoming requests and track requests you have submitted."
                        </Caption1>
                    </Flex>
                </Card>

                <div id="gauge-requests-filters">
                <Card>
                    <Flex vertical=true gap=FlexGap::Small padding=SpacingSize::Size160.inset()>
                        <Field label="Search">
                            <Input
                                bind=search
                                appearance=InputAppearance::with_placeholder("Search by requester, target, or service user")
                            />
                        </Field>
                        <Flex gap=FlexGap::Medium wrap=FlexWrap::Wrap>
                            <div id="gauge-requests-filter-status">
                            <Field label="Status">
                                <Select bind=status>
                                    <option value="">"Any"</option>
                                    <option value="pending">"Pending"</option>
                                    <option value="approved">"Approved"</option>
                                    <option value="denied">"Denied"</option>
                                </Select>
                            </Field>
                            </div>
                            <div id="gauge-requests-filter-for">
                            <Field label="For">
                                <Select bind=for_kind>
                                    <option value="">"Anyone"</option>
                                    <option value="people">"People"</option>
                                    <option value="service_users">"Service users"</option>
                                </Select>
                            </Field>
                            </div>
                        </Flex>
                    </Flex>
                </Card>
                </div>

                <div id="gauge-requests-needs-review">
                <Card>
                    <Flex vertical=true gap=FlexGap::Small padding=SpacingSize::Size200.inset()>
                        <Title3>"Needs Review"</Title3>
                        <Transition fallback=move || view! {
                            <Flex vertical=true gap=FlexGap::Small>
                                {(0..2).map(|_| view! {
                                    <SkeletonItem
                                        size=Signal::from(SkeletonItemSize::S32)
                                        width="100%".to_string()
                                    />
                                }).collect_view()}
                            </Flex>
                        }>
                            {move || {
                                match review_requests.get() {
                                    None => view! {
                                        <Flex vertical=true gap=FlexGap::Small>
                                            {(0..2).map(|_| view! {
                                                <SkeletonItem
                                                    size=Signal::from(SkeletonItemSize::S32)
                                                    width="100%".to_string()
                                                />
                                            }).collect_view()}
                                        </Flex>
                                    }.into_any(),
                                    Some(Ok(rows)) if rows.is_empty() => view! {
                                        <div id="gauge-requests-review-open">
                                        <EmptyState message="No requests are waiting for your review." />
                                        </div>
                                    }.into_any(),
                                    Some(Ok(rows)) => view! {
                                        <RequestRows rows=rows filters=filters open_id="gauge-requests-review-open" />
                                    }.into_any(),
                                    Some(Err(err)) => view! {
                                        <MessageBar intent=MessageBarIntent::Error>
                                            {format!("Failed to load review requests: {err}")}
                                        </MessageBar>
                                    }.into_any(),
                                }
                            }}
                        </Transition>
                    </Flex>
                </Card>
                </div>

                <div id="gauge-requests-mine">
                <Card>
                    <Flex vertical=true gap=FlexGap::Small padding=SpacingSize::Size200.inset()>
                        <Title3>"My Requests"</Title3>
                        <Transition fallback=move || view! {
                            <Flex vertical=true gap=FlexGap::Small>
                                {(0..2).map(|_| view! {
                                    <SkeletonItem
                                        size=Signal::from(SkeletonItemSize::S32)
                                        width="100%".to_string()
                                    />
                                }).collect_view()}
                            </Flex>
                        }>
                            {move || {
                                match my_requests.get() {
                                    None => view! {
                                        <Flex vertical=true gap=FlexGap::Small>
                                            {(0..2).map(|_| view! {
                                                <SkeletonItem
                                                    size=Signal::from(SkeletonItemSize::S32)
                                                    width="100%".to_string()
                                                />
                                            }).collect_view()}
                                        </Flex>
                                    }.into_any(),
                                    Some(Ok(rows)) if rows.is_empty() => view! {
                                        <div id="gauge-requests-mine-open">
                                        <EmptyState message="You have not created any permission requests yet." />
                                        </div>
                                    }.into_any(),
                                    Some(Ok(rows)) => view! {
                                        <RequestRows rows=rows filters=filters open_id="gauge-requests-mine-open" />
                                    }.into_any(),
                                    Some(Err(err)) => view! {
                                        <MessageBar intent=MessageBarIntent::Error>
                                            {format!("Failed to load your requests: {err}")}
                                        </MessageBar>
                                    }.into_any(),
                                }
                            }}
                        </Transition>
                    </Flex>
                </Card>
                </div>
            </Flex>
        </ContentContainer>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use gauge::types::{PrincipalKind, PrincipalRefDto};

    use super::*;

    fn row(
        status: PermissionRequestStatusDto,
        for_handle: Option<&str>,
    ) -> PermissionRequestRowDto {
        PermissionRequestRowDto {
            id: "r1".into(),
            target_kind: PermissionRequestTargetKind::Group,
            target_id: "g1".into(),
            target_label: "Service user creators".into(),
            requestor_user_id: "requestor".into(),
            approver_user_id: None,
            reason: "need it".into(),
            status,
            created_at: String::new(),
            updated_at: String::new(),
            can_review: false,
            on_behalf_of: for_handle.map(|handle| PrincipalRefDto {
                kind: PrincipalKind::ServiceUser,
                id: "su1".into(),
                label: handle.into(),
                owner_label: None,
            }),
        }
    }

    fn filters(search: &str, status: &str, for_kind: &str) -> RequestFilters {
        RequestFilters {
            search: search.into(),
            status: status.into(),
            for_kind: for_kind.into(),
        }
    }

    #[test]
    fn empty_filters_match_everything() {
        let pending = row(PermissionRequestStatusDto::Pending, None);
        assert!(request_matches(&pending, &RequestFilters::default()));
    }

    #[test]
    fn status_filter_narrows() {
        let pending = row(PermissionRequestStatusDto::Pending, None);
        assert!(request_matches(&pending, &filters("", "pending", "")));
        assert!(!request_matches(&pending, &filters("", "approved", "")));
        assert!(!request_matches(&pending, &filters("", "denied", "")));
    }

    #[test]
    fn for_filter_splits_people_and_service_users() {
        let person = row(PermissionRequestStatusDto::Pending, None);
        let agent = row(PermissionRequestStatusDto::Pending, Some("spectra-agent"));
        assert!(request_matches(&person, &filters("", "", "people")));
        assert!(!request_matches(&person, &filters("", "", "service_users")));
        assert!(request_matches(&agent, &filters("", "", "service_users")));
        assert!(!request_matches(&agent, &filters("", "", "people")));
    }

    #[test]
    fn search_matches_requester_target_or_handle() {
        let agent = row(PermissionRequestStatusDto::Pending, Some("spectra-agent"));
        assert!(request_matches(&agent, &filters("REQUESTOR", "", "")));
        assert!(request_matches(&agent, &filters("creators", "", "")));
        assert!(request_matches(&agent, &filters("spectra", "", "")));
        assert!(!request_matches(&agent, &filters("deployers", "", "")));
    }

    #[test]
    fn caption_names_the_service_user() {
        let agent = row(PermissionRequestStatusDto::Approved, Some("spectra-agent"));
        assert_eq!(
            request_caption(&agent),
            "Group - APPROVED - for @spectra-agent"
        );
        let person = row(PermissionRequestStatusDto::Pending, None);
        assert_eq!(request_caption(&person), "Group - PENDING");
    }
}
