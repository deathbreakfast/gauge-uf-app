//! The resource kinds a host has told Gauge about, by prefix.

use std::sync::RwLock;

use gauge::resource_permissions::ResourceKindDescriptor;

/// One resource kind a host serves: its Gauge descriptor and where its resources live.
///
/// Gauge keeps no list of kinds (each app declares its descriptor in its own crate), so
/// the host registers each kind it serves at boot with [`register_resource_kind`]. The
/// access backend needs the descriptor to change roles; the domain and request pages use
/// `app` and `href` for "Open in Chronon".
#[derive(Clone, Copy, Debug)]
pub struct ResourceKindEntry {
    /// The kind's Gauge descriptor, as its app declares it.
    pub descriptor: ResourceKindDescriptor,
    /// The app that owns the resources, such as `"Chronon"`.
    pub app: &'static str,
    /// Path of one resource in its app, from the resource id.
    pub href: fn(&str) -> String,
    /// The app's admin group, shown as Default owner on code-declared resources.
    pub default_owner_group: Option<&'static str>,
}

static KINDS: RwLock<Vec<ResourceKindEntry>> = RwLock::new(Vec::new());

/// Tell Gauge's pages and access backend about a resource kind. Registering a prefix
/// again replaces the earlier entry. Call at host boot, before serving requests.
///
/// ```rust,ignore
/// gauge_app::register_resource_kind(gauge_app::ResourceKindEntry {
///     descriptor: chronon_app::permissions::CHRONON_JOB,
///     app: "Chronon",
///     href: |id| format!("/chronon/jobs/{id}"),
///     default_owner_group: Some("chronon.admins"),
/// });
/// ```
pub fn register_resource_kind(entry: ResourceKindEntry) {
    let mut kinds = KINDS
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    kinds.retain(|k| k.descriptor.prefix != entry.descriptor.prefix);
    kinds.push(entry);
}

/// The registered kind with `prefix`, if the host registered one.
#[must_use]
pub fn registered_resource_kind(prefix: &str) -> Option<ResourceKindEntry> {
    KINDS
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .iter()
        .find(|k| k.descriptor.prefix == prefix)
        .copied()
}
