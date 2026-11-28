//! What the signed-in person may do in Gauge, read once per page load.
//!
//! Pages offer admin and owner buttons only when [`ViewerAccess`] allows them. The server
//! keeps its own checks on every call; this only stops a page from offering what the server
//! would refuse.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// The signed-in person's standing in Gauge.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewerAccess {
    /// They hold `GaugeAdmin`.
    pub gauge_admin: bool,
    /// Ids of the groups they own, directly or through an owner group.
    pub owned_groups: Vec<String>,
    /// Ids of the permissions they own, directly or through an owner group.
    pub owned_permissions: Vec<String>,
    /// Pending requests waiting for their answer.
    pub waiting_count: usize,
}

impl ViewerAccess {
    /// Whether they may change `group_id`'s members (its owners and Gauge admins).
    #[must_use]
    pub fn can_manage_group(&self, group_id: &str) -> bool {
        self.gauge_admin || self.owned_groups.iter().any(|g| g == group_id)
    }

    /// Whether they may change `permission_id`'s allow list (its owners and Gauge admins).
    #[must_use]
    pub fn can_manage_permission(&self, permission_id: &str) -> bool {
        self.gauge_admin || self.owned_permissions.iter().any(|p| p == permission_id)
    }
}

/// The signed-in person's [`ViewerAccess`].
#[uf_product_macros::server]
pub async fn get_viewer_access() -> Result<ViewerAccess, ServerFnError> {
    use gauge::types::{PermissionRequestStatusDto, RecordKind};

    let ctx = higgs::Higgs::from_request().await?;
    if ctx.session_user_id().is_none() {
        return Err(ServerFnError::new("Authentication required"));
    }
    let v = ctx
        .valence()
        .map_err(|e| ServerFnError::new(format!("Failed to build Valence: {e}")))?;
    let gauge_admin = gauge::service::actor_is_gauge_admin(&v)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to load your access: {e}")))?;
    let mine = gauge::service::principal_access_self(&v)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to load your access: {e}")))?;
    let owned = |kind: RecordKind| {
        mine.owns
            .iter()
            .filter(|r| r.kind == kind)
            .map(|r| r.id.clone())
            .collect::<Vec<_>>()
    };
    let waiting_count = gauge::service::list_permission_requests_for_review(&v)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to load requests: {e}")))?
        .iter()
        .filter(|r| r.status == PermissionRequestStatusDto::Pending)
        .count();
    Ok(ViewerAccess {
        gauge_admin,
        owned_groups: owned(RecordKind::Group),
        owned_permissions: owned(RecordKind::Permission),
        waiting_count,
    })
}

/// The page's viewer access: the one already in context, or a new resource provided as
/// context so the rest of the page reads the same load.
pub fn use_viewer_access() -> Resource<Result<ViewerAccess, ServerFnError>> {
    if let Some(existing) = use_context::<Resource<Result<ViewerAccess, ServerFnError>>>() {
        return existing;
    }
    let resource = Resource::new(|| (), |()| get_viewer_access());
    provide_context(resource);
    resource
}

/// The loaded viewer access, or the default (no admin, owns nothing) while it loads or
/// when it failed.
pub fn viewer_now(resource: Resource<Result<ViewerAccess, ServerFnError>>) -> ViewerAccess {
    resource.get().and_then(Result::ok).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owners_and_admins_manage_happy() {
        let owner = ViewerAccess {
            owned_groups: vec!["g1".into()],
            owned_permissions: vec!["p1".into()],
            ..Default::default()
        };
        assert!(owner.can_manage_group("g1"));
        assert!(owner.can_manage_permission("p1"));
        let admin = ViewerAccess {
            gauge_admin: true,
            ..Default::default()
        };
        assert!(admin.can_manage_group("any"));
    }

    #[test]
    fn others_manage_nothing_sad() {
        let nobody = ViewerAccess::default();
        assert!(!nobody.can_manage_group("g1"));
        assert!(!nobody.can_manage_permission("p1"));
    }
}
