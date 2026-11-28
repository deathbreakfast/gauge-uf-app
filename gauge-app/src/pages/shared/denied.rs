//! What a page shows someone without the access it needs, before they fill anything in.

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;
use uf_product::components::{EmptyState, EmptyStateCallToAction};
use uf_product::primitives::{
    Breadcrumb, BreadcrumbButton, BreadcrumbDivider, BreadcrumbItem, Button, ButtonAppearance,
    Flex, FlexGap,
};

/// Opens the GaugeAdmin permission with its Request access dialog.
pub const ASK_FOR_GAUGE_ADMIN_HREF: &str = "/permission/permissions?name=GaugeAdmin&request=1";

/// A locked empty state under a breadcrumb, with one way forward.
#[component]
pub fn DeniedPage(
    /// Headline ("Creating permissions needs GaugeAdmin").
    heading: &'static str,
    /// What to do instead.
    text: &'static str,
    /// The button ("Ask for GaugeAdmin").
    action: &'static str,
    /// Where the button goes.
    action_href: &'static str,
    /// The list this page belongs to: `(label, href)`.
    parent: (&'static str, &'static str),
    /// This page's crumb.
    current: &'static str,
) -> impl IntoView {
    let navigate = use_navigate();
    view! {
        <div data-testid="gauge-denied-page">
            <Flex vertical=true gap=FlexGap::Medium>
                <Breadcrumb>
                    <BreadcrumbItem>
                        <BreadcrumbButton href=parent.1.to_string()>{parent.0}</BreadcrumbButton>
                    </BreadcrumbItem>
                    <BreadcrumbDivider />
                    <BreadcrumbItem>
                        <BreadcrumbButton current=true>{current}</BreadcrumbButton>
                    </BreadcrumbItem>
                </Breadcrumb>
                <EmptyState
                    message=heading
                    description=text
                    icon=icondata_core::Icon::from(icondata::AiLockOutlined)
                >
                    <EmptyStateCallToAction slot:call_to_action>
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click=Callback::new(move |_| navigate(action_href, NavigateOptions::default()))
                        >
                            {action}
                        </Button>
                    </EmptyStateCallToAction>
                </EmptyState>
            </Flex>
        </div>
    }
}
