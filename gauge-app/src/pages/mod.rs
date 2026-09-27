//! Top-level route pages for the Permission app: domains, groups, permissions, and requests.

/// [`DomainCreatePage`] — permission domain creation form.
pub mod domain_create;
/// [`DomainDetailPage`] — permission domain detail/edit page.
pub mod domain_detail;
/// [`GroupCreatePage`] — permission group creation form.
pub mod group_create;
/// [`GroupDetailPage`] — permission group detail/edit page.
pub mod group_detail;
/// [`GroupsIndexPage`] — permission group list.
pub mod groups_index;
/// [`PermissionCreatePage`] — permission creation form.
pub mod permission_create;
/// [`PermissionDetailPage`] — permission detail/edit page.
pub mod permission_detail;
/// [`PermissionsIndexPage`] — permission list.
pub mod permissions_index;
/// [`RequestDetailPage`] — access request detail/review page.
pub mod request_detail;
/// [`RequestsIndexPage`] — access request inbox.
pub mod requests_index;
/// Shared UI pieces used across multiple pages.
pub mod shared;
/// TOTP step-up retry helpers for Tier A mutations.
pub mod step_up;

pub use domain_create::DomainCreatePage;
pub use domain_detail::DomainDetailPage;
pub use group_create::GroupCreatePage;
pub use group_detail::GroupDetailPage;
pub use groups_index::GroupsIndexPage;
pub use permission_create::PermissionCreatePage;
pub use permission_detail::PermissionDetailPage;
pub use permissions_index::PermissionsIndexPage;
pub use request_detail::RequestDetailPage;
pub use requests_index::RequestsIndexPage;
