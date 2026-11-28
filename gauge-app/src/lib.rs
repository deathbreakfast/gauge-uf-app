#![recursion_limit = "256"]
//! Permission administration app routes and UI composition.
//!
//! Orbital ops UI for permission domains, permissions, groups, and
//! request/review under `/permission`. Domain rules and `actor_can` live in the
//! sibling crate `gauge`
//! ([gauge](https://github.com/unified-field-dev/gauge)).
//!
//! ## Features
//!
//! - **Permission admin routes** — Nested `/permission` route tree
//!   ([`PermissionRoutes`]) with layout auth gating, CRUD pages, and Higgs
//!   server wrappers. Mount once when the host router starts.
//!   [Get started](#mount-permission-admin-routes)
//!
//! - **Privilege mutation step-up** — Tier A admin mutations carry
//!   `#[uf_product_macros::server(..., step_up)]`. Pages retry after
//!   [`pages::step_up::spawn_with_step_up`] opens the TOTP prompt. Super User
//!   membership changes use [`pages::step_up::spawn_with_fresh_totp`].
//!   [Get started](#add-step-up-to-a-privilege-mutation)
//!
//! - **Activity tab** — Permission, group and domain pages list each change by name
//!   (When, Who, What changed) in an Activity tab
//!   ([`pages::shared::record_activity::RecordActivity`]), loaded through
//!   [`server::get_record_activity`]: Gauge's change log for Gauge admins, the record's
//!   history rows for its owners.
//!   [Get started](#activity-on-detail-pages)
//!
//! - **Help spotlight tours** — Route-scoped Help steps under [`mod@help_steps`]
//!   (permissions, create flows, detail pages, groups, requests). Hosts enable
//!   `offering-help` (or `full`) so `HelpTourPlayer` mounts; call
//!   [`ensure_help_steps_linked`] so inventory links. Bare `/permission` matches
//!   inventory `/permission/permissions`.
//!   [Get started](#help-spotlight-tours)
//!
//! - **Resource bundles and team groups** — A resource-scoped domain's page names its
//!   resource, links back to it, and lists each action's permission, its holders and the
//!   roles. Groups another product manages (Polaron teams) get a Teams tab, read-only
//!   members and an "Access through this group" card, and resource requests read as
//!   "Chronon job billing-rollup · Can view" ([`mod@resource_access`]).
//!   [Get started](#serve-resource-access-from-gauge)
//!
//! Hosts supply Valence, session chrome, and identity. Schemas, grant
//! resolution, and Super User bootstrap stay in `gauge`.
//!
//! ## Mount permission admin routes
//!
//! [`PermissionRoutes`] nests the full `/permission` subtree inside a host
//! Leptos `<Routes>` tree so operators can manage taxonomy, grants, and
//! requests. Mount during host router setup at startup, alongside other
//! `uf_app!` product routes — the macro registers launcher metadata and the
//! `/permission` inventory entry.
//!
//! **Prerequisites:** `ssr` (and hydrate matching the host); authenticated
//! session; `GaugeAdmin` for mutations
//! ([`permissions::GaugePermission`]); Valence + permission backend wiring
//! (see [`wire_gauge_permissions`] on SSR hosts).
//!
//! 1. Depend on `gauge-app` with `ssr` / `hydrate` aligned to the host.
//! 2. Call `wire_gauge_permissions` (SSR) so uf-product checks use `gauge`.
//! 3. Mount `<PermissionRoutes />` under the host `<Routes>`.
//!
//! ```rust,ignore
//! use gauge_app::PermissionRoutes;
//! use leptos::prelude::*;
//! use leptos_router::components::Routes;
//!
//! view! {
//!     <Routes fallback=|| "not found">
//!         <PermissionRoutes />
//!     </Routes>
//! }
//! ```
//!
//! On success `/permission` resolves to the permissions index; nested routes
//! cover groups, domains, and requests. Unauthenticated sessions are rejected
//! inside [`PermissionLayout`] (app bar stays visible). Admin mutations use
//! Higgs `#[uf_product_macros::server(permission = "GaugeAdmin")]` — see
//! [gauge `SECURITY.md`](https://github.com/unified-field-dev/gauge/blob/main/SECURITY.md).
//!
//! ## Add step-up to a privilege mutation
//!
//! Privilege mutations (grant/revoke, membership, permission delete/update,
//! request decide, domain owner/delete) expand `step_up` on the Higgs server
//! attribute so the call fails closed without a recent TOTP window. Pages wrap
//! the server fn with [`pages::step_up::spawn_with_step_up`]: on
//! `STEP_UP:step_up_required` / `_expired`, the helper opens the mounted
//! step-up dialog, verifies TOTP, then retries once. Super User membership
//! changes use [`pages::step_up::spawn_with_fresh_totp`] instead of the window
//! path.
//!
//! **Prerequisites:** Host mounted `lepton-auth-ui` `provide_step_up_controller`
//! + `StepUpDialog`; `ssr` on this crate; actor with `GaugeAdmin`.
//!
//! ```rust,ignore
//! use gauge_app::pages::step_up::spawn_with_step_up;
//! use gauge_app::server::add_permission_user;
//! use leptos::prelude::*;
//!
//! let error = RwSignal::new(None::<String>);
//! let finish_ok = Callback::new(move |_: ()| { /* refresh */ });
//! spawn_with_step_up(error, finish_ok, move || {
//!     let permission_id = permission_id.clone();
//!     let user_id = user_id.clone();
//!     async move { add_permission_user(permission_id, user_id).await }
//! });
//! // server: #[uf_product_macros::server(permission = "GaugeAdmin", step_up)]
//! ```
//!
//! On `STEP_UP:step_up_required` / `_expired`, helper opens dialog, verifies
//! TOTP, retries once; assert completed.
//!
//! ## Activity on detail pages
//!
//! Detail pages put [`RecordActivity`](pages::shared::record_activity::RecordActivity)
//! in their Activity tab. It loads when the tab opens, in its own `Suspense`, so the page
//! around it never falls back.
//!
//! **Prerequisites:** Mounted [`PermissionRoutes`]; authenticated session; a Gauge admin
//! or an owner of the record.
//!
//! ```rust,ignore
//! use gauge::types::{RecordKind, RecordRef};
//! use gauge_app::pages::shared::record_activity::RecordActivity;
//! use leptos::prelude::*;
//!
//! let record = RecordRef { kind: RecordKind::Group, id: group_id, label: group_name };
//! view! { <RecordActivity record=record /> }
//! ```
//!
//! Each row names who made the change and what changed ("Added Ravi Menon to the group").
//! Anyone else sees "You don't have access" instead of rows.
//!
//! ## Help spotlight tours
//!
//! Permission ships Help spotlight steps for each admin route (permissions
//! index, create permission, permission detail, create domain, groups index,
//! create group, group detail, requests index, request detail). Hosts that
//! enable `offering-help` (or `full`) mount `HelpTourPlayer`; call
//! [`ensure_help_steps_linked`] at route mount so `inventory` submissions from
//! [`mod@help_steps`] are retained. Inventory route `/permission/permissions`
//! also matches bare `/permission` (same page).
//!
//! **Prerequisites:** `uf-help` hydrate/ssr features on this crate; product host
//! with Help player mounted; authenticated session for Valence visit tracking.
//!
//! ```rust,ignore
//! use gauge_app::{ensure_help_steps_linked, PermissionRoutes};
//!
//! ensure_help_steps_linked();
//! // Mount PermissionRoutes inside the host <Routes> tree as usual.
//! ```
//!
//! On success, visiting `/permission` (and other Permission paths) can show
//! pending spotlight steps. Replay restarts the tour for the current route via
//! the Help menu.
//!
//! Next: page modules under [`pages`], or domain APIs in `gauge`.
//!
//! ## Serve resource access from Gauge
//!
//! Apps show who can use one of their resources through uf-product's Access card, and
//! Gauge answers it: `GaugeResourceAccessBackend` reads the resource's bundle and changes
//! roles as the signed-in caller. Gauge keeps no list of resource kinds, so the host
//! registers each kind it serves with `register_resource_kind`, which also gives Gauge's
//! domain and request pages the "Open in {app}" link. Do both at boot, before serving.
//!
//! **Prerequisites:** `ssr`; the resource's bundle ensured at boot or on create (see
//! `gauge::resource_permissions`).
//!
//! ```rust,ignore
//! use std::sync::Arc;
//!
//! gauge_app::register_resource_kind(gauge_app::ResourceKindEntry {
//!     descriptor: chronon_app::permissions::CHRONON_JOB,
//!     app: "Chronon",
//!     href: |id| format!("/chronon/jobs/{id}"),
//!     default_owner_group: Some("chronon.admins"),
//! });
//! // In the per-request context, beside wire_gauge_permissions():
//! uf_product::access::provide_resource_access_backend(Arc::new(
//!     gauge_app::GaugeResourceAccessBackend,
//! ));
//! ```
//!
//! On success the app's Access card lists the resource's roles, and the bundle's domain
//! page in Gauge links back to the resource. An unregistered kind still shows on the
//! domain page, without the link, and the backend refuses role changes for it.
//!
//! ## Feature flags
//!
//! | Flag | What it enables |
//! |------|-----------------|
//! | `ssr` | Server functions, Valence-backed pages, `wire_gauge_permissions` |
//! | `hydrate` | WASM client hydrate graph for the same UI |
//! | `e2e-lab` | Playwright seed overrides (`e2e_lab`); enable only from `gauge-uf-app-e2e` |
//!
//! ## Module map
//!
//! | Concern | Where |
//! |---------|--------|
//! | Mount + Features guide | this crate root |
//! | Higgs server wrappers | [`mod@server`] |
//! | Resource bundle and team group DTOs | [`mod@resource_access`] |
//! | Resource access backend, kind registry (SSR) | `GaugeResourceAccessBackend`, `register_resource_kind` |
//! | Pages / routes | [`pages`], lazy re-exports on this root |
//! | Help spotlight inventory | [`mod@help_steps`]; call [`ensure_help_steps_linked`] |
//! | Shell / layout | [`shell`], [`PermissionLayout`] |
//! | Domain `actor_can`, schemas, service | sibling crate `gauge` |
//!
//! ## Examples
//!
//! Mount [`PermissionRoutes`] per [Mount permission admin routes](#mount-permission-admin-routes).
//! Domain rules and `actor_can` live in sibling crate `gauge`; workspace example
//! `embedded-gauge-host` shows bootstrap without this UI.
//!
//! Run `cargo test -p gauge --test permission_domain_contract` and
//! `cargo test -p gauge --test permission_flows_integration` for the service APIs
//! these pages wrap.
//!
//! Related: [`PermissionLayout`], [`shell`], [`mod@server`], page and lazy route
//! re-exports on this crate root.

#![allow(missing_docs)] // uf_app! / routes macros emit undocumented associated items

use leptos::prelude::*;
use leptos_router::{
    components::{ParentRoute, Route},
    path, Lazy,
};
use uf_product_macros::uf_app;

mod bridge;
/// Shared page pieces such as the error bar.
pub mod components;
/// Help spotlight tour step inventory for Permission routes.
pub mod help_steps;
/// Shell layout wrapping routed pages ([`PermissionLayout`]).
pub mod layout;
mod lazy_routes;
pub mod pages;
pub mod permissions;
pub mod resource_access;
pub mod server;
pub mod shell;

/// Playwright seed overrides — enable with Cargo feature `e2e-lab` (lab host only).
#[cfg(all(feature = "ssr", feature = "e2e-lab"))]
pub mod e2e_lab;

#[cfg(feature = "ssr")]
pub use bridge::{
    register_resource_kind, registered_resource_kind, wire_gauge_permissions,
    GaugeResourceAccessBackend, ResourceKindEntry,
};

pub use help_steps::ensure_help_steps_linked;
pub use layout::PermissionLayout;
pub use lazy_routes::{
    prefetch_family, DomainCreateRoute, DomainDetailRoute, GroupCreateRoute, GroupDetailRoute,
    GroupsIndexRoute, PermissionCreateRoute, PermissionDetailRoute, PermissionsIndexRoute,
    RequestDetailRoute, RequestsIndexRoute,
};
pub use pages::{
    DomainCreatePage, DomainDetailPage, GroupCreatePage, GroupDetailPage, GroupsIndexPage,
    PermissionCreatePage, PermissionDetailPage, PermissionsIndexPage, RequestDetailPage,
    RequestsIndexPage,
};

uf_app! {
    name: "Permission",
    id: "permission",
    description: "Permission and group administration",
    icon: "🔐",
    version: "0.1.0",
    routes: PermissionRoutes,
    route_path: "/permission",
    repository: "https://github.com/unified-field-dev/gauge-uf-app",
    permission_manifest: permissions::GaugePermission,
}

/// Route tree for the Permission app: index/detail/create pages for permissions,
/// groups, domains, and requests. Auth gating lives inside [`PermissionLayout`] so the
/// app bar stays visible when access is denied.
#[allow(missing_docs)]
#[orbital_macros::orbital_routes_extract]
#[component(transparent)]
pub fn PermissionRoutes() -> impl leptos_router::MatchNestedRoutes + Clone {
    crate::help_steps::ensure_help_steps_linked();
    view! {
        <ParentRoute path=path!("permission") view=PermissionLayout>
            <Route path=path!("") view={Lazy::<PermissionsIndexRoute>::new()} />
            <Route path=path!("permissions") view={Lazy::<PermissionsIndexRoute>::new()} />
            <Route path=path!("permissions/:id") view={Lazy::<PermissionDetailRoute>::new()} />
            <Route path=path!("create-permission") view={Lazy::<PermissionCreateRoute>::new()} />
            <Route path=path!("create-domain") view={Lazy::<DomainCreateRoute>::new()} />
            <Route path=path!("domains/:id") view={Lazy::<DomainDetailRoute>::new()} />
            <Route path=path!("groups") view={Lazy::<GroupsIndexRoute>::new()} />
            <Route path=path!("groups/:id") view={Lazy::<GroupDetailRoute>::new()} />
            <Route path=path!("create-group") view={Lazy::<GroupCreateRoute>::new()} />
            <Route path=path!("requests") view={Lazy::<RequestsIndexRoute>::new()} />
            <Route path=path!("requests/:id") view={Lazy::<RequestDetailRoute>::new()} />
        </ParentRoute>
    }
    .into_inner()
}
