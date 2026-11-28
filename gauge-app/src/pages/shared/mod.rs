//! Shared UI pieces used across multiple Permission app pages.

/// [`give_access::GiveAccessDialog`] — give a permission to a person (with the group suggestion), or access to a person.
pub mod give_access;
/// [`query_state::use_query_signal`] / [`query_state::use_index_search`] — list state kept in the URL query and the `?name=` lookup.
pub mod query_state;
/// [`request_access::RequestAccessDialog`] — Request Access dialog and its `?request=` / `?for=` URL helpers.
pub mod request_access;

/// [`principal`] — kind label and icon for user, group and service-user rows.
pub mod principal;

/// [`resource_cards::ResourceBundleCard`] / [`resource_cards::GroupResourceAccessCard`] — a resource-scoped domain's resource, and the resources a group can reach.
pub mod resource_cards;

/// [`record_activity::RecordActivity`] — the Activity tab on a permission, group or domain.
pub mod record_activity;

/// [`remove_confirm::RemoveConfirmDialog`] — what someone loses and keeps, before the code dialog.
pub mod remove_confirm;

/// [`words`] — access paths, kinds and times in words.
pub mod words;

/// [`denied::DeniedPage`] — the locked page someone sees before a form they can't submit.
pub mod denied;

/// A browser-only resource from a key and a fetcher, like `Resource::new` but never run or
/// serialized during server rendering.
///
/// Pages server-render only their main record. Everything else (holders, activity, viewer
/// access) loads here after hydration, so slow reads never hold up the page and the server
/// and browser always start from the same empty state.
pub fn local_resource<K, T, Fut>(
    key: impl Fn() -> K + 'static,
    fetch: impl Fn(K) -> Fut + 'static,
) -> leptos::prelude::LocalResource<T>
where
    T: 'static,
    Fut: std::future::Future<Output = T> + 'static,
{
    leptos::prelude::LocalResource::new(move || fetch(key()))
}
