//! Top-level route pages for Gauge: the Overview, people, groups, permissions, domains,
//! requests, service users and activity.

/// [`ActivityPage`] — Gauge's change log, for Gauge admins.
pub mod activity;
/// [`DomainCreatePage`] — permission domain creation form.
pub mod domain_create;
/// [`DomainDetailPage`] — permission domain detail/edit page.
pub mod domain_detail;
/// [`DomainsIndexPage`] — permission domain list.
pub mod domains_index;
/// [`GroupCreatePage`] — permission group creation form.
pub mod group_create;
/// [`GroupDetailPage`] — permission group detail/edit page.
pub mod group_detail;
/// [`GroupsIndexPage`] — permission group list.
pub mod groups_index;
/// [`OverviewPage`] — the landing page: what waits for you, counts, recent changes, your access.
pub mod overview;
/// [`PeopleIndexPage`] — people and service users, for Gauge admins.
pub mod people_index;
/// [`PermissionCreatePage`] — permission creation form.
pub mod permission_create;
/// [`PermissionDetailPage`] — permission detail/edit page.
pub mod permission_detail;
/// [`PermissionsIndexPage`] — permission list.
pub mod permissions_index;
/// [`PersonDetailPage`] — what one person can do and why.
pub mod person_detail;
/// Maps `PersonPicker` picks to Gauge principals and offers permission groups beside people.
pub mod principal_pick;
/// [`RequestDetailPage`] — access request detail/review page.
pub mod request_detail;
/// [`RequestsIndexPage`] — access request inbox.
pub mod requests_index;
/// [`ServiceUserDetailPage`] — one service user's groups and permissions.
pub mod service_user_detail;
/// Shared UI pieces used across multiple pages.
pub mod shared;
/// TOTP step-up retry helpers for Tier A mutations.
pub mod step_up;

pub use activity::ActivityPage;
pub use domain_create::DomainCreatePage;
pub use domain_detail::DomainDetailPage;
pub use domains_index::DomainsIndexPage;
pub use group_create::GroupCreatePage;
pub use group_detail::GroupDetailPage;
pub use groups_index::GroupsIndexPage;
pub use overview::OverviewPage;
pub use people_index::PeopleIndexPage;
pub use permission_create::PermissionCreatePage;
pub use permission_detail::PermissionDetailPage;
pub use permissions_index::PermissionsIndexPage;
pub use person_detail::PersonDetailPage;
pub use request_detail::RequestDetailPage;
pub use requests_index::RequestsIndexPage;
pub use service_user_detail::ServiceUserDetailPage;
