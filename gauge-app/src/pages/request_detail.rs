use gauge::types::PermissionRequestStatusDto;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use uf_product::components::{Body1, Caption1, Card};
use uf_product::components::{ContentContainer, SpacingSize, Title3};
use uf_product::primitives::{
    Button, ButtonAppearance, Flex, FlexGap, FlexJustify, MessageBar, MessageBarIntent,
};

use crate::pages::step_up::spawn_with_step_up;
use crate::server::{decide_permission_request, get_permission_request};

const fn status_label(status: &PermissionRequestStatusDto) -> &'static str {
    match status {
        PermissionRequestStatusDto::Pending => "PENDING",
        PermissionRequestStatusDto::Approved => "APPROVED",
        PermissionRequestStatusDto::Denied => "DENIED",
    }
}

/// Access request detail page: target/requestor/status summary, reason, and
/// approve/deny actions for eligible reviewers on pending requests.
#[component]
pub fn RequestDetailPage() -> impl IntoView {
    let params = use_params_map();
    let request_id = Memo::new(move |_| params.read().get("id").unwrap_or_default());
    let refresh = RwSignal::new(0u64);
    let error = RwSignal::new(None::<String>);

    let request = Resource::new(
        move || (request_id.get(), refresh.get()),
        move |(id, _)| async move { get_permission_request(id).await },
    );

    let decide = move |decision: gauge::types::PermissionRequestDecision| {
        let request_id = request_id.get();
        let finish_ok = Callback::new(move |_: ()| {
            refresh.update(|n| *n += 1);
            error.set(None);
        });
        spawn_with_step_up(error, finish_ok, move || {
            let request_id = request_id.clone();
            let decision = decision.clone();
            async move {
                decide_permission_request(gauge::types::PermissionRequestDecisionInput {
                    request_id,
                    decision,
                })
                .await
                .map(|_| ())
            }
        });
    };

    view! {
        <ContentContainer max_width="900px" data_testid="gauge-request-detail">
            <Transition fallback=move || view! { <Body1>"Loading request..."</Body1> }>
                {move || {
                    request.get().map(|result| match result {
                        Ok(Some(row)) => {
                            let is_pending = row.status == PermissionRequestStatusDto::Pending;
                            view! {
                                <Flex vertical=true gap=FlexGap::Medium>
                                    <Card>
                                        <Flex vertical=true gap=FlexGap::Small padding=SpacingSize::Size200.inset()>
                                            <Title3>"Permission Request"</Title3>
                                            <Caption1>{format!("ID: {}", row.id)}</Caption1>
                                        </Flex>
                                    </Card>
                                    <div id="gauge-request-detail-summary">
                                    <Card>
                                        <Flex vertical=true gap=FlexGap::Small padding=SpacingSize::Size200.inset()>
                                            <Body1>{format!("Target: {}", row.target_label)}</Body1>
                                            <Caption1>
                                                {format!("Requestor: {}", row.requestor_user_id)}
                                            </Caption1>
                                            <Caption1>
                                                {format!("Status: {}", status_label(&row.status))}
                                            </Caption1>
                                            <Caption1>
                                                {format!("Created: {}", row.created_at)}
                                            </Caption1>
                                            <Caption1>
                                                {format!("Updated: {}", row.updated_at)}
                                            </Caption1>
                                        </Flex>
                                    </Card>
                                    </div>
                                    <Card>
                                        <Flex vertical=true gap=FlexGap::Small padding=SpacingSize::Size200.inset()>
                                            <Title3>"Reason"</Title3>
                                            <Body1>{row.reason}</Body1>
                                        </Flex>
                                    </Card>
                                    <Show when=move || row.can_review && is_pending fallback=move || view! {
                                        <div id="gauge-request-deny"></div>
                                        <div id="gauge-request-approve"></div>
                                    }>
                                        <Card>
                                            <Flex justify=FlexJustify::End gap=FlexGap::Small padding=SpacingSize::Size200.inset()>
                                                <div id="gauge-request-deny">
                                                <Button
                                                    appearance=ButtonAppearance::Secondary
                                                    on_click=Callback::new(move |_| {
                                                        decide(gauge::types::PermissionRequestDecision::Deny)
                                                    })
                                                >
                                                    "Deny"
                                                </Button>
                                                </div>
                                                <div id="gauge-request-approve">
                                                <Button
                                                    appearance=ButtonAppearance::Primary
                                                    on_click=Callback::new(move |_| {
                                                        decide(gauge::types::PermissionRequestDecision::Approve)
                                                    })
                                                >
                                                    "Approve"
                                                </Button>
                                                </div>
                                            </Flex>
                                        </Card>
                                    </Show>
                                    <Show when=move || error.get().is_some()>
                                        <MessageBar intent=MessageBarIntent::Error>
                                            {move || error.get().unwrap_or_default()}
                                        </MessageBar>
                                    </Show>
                                </Flex>
                            }
                            .into_any()
                        }
                        Ok(None) => view! { <Body1>"Request not found."</Body1> }.into_any(),
                        Err(err) => view! {
                            <MessageBar intent=MessageBarIntent::Error>
                                {format!("Failed to load request: {err}")}
                            </MessageBar>
                        }
                        .into_any(),
                    })
                }}
            </Transition>
        </ContentContainer>
    }
}
