use gauge::types::PermissionDomainCreateInput;
use leptos::prelude::*;
use leptos::task::spawn_local_scoped;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;
use uf_product::components::{Body1, Caption1, Card};
use uf_product::components::{ContentContainer, SpacingSize, Title3};
use uf_product::primitives::{
    Button, ButtonAppearance, Field, Flex, FlexAlign, FlexGap, FlexJustify, Input, MessageBar,
    MessageBarIntent, Textarea,
};

use crate::server::create_domain;

/// Form page for creating a new permission domain (taxonomy root).
#[component]
pub fn DomainCreatePage() -> impl IntoView {
    let navigate = use_navigate();
    let navigate_submit = navigate.clone();
    let name = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);

    let on_submit = move |_| {
        if name.get().trim().is_empty() {
            error.set(Some("Domain name is required.".to_string()));
            return;
        }
        let navigate = navigate_submit.clone();
        let payload = PermissionDomainCreateInput {
            name: name.get(),
            description: description.get(),
        };
        spawn_local_scoped(async move {
            match create_domain(payload).await {
                Ok(id) => navigate(
                    &format!("/permission/domains/{id}"),
                    NavigateOptions::default(),
                ),
                Err(err) => error.set(Some(err.to_string())),
            }
        });
    };

    view! {
        <div id="gauge-create-domain-page">
        <ContentContainer max_width="900px">
            <Flex vertical=true gap=FlexGap::Medium>
                <Card>
                    <Flex vertical=true gap=FlexGap::Small padding=SpacingSize::Size200.inset()>
                        <Title3>"Create Permission Domain"</Title3>
                        <Caption1>
                            "Create a permission domain used to group related permission definitions. You become the first owner."
                        </Caption1>
                    </Flex>
                </Card>

                <Card>
                    <div id="gauge-create-domain-form">
                    <Flex vertical=true gap=FlexGap::Medium padding=SpacingSize::Size200.inset()>
                        <Field label="Domain name">
                            <Input bind=name />
                        </Field>
                        <Field label="Description">
                            <Textarea bind=description />
                        </Field>
                        <Show when=move || error.get().is_some()>
                            <MessageBar intent=MessageBarIntent::Error>
                                {move || error.get().unwrap_or_default()}
                            </MessageBar>
                        </Show>
                        <Flex justify=FlexJustify::End align=FlexAlign::Center gap=FlexGap::Small>
                            <div id="gauge-create-domain-cancel">
                                <Button
                                    appearance=ButtonAppearance::Secondary
                                    on_click=Callback::new(move |_| navigate(crate::paths::PERMISSIONS, NavigateOptions::default()))
                                >
                                    "Cancel"
                                </Button>
                            </div>
                            <div id="gauge-create-domain-submit">
                                <Button appearance=ButtonAppearance::Primary on_click=Callback::new(on_submit)>
                                    "Create Domain"
                                </Button>
                            </div>
                        </Flex>
                        <Body1>
                            "You can assign this domain to new and existing permissions."
                        </Body1>
                    </Flex>
                    </div>
                </Card>
            </Flex>
        </ContentContainer>
        </div>
    }
}
