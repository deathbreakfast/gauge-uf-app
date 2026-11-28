//! Mount permission admin pages for Playwright.

use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;
use uf_integrations::{
    provide_shell_auth_menu, HostAuthMenu, ShellAppBar, ShellAuthMenu, UnifiedFieldAppBar,
    UnifiedFieldShellLayout,
};
use uf_product::components::ContentContainer;
use uf_product::primitives::{Body1, Flex, FlexAlign, FlexGap, Link, Title3};
use uf_product::{orbital_shell, OrbitalTemplate};

use crate::gate_demos::E2eAuthProvider;
use crate::harness_auth_menu::HarnessAuthMenu;
use crate::permission_routes_eager::PermissionRoutesEager;

/// SSR document shell.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    orbital_shell(options, || view! { <App/> })
}

/// Root: harness auth + eager permission routes (same pages as PermissionRoutes).
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    gauge_app::ensure_help_steps_linked();
    uf_help::ensure_linked();
    #[cfg(feature = "ssr")]
    {
        provide_context(crate::e2e_higgs_config());
        gauge_app::wire_gauge_permissions();
    }
    provide_shell_auth_menu(|| view! { <HarnessAuthMenu /> });
    // Provide the step-up controller at the root, so the pages and the dialog that
    // lepton-shell's avatar menu mounts share it. Left to the menu, it lands in the menu's
    // own subtree, where the pages can't reach it.
    lepton_auth_ui::provide_step_up_controller();

    view! {
        <OrbitalTemplate>
            <Stylesheet id="leptos" href="/pkg/gauge-uf-app-e2e.css"/>
            <Title text="gauge-uf-app e2e"/>
            <E2eAuthProvider>
                <Router>
                    <Routes fallback=|| view! { <p>"Not found"</p> }>
                        <Route path=path!("/") view=HomePage/>
                        <PermissionRoutesEager />
                    </Routes>
                </Router>
            </E2eAuthProvider>
        </OrbitalTemplate>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <UnifiedFieldShellLayout>
            <ShellAppBar slot>
                <UnifiedFieldAppBar app_name="Gauge e2e".to_string()>
                    <ShellAuthMenu slot:auth_menu>
                        <HostAuthMenu />
                    </ShellAuthMenu>
                </UnifiedFieldAppBar>
            </ShellAppBar>
            <ContentContainer max_width="900px" data_testid="gauge-e2e-home">
                <Flex vertical=true gap=FlexGap::Medium align=FlexAlign::Start>
                    <Title3>"gauge-uf-app e2e"</Title3>
                    <Body1>"PermissionRoutes host for Playwright."</Body1>
                    <Link href="/permission">"Open /permission"</Link>
                </Flex>
            </ContentContainer>
        </UnifiedFieldShellLayout>
    }
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    // Eager routes (see permission_routes_eager) — hydrate_body is enough.
    leptos::mount::hydrate_body(App);
    uf_product::hide_boot_loader();
}
