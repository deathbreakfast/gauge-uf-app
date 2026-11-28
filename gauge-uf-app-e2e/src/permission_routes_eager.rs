//! Eager `/permission` routes for the Playwright host.
//!
//! Production [`gauge_app::PermissionRoutes`] wraps leaf pages in `Lazy` for
//! wasm-split. Nested `Lazy` under `ParentRoute` still panics on
//! `hydrate_lazy` / `hydrate_body` in this Leptos pin, so the lab host mounts
//! the same page components without `Lazy`.

use gauge_app::{
    ActivityPage, DomainCreatePage, DomainDetailPage, DomainsIndexPage, GroupCreatePage,
    GroupDetailPage, GroupsIndexPage, OverviewPage, PeopleIndexPage, PermissionCreatePage,
    PermissionDetailPage, PermissionLayout, PermissionsIndexPage, PersonDetailPage,
    RequestDetailPage, RequestsIndexPage, ServiceUserDetailPage,
};
use leptos::prelude::*;
use leptos_router::{
    components::{ParentRoute, Route},
    path,
};

/// Same paths as [`gauge_app::PermissionRoutes`], without Lazy route views.
#[component(transparent)]
pub fn PermissionRoutesEager() -> impl leptos_router::MatchNestedRoutes + Clone {
    gauge_app::ensure_help_steps_linked();
    view! {
        <ParentRoute path=path!("permission") view=PermissionLayout>
            <Route path=path!("") view=OverviewPage />
            <Route path=path!("people") view=PeopleIndexPage />
            <Route path=path!("people/:id") view=PersonDetailPage />
            <Route path=path!("service-users/:id") view=ServiceUserDetailPage />
            <Route path=path!("activity") view=ActivityPage />
            <Route path=path!("domains") view=DomainsIndexPage />
            <Route path=path!("permissions") view=PermissionsIndexPage />
            <Route path=path!("permissions/:id") view=PermissionDetailPage />
            <Route path=path!("create-permission") view=PermissionCreatePage />
            <Route path=path!("create-domain") view=DomainCreatePage />
            <Route path=path!("domains/:id") view=DomainDetailPage />
            <Route path=path!("groups") view=GroupsIndexPage />
            <Route path=path!("groups/:id") view=GroupDetailPage />
            <Route path=path!("create-group") view=GroupCreatePage />
            <Route path=path!("requests") view=RequestsIndexPage />
            <Route path=path!("requests/:id") view=RequestDetailPage />
        </ParentRoute>
    }
    .into_inner()
}
