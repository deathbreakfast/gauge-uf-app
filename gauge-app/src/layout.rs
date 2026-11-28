use lepton_shell::AppBarUserMenu;
use leptos::prelude::*;
use leptos_router::components::Outlet;
use uf_integrations::{
    ShellAppBar, ShellAuthMenu, ShellLeftNav, UnifiedFieldAppBar, UnifiedFieldShellLayout,
};
use uf_product::components::{
    Navigation, NavigationBody, NavigationConfig, NavigationLink, NavigationMaterial,
};
use uf_product::routes::RequireAuthenticated;

use uf_product::primitives::{Badge, BadgeAppearance, BadgeColor, BadgeSize};

use crate::paths;
use crate::server::viewer::{use_viewer_access, viewer_now};
use crate::AppMetadata;

/// Shell layout for Gauge: app bar + left nav wrapping the routed page [`Outlet`].
#[component]
pub fn PermissionLayout() -> impl IntoView {
    let app_name = AppMetadata::name().to_string();
    let selected_value = RwSignal::new(None::<String>);
    let open_categories = RwSignal::new(Vec::<String>::new());
    // Read once here and shared as context, so pages reuse this load.
    let viewer = use_viewer_access();
    let is_admin = Signal::derive(move || viewer_now(viewer).gauge_admin);
    let waiting = Signal::derive(move || viewer_now(viewer).waiting_count);

    view! {
        <div data-testid="permission-app-root">
        <UnifiedFieldShellLayout>
            <ShellAppBar slot>
                <UnifiedFieldAppBar
                    app_name=app_name
                    app_id=AppMetadata::id()
                    homepage_url="/".to_string()
                >
                    <ShellAuthMenu slot:auth_menu>
                        <AppBarUserMenu />
                    </ShellAuthMenu>
                </UnifiedFieldAppBar>
            </ShellAppBar>
            <ShellLeftNav slot>
                <Navigation config=NavigationConfig::new().with_selected_value(selected_value).with_open_categories(open_categories)>
                    <NavigationMaterial slot />
                    <NavigationBody slot>
                        <div id="permission-left-nav">
                            <NavigationLink path="/permission" value="/permission" icon=icondata::AiHomeOutlined exact=true test_id="nav-overview">"Overview"</NavigationLink>
                            <Show when=move || is_admin.get()>
                                <NavigationLink path="/permission/people" value="/permission/people" icon=icondata::AiUserOutlined test_id="nav-people">"People"</NavigationLink>
                            </Show>
                            <NavigationLink path=paths::GROUPS value=paths::GROUPS icon=icondata::AiTeamOutlined test_id="nav-groups">"Groups"</NavigationLink>
                            <NavigationLink path=paths::PERMISSIONS value=paths::PERMISSIONS icon=icondata::AiSafetyCertificateOutlined test_id="nav-permissions">"Permissions"</NavigationLink>
                            <Show when=move || is_admin.get()>
                                <NavigationLink path="/permission/domains" value="/permission/domains" icon=icondata::AiAppstoreOutlined test_id="nav-domains">"Domains"</NavigationLink>
                            </Show>
                            <NavigationLink path="/permission/requests" value="/permission/requests" icon=icondata::AiBellOutlined test_id="nav-requests">
                                "Requests"
                                <Show when=move || { waiting.get() > 0 }>
                                    <span data-testid="nav-requests-count">
                                        <Badge appearance=BadgeAppearance::Filled color=BadgeColor::Brand size=BadgeSize::Small>
                                            {move || waiting.get().to_string()}
                                        </Badge>
                                    </span>
                                </Show>
                            </NavigationLink>
                            <Show when=move || is_admin.get()>
                                <NavigationLink path="/permission/activity" value="/permission/activity" icon=icondata::AiHistoryOutlined test_id="nav-activity">"Activity"</NavigationLink>
                            </Show>
                        </div>
                    </NavigationBody>
                </Navigation>
            </ShellLeftNav>
            <RequireAuthenticated requires_email_verification=true>
                <Outlet />
            </RequireAuthenticated>
        </UnifiedFieldShellLayout>
        </div>
    }
}
