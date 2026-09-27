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
//! - **Show History** — Permission and group detail pages open a dialog with a
//!   paginated Record History timeline ([`pages::shared::history_dialog::HistoryDialog`]).
//!   The page loader ([`server::get_gauge_history_page`]) requires a session and
//!   can-edit on the source (`gauge::service::actor_can_view_history_subject`);
//!   denied viewers see the same "Not authorized to view this history"
//!   MessageBar as the server error.
//!   [Get started](#show-history-on-detail-pages)
//!
//! - **Help spotlight tours** — Route-scoped Help steps under [`mod@help_steps`]
//!   (permissions, create flows, detail pages, groups, requests). Hosts enable
//!   `offering-help` (or `full`) so `HelpTourPlayer` mounts; call
//!   [`ensure_help_steps_linked`] so inventory links. Bare `/permission` matches
//!   inventory `/permission/permissions`.
//!   [Get started](#help-spotlight-tours)
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
//! ## Show History on detail pages
//!
//! Detail pages embed [`HistoryDialog`](pages::shared::history_dialog::HistoryDialog)
//! so editors can open a paginated audit timeline without leaving the subject.
//! The dialog loads when the request opens it; [`get_gauge_history_page`](server::get_gauge_history_page)
//! enforces session + can-edit before paging Record History rows.
//!
//! **Prerequisites:** Mounted [`PermissionRoutes`]; authenticated session; actor
//! who can edit the permission or group (or Super User).
//!
//! ```rust,ignore
//! use gauge_app::pages::shared::history_dialog::HistoryDialog;
//! use gauge_app::server::get_gauge_history_page;
//! use leptos::prelude::*;
//! use valence::RecordId;
//!
//! let subject_kind = Signal::derive(|| "permission".to_string());
//! let subject_id = Signal::derive(|| permission_id.get());
//! view! {
//!     <HistoryDialog
//!         subject_kind=subject_kind
//!         subject_id=subject_id
//!         trigger_id="gauge-show-history"
//!     />
//! }
//! // SSR path used by the timeline fetcher (can-edit ACL):
//! let page = get_gauge_history_page(
//!     0,
//!     25,
//!     RecordId::new("permission", &permission_id.get()),
//!     Some(vec!["permission_history".into()]),
//! )
//! .await?;
//! assert!(!page.items.is_empty() || !page.has_more);
//! assert!(!permission_id.get().is_empty());
//! ```
//!
//! On success the dialog shows Orbital timeline rows (relation grants as
//! Added/Removed with Avatar). Actors without can-edit get the deny MessageBar
//! (`Not authorized to view this history`) instead of rows.
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
/// Help spotlight tour step inventory for Permission routes.
pub mod help_steps;
/// Shell layout wrapping routed pages ([`PermissionLayout`]).
pub mod layout;
mod lazy_routes;
pub mod pages;
pub mod permissions;
pub mod server;
pub mod shell;

/// Playwright seed overrides — enable with Cargo feature `e2e-lab` (lab host only).
#[cfg(all(feature = "ssr", feature = "e2e-lab"))]
pub mod e2e_lab;

#[cfg(feature = "ssr")]
pub use bridge::wire_gauge_permissions;

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
