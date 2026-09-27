use gauge::types::PermissionCreateInput;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;
use uf_product::components::{Body1, Caption1, Card};
use uf_product::components::{ContentContainer, SpacingSize, Title3};
use uf_product::primitives::{
    Button, ButtonAppearance, Field, Flex, FlexAlign, FlexGap, FlexJustify, Input, MessageBar,
    MessageBarIntent, Select, SelectAppearance, Textarea,
};

use crate::pages::step_up::spawn_with_step_up;
use crate::server::{create_permission, list_domains};

/// Form page for creating a new permission (name, description, domain); the owner
/// group is derived automatically from the actor's default owner group.
#[component]
// Create form + domain Select wiring; split later if modules stay cohesive.
#[allow(clippy::too_many_lines)]
pub fn PermissionCreatePage() -> impl IntoView {
    let navigate = use_navigate();
    let navigate_submit = navigate.clone();
    let name = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let domain_id = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let domains = Resource::new(|| (), |()| async move { list_domains(None).await });

    let on_submit = move |_| {
        if domain_id.get().trim().is_empty() {
            error.set(Some("Permission domain is required.".to_string()));
            return;
        }
        let navigate = navigate_submit.clone();
        let payload = PermissionCreateInput {
            name: name.get(),
            description: description.get(),
            owners_group_id: String::new(),
            domain_id: domain_id.get(),
        };
        let finish_ok = Callback::new(move |new_id: String| {
            navigate(
                &format!("/permission/permissions/{new_id}"),
                NavigateOptions::default(),
            );
        });
        spawn_with_step_up(error, finish_ok, move || {
            let payload = payload.clone();
            async move { create_permission(payload).await }
        });
    };

    view! {
        <div id="gauge-create-perm-page">
        <ContentContainer max_width="900px">
            <Flex vertical=true gap=FlexGap::Medium>
                <Card>
                    <Flex vertical=true gap=FlexGap::Small padding=SpacingSize::Size200.inset()>
                        <Title3>"Create Permission"</Title3>
                        <Caption1>
                            "Define a permission and assign the owner group that can maintain it."
                        </Caption1>
                    </Flex>
                </Card>

                <Card>
                    <div id="gauge-create-perm-form">
                    <Flex vertical=true gap=FlexGap::Medium padding=SpacingSize::Size200.inset()>
                        <Field label="Display name">
                            <Input bind=name />
                        </Field>
                        <Field label="Description">
                            <Textarea bind=description />
                        </Field>
                        <Field label="Domain">
                            <Select
                                bind=domain_id
                                appearance=SelectAppearance {
                                    disabled: Signal::derive(move || {
                                        domains.get().is_none() || matches!(domains.get(), Some(Err(_)))
                                    }),
                                    ..Default::default()
                                }
                            >
                                <option value="">"Select a domain"</option>
                                {move || {
                                    domains
                                        .get()
                                        .and_then(std::result::Result::ok)
                                        .unwrap_or_default()
                                        .into_iter()
                                        .map(|row| {
                                            view! {
                                                <option value={row.id}>{row.name}</option>
                                            }
                                        })
                                        .collect_view()
                                }}
                            </Select>
                            <Show when=move || domains.get().is_none()>
                                <Caption1>"Loading domains..."</Caption1>
                            </Show>
                            <Show
                                when=move || {
                                    matches!(domains.get(), Some(Ok(items)) if items.is_empty())
                                }
                            >
                                <Caption1>"No domains available. Create one first."</Caption1>
                            </Show>
                            <Show when=move || matches!(domains.get(), Some(Err(_)))>
                                <MessageBar intent=MessageBarIntent::Error>
                                    {move || {
                                        domains
                                            .get()
                                            .and_then(std::result::Result::err)
                                            .map(|e| e.to_string())
                                            .unwrap_or_default()
                                    }}
                                </MessageBar>
                            </Show>
                        </Field>
                        <Show when=move || error.get().is_some()>
                            <MessageBar intent=MessageBarIntent::Error>
                                {move || error.get().unwrap_or_default()}
                            </MessageBar>
                        </Show>
                        <Flex justify=FlexJustify::End align=FlexAlign::Center gap=FlexGap::Small>
                            <div id="gauge-create-perm-cancel">
                                <Button
                                    appearance=ButtonAppearance::Secondary
                                    on_click=Callback::new(move |_| navigate(crate::paths::PERMISSIONS, NavigateOptions::default()))
                                >
                                    "Cancel"
                                </Button>
                            </div>
                            <div id="gauge-create-perm-submit">
                                <Button appearance=ButtonAppearance::Primary on_click=Callback::new(on_submit)>
                                    "Create Permission"
                                </Button>
                            </div>
                        </Flex>
                        <Body1>
                            "Owner group is automatically assigned from your user context."
                        </Body1>
                    </Flex>
                    </div>
                </Card>
            </Flex>
        </ContentContainer>
        </div>
    }
}
