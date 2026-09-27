//! Lazy-loaded route views for WASM code-splitting (`cargo leptos --split`).

use leptos::prelude::*;
use leptos_router::{lazy_route, LazyRoute};

use crate::pages::{
    DomainCreatePage, DomainDetailPage, GroupCreatePage, GroupDetailPage, GroupsIndexPage,
    PermissionCreatePage, PermissionDetailPage, PermissionsIndexPage, RequestDetailPage,
    RequestsIndexPage,
};

/// Prefetch the permission family WASM chunk (leaf pages share split modules).
pub async fn prefetch_family() {
    PermissionsIndexRoute::preload().await;
}

/// Lazy `/permission` and `/permission/permissions` index.
#[derive(Clone, Copy, Debug, Default)]
pub struct PermissionsIndexRoute;

#[lazy_route]
impl LazyRoute for PermissionsIndexRoute {
    fn data() -> Self {
        Self
    }

    fn view(_this: Self) -> AnyView {
        view! { <PermissionsIndexPage /> }.into_any()
    }
}

/// Lazy `/permission/permissions/:id` detail page.
#[derive(Clone, Copy, Debug, Default)]
pub struct PermissionDetailRoute;

#[lazy_route]
impl LazyRoute for PermissionDetailRoute {
    fn data() -> Self {
        Self
    }

    fn view(_this: Self) -> AnyView {
        view! { <PermissionDetailPage /> }.into_any()
    }
}

/// Lazy `/permission/create-permission` page.
#[derive(Clone, Copy, Debug, Default)]
pub struct PermissionCreateRoute;

#[lazy_route]
impl LazyRoute for PermissionCreateRoute {
    fn data() -> Self {
        Self
    }

    fn view(_this: Self) -> AnyView {
        view! { <PermissionCreatePage /> }.into_any()
    }
}

/// Lazy `/permission/create-domain` page.
#[derive(Clone, Copy, Debug, Default)]
pub struct DomainCreateRoute;

#[lazy_route]
impl LazyRoute for DomainCreateRoute {
    fn data() -> Self {
        Self
    }

    fn view(_this: Self) -> AnyView {
        view! { <DomainCreatePage /> }.into_any()
    }
}

/// Lazy `/permission/domains/:id` detail page.
#[derive(Clone, Copy, Debug, Default)]
pub struct DomainDetailRoute;

#[lazy_route]
impl LazyRoute for DomainDetailRoute {
    fn data() -> Self {
        Self
    }

    fn view(_this: Self) -> AnyView {
        view! { <DomainDetailPage /> }.into_any()
    }
}

/// Lazy `/permission/groups` index.
#[derive(Clone, Copy, Debug, Default)]
pub struct GroupsIndexRoute;

#[lazy_route]
impl LazyRoute for GroupsIndexRoute {
    fn data() -> Self {
        Self
    }

    fn view(_this: Self) -> AnyView {
        view! { <GroupsIndexPage /> }.into_any()
    }
}

/// Lazy `/permission/groups/:id` detail page.
#[derive(Clone, Copy, Debug, Default)]
pub struct GroupDetailRoute;

#[lazy_route]
impl LazyRoute for GroupDetailRoute {
    fn data() -> Self {
        Self
    }

    fn view(_this: Self) -> AnyView {
        view! { <GroupDetailPage /> }.into_any()
    }
}

/// Lazy `/permission/create-group` page.
#[derive(Clone, Copy, Debug, Default)]
pub struct GroupCreateRoute;

#[lazy_route]
impl LazyRoute for GroupCreateRoute {
    fn data() -> Self {
        Self
    }

    fn view(_this: Self) -> AnyView {
        view! { <GroupCreatePage /> }.into_any()
    }
}

/// Lazy `/permission/requests` index.
#[derive(Clone, Copy, Debug, Default)]
pub struct RequestsIndexRoute;

#[lazy_route]
impl LazyRoute for RequestsIndexRoute {
    fn data() -> Self {
        Self
    }

    fn view(_this: Self) -> AnyView {
        view! { <RequestsIndexPage /> }.into_any()
    }
}

/// Lazy `/permission/requests/:id` detail page.
#[derive(Clone, Copy, Debug, Default)]
pub struct RequestDetailRoute;

#[lazy_route]
impl LazyRoute for RequestDetailRoute {
    fn data() -> Self {
        Self
    }

    fn view(_this: Self) -> AnyView {
        view! { <RequestDetailPage /> }.into_any()
    }
}
