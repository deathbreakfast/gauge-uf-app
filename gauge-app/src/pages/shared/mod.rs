//! Shared UI pieces used across multiple Permission app pages.

/// [`history_dialog::HistoryDialog`] — Show History dialog with Record History timeline.
pub mod history_dialog;
/// [`query_state::use_query_signal`] / [`query_state::use_index_search`] — list state kept in the URL query and the `?name=` lookup.
pub mod query_state;
/// [`request_access::RequestAccessDialog`] — Request Access dialog and its `?request=` / `?for=` URL helpers.
pub mod request_access;

/// [`principal`] — kind label and icon for user, group and service-user rows.
pub mod principal;
