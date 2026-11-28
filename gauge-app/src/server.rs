//! Permission app server functions and DTOs.
//!
//! This module backs the `/permission` UI with APIs for:
//! - permission/group/domain CRUD,
//! - relationship management (users/groups/permissions),
//! - request workflow create/review/decision,
//! - audit/history (can-edit gated page fetch for Show History) and principal search.
//!
//! ## Security map
//!
//! - Every endpoint requires an authenticated session (fail closed).
//! - Creating, editing and deleting records, and principal search, require `GaugeAdmin`.
//! - Member, owner and allow-list changes need a session plus a step-up code; Gauge
//!   itself allows them only for the record's owners and Gauge admins.
//! - Request workflow endpoints require session only; service-layer owner/reviewer
//!   checks still apply.
//! - Fine-grained ownership checks live in [`gauge::service`].

use gauge::types::{
    HistoryEntryDto, PermissionCreateInput, PermissionDetailDto, PermissionDomainCreateInput,
    PermissionDomainDetailDto, PermissionGroupCreateInput, PermissionGroupDetailDto,
    PermissionRequestCreateInput, PermissionRequestDecisionInput, PermissionRequestRowDto,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use uf_search_core::{SearchSourceItem, SearchSourceKey};

#[cfg(feature = "ssr")]
mod activity;
/// [`viewer::get_viewer_access`] — what the signed-in person may do, read once per page.
pub mod viewer;

use crate::resource_access::{
    GroupResourceAccessRow, GroupRowDto, ManagedGroupDto, ResourceBundleDto, ResourceLink,
};

/// Permission name required for gauge admin mutations (manifest: [`crate::permissions::GaugePermission::GaugeAdmin`]).
pub const GAUGE_ADMIN_PERMISSION: &str = "GaugeAdmin";

/// Well-known Super User group id (mirrors `gauge::super_user::SUPER_USER_GROUP_ID` on SSR).
pub const SUPER_USER_GROUP_ID: &str = "super_user_group";

#[cfg(feature = "ssr")]
fn require_session(ctx: &higgs::Higgs) -> Result<(), ServerFnError> {
    if ctx.session_user_id().is_some() {
        Ok(())
    } else {
        Err(ServerFnError::new("Authentication required"))
    }
}

/// Log once at the Higgs boundary, then return a UI-facing `ServerFnError`.
///
/// Gauge's own refusals keep a prefix the error bar reads (`not_authorized:`,
/// `not_found:`, `validation:`); anything else is a store failure.
#[cfg(feature = "ssr")]
fn map_service_err<E: std::fmt::Display + 'static>(op: &'static str, e: E) -> ServerFnError {
    use gauge::service::GaugeServiceError;
    tracing::error!(error = %e, operation = op, "gauge service failed");
    let any: &dyn std::any::Any = &e;
    let gauge_err = any
        .downcast_ref::<anyhow::Error>()
        .and_then(|e| e.downcast_ref::<GaugeServiceError>())
        .or_else(|| any.downcast_ref::<GaugeServiceError>());
    let class = match gauge_err {
        Some(GaugeServiceError::NotAuthorized { .. }) => "not_authorized",
        Some(GaugeServiceError::NotFound { .. }) => "not_found",
        Some(GaugeServiceError::Validation { .. }) => "validation",
        Some(GaugeServiceError::Invariant { .. }) | None => {
            return ServerFnError::new(format!("Failed to {op}: {e}"))
        }
    };
    ServerFnError::new(format!("{class}: {e}"))
}

#[cfg(feature = "ssr")]
fn valence_from_ctx(ctx: &higgs::Higgs) -> Result<valence::Valence, ServerFnError> {
    ctx.valence()
        .map_err(|e| map_service_err("build Valence", e))
}

/// Window step-up for ordinary groups; fresh TOTP when mutating Super User membership.
#[cfg(feature = "ssr")]
async fn require_group_membership_step_up(
    group_id: &str,
    totp_code: &str,
) -> Result<(), ServerFnError> {
    if group_id == SUPER_USER_GROUP_ID {
        lepton_auth::verify_fresh_totp(totp_code)
            .await
            .map_err(|e| e.to_server_fn_error())?;
    } else {
        uf_product::permissions::require_step_up("window").await?;
    }
    Ok(())
}

/// List permissions, optionally filtered by search text.
#[uf_product_macros::server]
pub async fn list_permissions(
    /// Optional case-insensitive search text to filter permissions by.
    search: Option<String>,
) -> Result<Vec<PermissionDetailDto>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::list_permissions(&valence_from_ctx(&ctx)?, search)
        .await
        .map_err(|e| map_service_err("list permissions", e))
}

/// List groups, optionally filtered by search text.
#[uf_product_macros::server]
pub async fn list_groups(
    /// Optional case-insensitive search text to filter groups by.
    search: Option<String>,
) -> Result<Vec<PermissionGroupDetailDto>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::list_groups(&valence_from_ctx(&ctx)?, search)
        .await
        .map_err(|e| map_service_err("list groups", e))
}

/// List permission domains, optionally filtered by search text.
#[uf_product_macros::server]
pub async fn list_domains(
    /// Optional case-insensitive search text to filter domains by.
    search: Option<String>,
) -> Result<Vec<PermissionDomainDetailDto>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    // Lab-only override (feature `e2e-lab`) for create-permission Select sads.
    #[cfg(feature = "e2e-lab")]
    match crate::e2e_lab::list_domains_override() {
        crate::e2e_lab::ListDomainsOverride::Empty => return Ok(Vec::new()),
        crate::e2e_lab::ListDomainsOverride::Error => {
            return Err(ServerFnError::new(
                "Failed to list domains: e2e forced failure".to_string(),
            ));
        }
        crate::e2e_lab::ListDomainsOverride::Normal => {}
    }
    gauge::service::list_domains(&valence_from_ctx(&ctx)?, search)
        .await
        .map_err(|e| map_service_err("list domains", e))
}

/// Fetch one permission by id.
#[uf_product_macros::server]
pub async fn get_permission(
    /// Unique identifier of the permission to fetch.
    id: String,
) -> Result<Option<PermissionDetailDto>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::get_permission_detail(&id, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("load permission detail", e))
}

/// Fetch one permission group by id.
#[uf_product_macros::server]
pub async fn get_group(
    /// Unique identifier of the group to fetch.
    id: String,
) -> Result<Option<PermissionGroupDetailDto>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::get_group_detail(&id, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("load group detail", e))
}

/// Create a permission domain and return its id.
#[uf_product_macros::server(permission = "GaugeAdmin")]
pub async fn create_domain(
    /// Fields describing the new permission domain.
    input: PermissionDomainCreateInput,
) -> Result<String, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let created = gauge::service::create_domain(input, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("create domain", e))?;
    Ok(created
        .id()
        .and_then(|t| valence::extract_id_from_record(t).ok())
        .unwrap_or_default())
}

/// Fetch one permission domain by id.
#[uf_product_macros::server]
pub async fn get_domain(
    /// Unique identifier of the domain to fetch.
    id: String,
) -> Result<Option<PermissionDomainDetailDto>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::get_domain_detail(&id, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("load domain detail", e))
}

/// Payload for [`update_domain`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateDomainInput {
    /// Domain id to update.
    pub id: String,
    /// New display name.
    pub name: String,
    /// New description.
    pub description: String,
}

/// Update a permission domain.
#[uf_product_macros::server(permission = "GaugeAdmin")]
pub async fn update_domain(
    /// Updated fields for the target domain.
    input: UpdateDomainInput,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::update_domain(
        &input.id,
        input.name,
        input.description,
        &valence_from_ctx(&ctx)?,
    )
    .await
    .map_err(|e| map_service_err("update domain", e))?;
    Ok(())
}

/// Delete a permission domain.
#[uf_product_macros::server(permission = "GaugeAdmin", step_up)]
pub async fn delete_domain(
    /// Unique identifier of the domain to delete.
    id: String,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::delete_domain(&id, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("delete domain", e))?;
    Ok(())
}

/// Add a user as an owner of a domain.
#[uf_product_macros::server(permission = "GaugeAdmin", step_up)]
pub async fn add_domain_owner_user(
    /// Unique identifier of the domain to add the owner to.
    domain_id: String,
    /// Unique identifier of the user to add as an owner.
    user_id: String,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::add_domain_owner_user(&domain_id, &user_id, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("add domain owner user", e))?;
    Ok(())
}

/// Remove a user from the owner list of a domain.
#[uf_product_macros::server(permission = "GaugeAdmin", step_up)]
pub async fn remove_domain_owner_user(
    /// Unique identifier of the domain to remove the owner from.
    domain_id: String,
    /// Unique identifier of the user to remove from the owner list.
    user_id: String,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::remove_domain_owner_user(&domain_id, &user_id, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("remove domain owner user", e))?;
    Ok(())
}

/// Create a permission and return its id.
#[uf_product_macros::server(permission = "GaugeAdmin", step_up)]
pub async fn create_permission(
    /// Fields describing the new permission.
    input: PermissionCreateInput,
) -> Result<String, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let created = gauge::service::create_permission(input, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("create permission", e))?;
    Ok(created
        .id()
        .and_then(|t| valence::extract_id_from_record(t).ok())
        .unwrap_or_default())
}

/// Payload for [`update_permission`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdatePermissionInput {
    /// Permission id to update.
    pub id: String,
    /// The key code checks. Pass the current key; it never changes after create.
    pub name: String,
    /// The name people read. Empty leaves it as it is.
    #[serde(default)]
    pub display_name: String,
    /// New description.
    pub description: String,
    /// New owners group id.
    pub owners_group_id: String,
    /// New permission domain id.
    pub domain_id: String,
}

/// Update an existing permission.
#[uf_product_macros::server(permission = "GaugeAdmin", step_up)]
pub async fn update_permission(
    /// Updated fields for the target permission.
    input: UpdatePermissionInput,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let v = valence_from_ctx(&ctx)?;
    gauge::service::update_permission(
        &input.id,
        input.name,
        input.description,
        input.owners_group_id,
        input.domain_id,
        &v,
    )
    .await
    .map_err(|e| map_service_err("update permission", e))?;
    if !input.display_name.trim().is_empty() {
        gauge::service::set_permission_display_name(&input.id, &input.display_name, &v)
            .await
            .map_err(|e| map_service_err("rename permission", e))?;
    }
    Ok(())
}

/// Delete a permission by id.
#[uf_product_macros::server(permission = "GaugeAdmin", step_up)]
pub async fn delete_permission(
    /// Unique identifier of the permission to delete.
    id: String,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::delete_permission(&id, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("delete permission", e))?;
    Ok(())
}

/// Create a permission group and return its id.
#[uf_product_macros::server(permission = "GaugeAdmin")]
pub async fn create_group(
    /// Fields describing the new permission group.
    input: PermissionGroupCreateInput,
) -> Result<String, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let created = gauge::service::create_group(input, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("create group", e))?;
    Ok(created
        .id()
        .and_then(|t| valence::extract_id_from_record(t).ok())
        .unwrap_or_default())
}

/// Payload for [`update_group`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateGroupInput {
    /// Group id to update.
    pub id: String,
    /// New display name.
    pub name: String,
    /// New description.
    pub description: String,
}

/// Update a permission group.
#[uf_product_macros::server(permission = "GaugeAdmin")]
pub async fn update_group(
    /// Updated fields for the target group.
    input: UpdateGroupInput,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::update_group(
        &input.id,
        input.name,
        input.description,
        &valence_from_ctx(&ctx)?,
    )
    .await
    .map_err(|e| map_service_err("update group", e))?;
    Ok(())
}

/// Delete a permission group by id.
#[uf_product_macros::server(permission = "GaugeAdmin", step_up)]
pub async fn delete_group(
    /// Unique identifier of the group to delete.
    id: String,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::delete_group(&id, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("delete group", e))?;
    Ok(())
}

/// Grant a permission directly to a user.
#[uf_product_macros::server(step_up)]
pub async fn add_permission_user(
    /// Unique identifier of the permission to grant.
    permission_id: String,
    /// Unique identifier of the user to grant the permission to.
    user_id: String,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::grant_permission_to_user(&permission_id, &user_id, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("add permission user", e))?;
    Ok(())
}

/// Revoke a direct user permission grant.
#[uf_product_macros::server(step_up)]
pub async fn remove_permission_user(
    /// Unique identifier of the permission to revoke.
    permission_id: String,
    /// Unique identifier of the user to revoke the permission from.
    user_id: String,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::revoke_permission_from_user(&permission_id, &user_id, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("remove permission user", e))?;
    Ok(())
}

/// Grant a permission to a group.
#[uf_product_macros::server(step_up)]
pub async fn add_permission_group(
    /// Unique identifier of the permission to grant.
    permission_id: String,
    /// Unique identifier of the group to grant the permission to.
    group_id: String,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::grant_permission_to_group(&permission_id, &group_id, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("add permission group", e))?;
    Ok(())
}

/// Revoke a group permission grant.
#[uf_product_macros::server(step_up)]
pub async fn remove_permission_group(
    /// Unique identifier of the permission to revoke.
    permission_id: String,
    /// Unique identifier of the group to revoke the permission from.
    group_id: String,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::revoke_permission_from_group(
        &permission_id,
        &group_id,
        &valence_from_ctx(&ctx)?,
    )
    .await
    .map_err(|e| map_service_err("remove permission group", e))?;
    Ok(())
}

/// Add a user as a member of a group.
#[uf_product_macros::server]
pub async fn add_group_user(
    /// Unique identifier of the group to add the member to.
    group_id: String,
    /// Unique identifier of the user to add as a member.
    user_id: String,
    /// Fresh TOTP when `group_id` is the Super User group; ignored otherwise.
    totp_code: String,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    require_group_membership_step_up(&group_id, &totp_code).await?;
    gauge::service::add_group_member_user(&group_id, &user_id, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("add group user", e))?;
    Ok(())
}

/// Add a user as an owner of a group.
#[uf_product_macros::server]
pub async fn add_group_owner_user(
    /// Unique identifier of the group to add the owner to.
    group_id: String,
    /// Unique identifier of the user to add as an owner.
    user_id: String,
    /// Fresh TOTP when `group_id` is the Super User group; ignored otherwise.
    totp_code: String,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    require_group_membership_step_up(&group_id, &totp_code).await?;
    gauge::service::add_group_owner_user(&group_id, &user_id, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("add group owner user", e))?;
    Ok(())
}

/// Remove a user from the owner list of a group.
#[uf_product_macros::server]
pub async fn remove_group_owner_user(
    /// Unique identifier of the group to remove the owner from.
    group_id: String,
    /// Unique identifier of the user to remove from the owner list.
    user_id: String,
    /// Fresh TOTP when `group_id` is the Super User group; ignored otherwise.
    totp_code: String,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    require_group_membership_step_up(&group_id, &totp_code).await?;
    gauge::service::remove_group_owner_user(&group_id, &user_id, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("remove group owner user", e))?;
    Ok(())
}

/// Remove a user from group membership.
#[uf_product_macros::server]
pub async fn remove_group_user(
    /// Unique identifier of the group to remove the member from.
    group_id: String,
    /// Unique identifier of the user to remove from membership.
    user_id: String,
    /// Fresh TOTP when `group_id` is the Super User group; ignored otherwise.
    totp_code: String,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    require_group_membership_step_up(&group_id, &totp_code).await?;
    gauge::service::remove_group_member_user(&group_id, &user_id, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("remove group user", e))?;
    Ok(())
}

/// Remove a service user from group membership.
#[uf_product_macros::server]
pub async fn remove_group_service_user(
    /// Unique identifier of the group to remove the member from.
    group_id: String,
    /// Unique identifier of the service user to remove from membership.
    service_user_id: String,
    /// Fresh TOTP when `group_id` is the Super User group; ignored otherwise.
    totp_code: String,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let service_user = lepton_identity::service_users::ServiceUserId::parse(&service_user_id)
        .map_err(|_| ServerFnError::new("Invalid service user id"))?;
    require_group_membership_step_up(&group_id, &totp_code).await?;
    gauge::service::remove_group_member_service_user(
        &group_id,
        &service_user,
        &valence_from_ctx(&ctx)?,
    )
    .await
    .map_err(|e| map_service_err("remove group service user", e))?;
    Ok(())
}

/// Add a service user as a member of a group.
///
/// Gauge refuses the Super User group, a group the caller doesn't own, and a service user
/// the caller can't see (its owner, or someone who manages every service user).
#[uf_product_macros::server]
pub async fn add_group_service_user(
    /// Unique identifier of the group to add the member to.
    group_id: String,
    /// Unique identifier of the service user to add as a member.
    service_user_id: String,
    /// Fresh TOTP when `group_id` is the Super User group; ignored otherwise.
    totp_code: String,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let service_user = lepton_identity::service_users::ServiceUserId::parse(&service_user_id)
        .map_err(|_| ServerFnError::new("Invalid service user id"))?;
    require_group_membership_step_up(&group_id, &totp_code).await?;
    gauge::service::add_group_member_service_user(
        &group_id,
        &service_user,
        &valence_from_ctx(&ctx)?,
    )
    .await
    .map_err(|e| map_service_err("add group service user", e))?;
    Ok(())
}

/// Add a child group relationship to a parent group.
#[uf_product_macros::server(step_up)]
pub async fn add_group_group(
    /// Unique identifier of the parent group.
    group_id: String,
    /// Unique identifier of the child group to add.
    child_group_id: String,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::add_group_member_group(&group_id, &child_group_id, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("add child group", e))?;
    Ok(())
}

/// Remove a child group relationship from a parent group.
#[uf_product_macros::server(step_up)]
pub async fn remove_group_group(
    /// Unique identifier of the parent group.
    group_id: String,
    /// Unique identifier of the child group to remove.
    child_group_id: String,
) -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::remove_group_member_group(&group_id, &child_group_id, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("remove child group", e))?;
    Ok(())
}

/// List permission history entries with optional subject filters.
#[uf_product_macros::server]
pub async fn list_history(
    /// Optional subject kind filter (e.g. `"permission"`, `"group"`).
    subject_kind: Option<String>,
    /// Optional subject id filter, scoping history to a single subject.
    subject_id: Option<String>,
) -> Result<Vec<HistoryEntryDto>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::list_history(&valence_from_ctx(&ctx)?, subject_kind, subject_id)
        .await
        .map_err(|e| map_service_err("list history", e))
}

/// One page of Gauge history for the Show History dialog, newest first, by `offset`.
///
/// Stricter than stock [`record_history_leptos::get_record_history_page`]: requires
/// session **and** can-edit on the `permission` / `permission_group` source (via
/// [`gauge::service::actor_can_view_history_subject`]), because parent Read is
/// `AUTHENTICATED` and would otherwise widen history to every signed-in user.
///
/// Pages by offset only. The returned page still carries `next_cursor`, which
/// [`get_gauge_history_page_by_cursor`] accepts for the next page.
#[uf_product_macros::server]
pub async fn get_gauge_history_page(
    /// Zero-based index of the first history row to return.
    offset: u32,
    /// Maximum number of history rows to return.
    limit: u32,
    /// Parent permission or permission_group record.
    source: valence::RecordId,
    /// Optional history-table kind filter (e.g. `permission_history`).
    kinds: Option<Vec<String>>,
) -> Result<orbital_paging::Page<record_history_leptos::HistoryRowView>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        serve_gauge_history_page(offset, limit, source, kinds, None).await
    }

    #[cfg(not(feature = "ssr"))]
    {
        let _ = (offset, limit, source, kinds);
        Err(ServerFnError::ServerError(
            "GetGaugeHistoryPage requires SSR".into(),
        ))
    }
}

/// One page of Gauge history, newest first, continued by `cursor`.
///
/// Same checks as [`get_gauge_history_page`]. Pass the previous page's
/// `next_cursor` as `cursor` to continue after its last entry instead of
/// re-reading from the newest one; a missing, stale or malformed cursor pages
/// by `offset`. The Show History dialog loads its pages through this function.
/// A cursor grants nothing: the session and can-edit checks run on every page.
#[uf_product_macros::server]
pub async fn get_gauge_history_page_by_cursor(
    /// Zero-based index of the first history row to return (offset path only).
    offset: u32,
    /// Maximum number of history rows to return.
    limit: u32,
    /// Parent permission or permission_group record.
    source: valence::RecordId,
    /// Optional history-table kind filter (e.g. `permission_history`).
    kinds: Option<Vec<String>>,
    /// The previous page's `next_cursor`; `None` pages by `offset`.
    cursor: Option<String>,
) -> Result<orbital_paging::Page<record_history_leptos::HistoryRowView>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        serve_gauge_history_page(offset, limit, source, kinds, cursor).await
    }

    #[cfg(not(feature = "ssr"))]
    {
        let _ = (offset, limit, source, kinds, cursor);
        Err(ServerFnError::ServerError(
            "GetGaugeHistoryPageByCursor requires SSR".into(),
        ))
    }
}

/// Session check and request actor for both Gauge history server fns.
#[cfg(feature = "ssr")]
async fn serve_gauge_history_page(
    offset: u32,
    limit: u32,
    source: valence::RecordId,
    kinds: Option<Vec<String>>,
    cursor: Option<String>,
) -> Result<orbital_paging::Page<record_history_leptos::HistoryRowView>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let v = valence_from_ctx(&ctx)?;
    load_gauge_history_page(&source, &v, offset, limit, kinds, cursor).await
}

/// The body of [`get_gauge_history_page`] and [`get_gauge_history_page_by_cursor`]
/// after the session check.
///
/// `v` carries the request actor. Only `permission` and `permission_group`
/// sources are accepted, and the actor must be able to edit the source.
/// `offset`, `limit` and `kinds` are clamped here; paging itself (cursor or
/// offset) is [`record_history_leptos::load_history_page`].
///
/// ## Errors
///
/// [`record_history::HISTORY_ACCESS_DENIED`] for another table, a source the
/// actor can't edit, or one that doesn't exist; `Failed to load record history`
/// for any other failure.
#[cfg(feature = "ssr")]
pub async fn load_gauge_history_page(
    source: &valence::RecordId,
    v: &valence::Valence,
    offset: u32,
    limit: u32,
    kinds: Option<Vec<String>>,
    cursor: Option<String>,
) -> Result<orbital_paging::Page<record_history_leptos::HistoryRowView>, ServerFnError> {
    use record_history_leptos::{
        clamp_history_page_limit, clamp_history_page_offset, load_history_page,
        sanitize_kind_filter,
    };

    let table = source.table();
    if !matches!(table, "permission" | "permission_group") {
        return Err(ServerFnError::new(record_history::HISTORY_ACCESS_DENIED));
    }

    let can_view = gauge::service::actor_can_view_history_subject(table, source.id(), v)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, operation = "get_gauge_history_page", "can-edit check");
            ServerFnError::new("Failed to load record history")
        })?;
    if !can_view {
        return Err(ServerFnError::new(record_history::HISTORY_ACCESS_DENIED));
    }

    let kinds = sanitize_kind_filter(kinds);
    load_history_page(
        source,
        v,
        clamp_history_page_offset(offset),
        clamp_history_page_limit(limit),
        kinds.as_deref(),
        cursor,
    )
    .await
}

/// Search principal sources (users/groups) for picker UIs.
#[uf_product_macros::server(permission = "GaugeAdmin")]
pub async fn search_principals(
    /// Search sources to query (e.g. users, groups).
    source_keys: Vec<SearchSourceKey>,
    /// Optional free-text query; empty/missing returns each source's default results.
    query: Option<String>,
    /// Maximum number of results to return per source.
    max_results: u32,
) -> Result<Vec<SearchSourceItem>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        /// Hard cap so GaugeAdmin callers cannot force unbounded principal scans.
        const MAX_PRINCIPAL_SEARCH_RESULTS: u32 = 50;

        let ctx = higgs::Higgs::from_request().await?;
        require_session(&ctx)?;
        let v = valence_from_ctx(&ctx)?;

        let query_text = query.unwrap_or_default().trim().to_string();
        let max_results = max_results.clamp(1, MAX_PRINCIPAL_SEARCH_RESULTS);
        let registry = uf_search_core::SearchSourceRegistry::auto_discover();
        log::info!(
            "permission search_principals start query_len={} source_keys={:?} max_results={}",
            query_text.chars().count(),
            source_keys,
            max_results
        );

        let out = registry
            .query_many(&source_keys, &v, &query_text, max_results)
            .await
            .map_err(|e| map_service_err("query principals", e))?;
        log::info!(
            "permission search_principals end total_results={}",
            out.len()
        );

        return Ok(out);
    }

    #[allow(unreachable_code)]
    {
        let _ = (source_keys, query, max_results);
        Ok(Vec::new())
    }
}

/// Create a permission request for review.
#[uf_product_macros::server]
pub async fn create_permission_request(
    /// Fields describing the requested permission grant.
    input: PermissionRequestCreateInput,
) -> Result<PermissionRequestRowDto, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::create_permission_request(input, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("create permission request", e))
}

/// One of the caller's active service users, as offered by the Request Access dialog's
/// "Request for" choice.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceUserOption {
    /// Handle, such as `spectra-agent`; submitted as the request's `on_behalf_of`.
    pub handle: String,
    /// Name shown next to the handle.
    pub display_name: String,
}

/// List the current actor's active service users, oldest first.
#[uf_product_macros::server]
pub async fn list_my_active_service_users() -> Result<Vec<ServiceUserOption>, ServerFnError> {
    use lepton::generated::ServiceUserAccountStatus;

    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let service_users = lepton_auth::service_users::list_my_service_users(&valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("list my service users", e))?;
    Ok(service_users
        .into_iter()
        .filter(|su| su.status() == &ServiceUserAccountStatus::Active)
        .map(|su| ServiceUserOption {
            handle: su.handle().clone(),
            display_name: su.display_name().clone(),
        })
        .collect())
}

/// List permission requests created by the current actor.
#[uf_product_macros::server]
pub async fn list_my_permission_requests() -> Result<Vec<PermissionRequestRowDto>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let v = valence_from_ctx(&ctx)?;
    let mut rows = gauge::service::list_permission_requests_for_actor(&v)
        .await
        .map_err(|e| map_service_err("list my permission requests", e))?;
    relabel_resource_requests(&v, &mut rows).await;
    Ok(rows)
}

/// List permission requests awaiting current actor review.
#[uf_product_macros::server]
pub async fn list_review_permission_requests() -> Result<Vec<PermissionRequestRowDto>, ServerFnError>
{
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let v = valence_from_ctx(&ctx)?;
    let mut rows = gauge::service::list_permission_requests_for_review(&v)
        .await
        .map_err(|e| map_service_err("list review permission requests", e))?;
    relabel_resource_requests(&v, &mut rows).await;
    Ok(rows)
}

/// Get one permission request by id.
#[uf_product_macros::server]
pub async fn get_permission_request(
    /// Unique identifier of the permission request to fetch.
    request_id: String,
) -> Result<Option<PermissionRequestRowDto>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let v = valence_from_ctx(&ctx)?;
    let mut row = gauge::service::get_permission_request_detail(&request_id, &v)
        .await
        .map_err(|e| map_service_err("load permission request", e))?;
    if let Some(row) = row.as_mut() {
        relabel_resource_requests(&v, std::slice::from_mut(row)).await;
    }
    Ok(row)
}

/// Approve or reject a permission request.
#[uf_product_macros::server(step_up)]
pub async fn decide_permission_request(
    /// Decision (approve/reject) and target request id.
    input: PermissionRequestDecisionInput,
) -> Result<PermissionRequestRowDto, ServerFnError> {
    use gauge::types::PermissionRequestDecision;

    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let v = valence_from_ctx(&ctx)?;
    // Gauge wants a reason with every denial. Until the request page asks for one, a deny
    // from it goes through `deny_request` with this sentence.
    if matches!(input.decision, PermissionRequestDecision::Deny) {
        return gauge::service::deny_request(&input.request_id, DEFAULT_DENY_REASON, &v)
            .await
            .map_err(|e| map_service_err("deny the request", e));
    }
    gauge::service::decide_permission_request(input, &v)
        .await
        .map_err(|e| map_service_err("decide permission request", e))
}

/// The reason a deny from the request page carries until that page asks for one.
pub const DEFAULT_DENY_REASON: &str = "The owners turned this request down.";

/// The resource behind a resource-scoped domain, or `None` for a taxonomy domain.
#[cfg(feature = "ssr")]
async fn resource_link_for_domain(
    v: &valence::Valence,
    domain_id: &str,
) -> Result<Option<ResourceLink>, ServerFnError> {
    use crate::resource_access::{humanize_kind, kind_of_domain};

    let Some(domain) = <gauge::generated::PermissionDomain as valence::Model>::get(domain_id, v, valence::use_!(r"When **Gauge** shows a **resource's permissions**, we **load the resource's permission domain** to find which resource (a job, a topic) it belongs to, so the page can name the resource and link back to it."))
        .await
        .map_err(|e| map_service_err("load resource domain", e))?
    else {
        return Ok(None);
    };
    if !*domain.resource_scoped() {
        return Ok(None);
    }
    let Some(resource_id) = domain.resource_id().cloned() else {
        return Ok(None);
    };
    let fragment = gauge::resource_permissions::normalize_id_fragment(&resource_id);
    let Some(kind) = kind_of_domain(domain_id, &fragment) else {
        return Ok(None);
    };
    let entry = crate::bridge::registered_resource_kind(kind);
    Ok(Some(ResourceLink {
        kind: kind.to_string(),
        kind_label: entry.map_or_else(
            || humanize_kind(kind),
            |e| e.descriptor.display_label.to_string(),
        ),
        href: entry.map(|e| (e.href)(&resource_id)),
        app: entry.map(|e| e.app.to_string()),
        resource_id,
        domain_id: domain_id.to_string(),
    }))
}

/// The resource and role a request is for, when it targets a resource bundle.
#[cfg(feature = "ssr")]
async fn resource_of_request(
    v: &valence::Valence,
    target_kind: &gauge::types::PermissionRequestTargetKind,
    target_id: &str,
) -> Result<Option<(ResourceLink, String)>, ServerFnError> {
    use crate::resource_access::{
        action_of_permission_name, domain_of_owners_group, role_label_for_action,
    };
    use gauge::types::PermissionRequestTargetKind;

    let (domain_id, role) = match target_kind {
        PermissionRequestTargetKind::Group => match domain_of_owners_group(target_id) {
            Some(domain_id) => (domain_id, "Owner".to_string()),
            None => return Ok(None),
        },
        PermissionRequestTargetKind::Permission => {
            let Some(permission) = gauge::service::get_permission_detail(target_id, v)
                .await
                .map_err(|e| map_service_err("load request permission", e))?
            else {
                return Ok(None);
            };
            let Some(action) = action_of_permission_name(&permission.name) else {
                return Ok(None);
            };
            (permission.domain_id, role_label_for_action(action))
        }
    };
    Ok(resource_link_for_domain(v, &domain_id)
        .await?
        .map(|link| (link, role)))
}

/// Rename resource requests to "Chronon job billing-rollup · Can view". A row whose
/// resource can't be read keeps Gauge's own label; the failure is logged.
///
/// Loads one domain per resource row; follow-up rag-gauge-app-fu2 batches those loads.
#[cfg(feature = "ssr")]
async fn relabel_resource_requests(v: &valence::Valence, rows: &mut [PermissionRequestRowDto]) {
    for row in rows {
        match resource_of_request(v, &row.target_kind, &row.target_id).await {
            Ok(Some((link, role))) => {
                row.target_label = crate::resource_access::request_label(&link, &role);
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(error = %e, request_id = %row.id, "resource request label failed");
            }
        }
    }
}

/// The resource a request targets, with its link, for the request page.
#[uf_product_macros::server]
pub async fn get_request_resource(
    /// Whether the request targets a permission or a group.
    target_kind: gauge::types::PermissionRequestTargetKind,
    /// The request's target id.
    target_id: String,
) -> Result<Option<ResourceLink>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    resource_of_request(&valence_from_ctx(&ctx)?, &target_kind, &target_id)
        .await
        .map(|found| found.map(|(link, _)| link))
}

/// The resource bundle behind a resource-scoped domain: the resource, each action's
/// permission and holders, and the roles. `None` for a taxonomy domain.
#[uf_product_macros::server]
pub async fn get_resource_bundle(
    /// Domain id, such as `rp_domain_chronon_job_billing_rollup_5c1e9a07`.
    domain_id: String,
) -> Result<Option<ResourceBundleDto>, ServerFnError> {
    use crate::resource_access::{bundle_roles, BundlePermissionRow, ACTION_ORDER, OWNERS_PREFIX};

    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let v = valence_from_ctx(&ctx)?;
    let Some(resource) = resource_link_for_domain(&v, &domain_id).await? else {
        return Ok(None);
    };
    let fragment = gauge::resource_permissions::normalize_id_fragment(&resource.resource_id);
    let name_prefix = format!("{}.{fragment}.", resource.kind);
    let mut permissions: Vec<PermissionDetailDto> =
        gauge::service::list_permissions(&v, Some(name_prefix.clone()))
            .await
            .map_err(|e| map_service_err("load resource bundle", e))?
            .into_iter()
            .filter(|p| p.domain_id == domain_id && p.name.starts_with(&name_prefix))
            .collect();
    let action = |p: &PermissionDetailDto| p.name[name_prefix.len()..].to_string();
    permissions.sort_by_key(|p| ACTION_ORDER.iter().position(|a| *a == action(p)));

    let owners_group_id = permissions.first().map_or_else(
        || {
            format!(
                "{OWNERS_PREFIX}{}",
                domain_id.trim_start_matches(crate::resource_access::DOMAIN_PREFIX)
            )
        },
        |p| p.owners_group_id.clone(),
    );
    let holders_visible = permissions.iter().any(|p| !p.allow_list.is_empty());
    let default_owner_id =
        crate::bridge::registered_resource_kind(&resource.kind).and_then(|e| e.default_owner_group);
    let default_owner = default_owner_id.map(|id| {
        permissions
            .iter()
            .flat_map(|p| &p.allow_list)
            .find(|holder| holder.id == id)
            .map_or_else(|| id.to_string(), |holder| holder.label.clone())
    });
    let actions: Vec<String> = permissions.iter().map(action).collect();
    let action_refs: Vec<&str> = actions.iter().map(String::as_str).collect();
    let roles = bundle_roles(&action_refs, &owners_group_id);
    let rows = permissions
        .into_iter()
        .zip(&actions)
        .map(|(p, action)| BundlePermissionRow {
            action: action.clone(),
            permission_id: p.id,
            permission_name: p.name,
            held_by: p
                .allow_list
                .iter()
                .map(|holder| {
                    if holder.id == owners_group_id {
                        "Owners".to_string()
                    } else {
                        holder.label.clone()
                    }
                })
                .collect(),
        })
        .collect();

    Ok(Some(ResourceBundleDto {
        resource,
        owners_group_id,
        default_owner,
        permissions: rows,
        roles,
        holders_visible,
    }))
}

/// Management fields of a group another product syncs (a Polaron team), as a page shows them.
#[cfg(feature = "ssr")]
fn managed_dto(
    group_id: &str,
    group: &gauge::generated::PermissionGroup,
) -> Option<ManagedGroupDto> {
    let managed_by = group.managed_by()?.clone();
    let external_ref = group.external_ref().cloned();
    let (source_label, source_href) =
        crate::resource_access::managed_source(&managed_by, group_id, external_ref.as_deref());
    Some(ManagedGroupDto {
        archived: group.managed_status().map(String::as_str) == Some("archived"),
        managed_by,
        source_label,
        external_ref,
        source_href,
    })
}

/// List groups like [`list_groups`], with each managed (team) group's management.
#[uf_product_macros::server]
pub async fn list_group_rows(
    /// Optional case-insensitive search text to filter groups by.
    search: Option<String>,
) -> Result<Vec<GroupRowDto>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let v = valence_from_ctx(&ctx)?;
    let groups = gauge::service::list_groups(&v, search)
        .await
        .map_err(|e| map_service_err("list groups", e))?;
    let models = gauge::generated::PermissionGroup::query(&v, valence::use_!(r"When someone opens the **Groups** page in **Gauge**, we **list permission groups** to find the ones **another product manages** (Polaron teams), so the page can badge them as Teams and show whether the team is archived."))
        .await
        .map_err(|e| map_service_err("list groups", e))?;
    let managed: std::collections::HashMap<String, ManagedGroupDto> = models
        .iter()
        .filter_map(|model| {
            let id = model
                .id()
                .and_then(|t| valence::extract_id_from_record(t).ok())?;
            let dto = managed_dto(&id, model)?;
            Some((id, dto))
        })
        .collect();
    Ok(groups
        .into_iter()
        .map(|group| GroupRowDto {
            managed: managed.get(&group.id).cloned(),
            group,
        })
        .collect())
}

/// The management of one group when another product syncs its members, else `None`.
#[uf_product_macros::server]
pub async fn get_group_managed(
    /// Group id.
    group_id: String,
) -> Result<Option<ManagedGroupDto>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let v = valence_from_ctx(&ctx)?;
    let group = <gauge::generated::PermissionGroup as valence::Model>::get(&group_id, &v, valence::use_!(r"When someone opens a **group's page** in **Gauge**, we **load the group** to see whether **another product manages its members** (a Polaron team) and whether that team is archived, so the page can show its members read-only."))
        .await
        .map_err(|e| map_service_err("load group", e))?;
    Ok(group.and_then(|g| managed_dto(&group_id, &g)))
}

/// The resources a group holds a role on: direct grants on a resource bundle, or a seat in
/// a bundle's owners group. Grants Gauge withholds from the caller don't show.
///
/// Scans every permission and group, then loads each bundle's domain; follow-up rag-gauge-app-fu2
/// replaces this with a Gauge query by holder and a batched domain load.
#[uf_product_macros::server]
pub async fn list_group_resource_access(
    /// Group id.
    group_id: String,
) -> Result<Vec<GroupResourceAccessRow>, ServerFnError> {
    use crate::resource_access::{
        action_of_permission_name, domain_of_owners_group, role_label_for_grants, DOMAIN_PREFIX,
    };
    use gauge::types::PrincipalKind;
    use std::collections::BTreeMap;

    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let v = valence_from_ctx(&ctx)?;

    // domain id → (bundle actions, actions the group holds, owner)
    let mut bundles: BTreeMap<String, (Vec<String>, Vec<String>, bool)> = BTreeMap::new();
    let permissions = gauge::service::list_permissions(&v, None)
        .await
        .map_err(|e| map_service_err("list group access", e))?;
    for p in permissions {
        if !p.domain_id.starts_with(DOMAIN_PREFIX) {
            continue;
        }
        let Some(action) = action_of_permission_name(&p.name) else {
            continue;
        };
        let held = p
            .allow_list
            .iter()
            .any(|holder| holder.kind == PrincipalKind::Group && holder.id == group_id);
        let bundle = bundles.entry(p.domain_id.clone()).or_default();
        bundle.0.push(action.to_string());
        if held {
            bundle.1.push(action.to_string());
        }
    }
    let groups = gauge::service::list_groups(&v, None)
        .await
        .map_err(|e| map_service_err("list group access", e))?;
    for owners in groups {
        let Some(domain_id) = domain_of_owners_group(&owners.id) else {
            continue;
        };
        let seated = owners
            .members
            .iter()
            .any(|m| m.kind == PrincipalKind::Group && m.id == group_id);
        if seated {
            bundles.entry(domain_id).or_default().2 = true;
        }
    }

    let mut rows = Vec::new();
    for (domain_id, (actions, held, owner)) in bundles {
        let role = if owner {
            Some("Owner".to_string())
        } else {
            let held: Vec<&str> = held.iter().map(String::as_str).collect();
            let actions: Vec<&str> = actions.iter().map(String::as_str).collect();
            role_label_for_grants(&held, &actions)
        };
        let Some(role) = role else {
            continue;
        };
        let Some(resource) = resource_link_for_domain(&v, &domain_id).await? else {
            continue;
        };
        rows.push(GroupResourceAccessRow { resource, role });
    }
    rows.sort_by_key(|row| row.resource.title());
    Ok(rows)
}

// ---------------------------------------------------------------------------------------
// Who has access and why, people, activity and request decisions.

/// Everyone who passes a permission's check, with every path that gives it to them.
///
/// Gauge lets the permission's owners and Gauge admins read this.
#[uf_product_macros::server]
pub async fn get_effective_holders(
    /// Permission id.
    permission_id: String,
) -> Result<Vec<gauge::types::HolderRow>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::list_effective_holders(&permission_id, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("list who has this permission", e))
}

/// What every member of a group gets from it. The group's owners and Gauge admins may read it.
#[uf_product_macros::server]
pub async fn get_group_access(
    /// Group id.
    group_id: String,
) -> Result<Vec<gauge::types::PermissionRef>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::list_group_access(&group_id, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("list what the group gives", e))
}

/// One page of the people Gauge knows about. Gauge admins only.
#[uf_product_macros::server]
pub async fn get_people(
    /// Name or handle to search for.
    query: Option<String>,
    /// The previous page's `next_cursor`.
    cursor: Option<String>,
) -> Result<gauge::types::Page<gauge::types::PersonRow>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::list_people(
        query.as_deref(),
        cursor.as_deref(),
        &valence_from_ctx(&ctx)?,
    )
    .await
    .map_err(|e| map_service_err("list people", e))
}

/// What one person or service user can do, and why.
#[uf_product_macros::server]
pub async fn get_principal_access(
    /// The user or service user.
    principal: gauge::types::PrincipalKey,
) -> Result<gauge::types::PrincipalAccess, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::principal_access(&principal, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("load their access", e))
}

/// What the signed-in person can do, and why.
#[uf_product_macros::server]
pub async fn get_my_access() -> Result<gauge::types::PrincipalAccess, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::principal_access_self(&valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("load your access", e))
}

/// One page of Gauge's change log. Gauge admins only.
#[uf_product_macros::server]
pub async fn get_activity(
    /// Which changes to list.
    filter: gauge::types::ActivityFilter,
    /// The previous page's `next_cursor`.
    cursor: Option<String>,
) -> Result<gauge::types::Page<gauge::types::ActivityRow>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::list_activity(filter, cursor.as_deref(), &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("load activity", e))
}

/// The changes to one permission, group or domain, newest first, for its Activity tab.
///
/// Gauge admins read Gauge's change log filtered to the record. The record's owners read its
/// history rows, named here by person and group.
#[uf_product_macros::server]
pub async fn get_record_activity(
    /// The record.
    record: gauge::types::RecordRef,
) -> Result<Vec<gauge::types::ActivityRow>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let v = valence_from_ctx(&ctx)?;
    let admin = gauge::service::actor_is_gauge_admin(&v)
        .await
        .map_err(|e| map_service_err("load activity", e))?;
    if admin {
        let filter = gauge::types::ActivityFilter {
            record: Some(record),
            ..Default::default()
        };
        return gauge::service::list_activity(filter, None, &v)
            .await
            .map(|page| page.items)
            .map_err(|e| map_service_err("load activity", e));
    }
    crate::server::activity::owner_record_activity(record, &v).await
}

/// Turn down an access request, with a reason the requester sees.
#[uf_product_macros::server]
pub async fn deny_request(
    /// Request id.
    id: String,
    /// Why, in a sentence.
    reason: String,
) -> Result<PermissionRequestRowDto, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::deny_request(&id, &reason, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("deny the request", e))
}

/// Withdraw one of the caller's own pending requests.
#[uf_product_macros::server]
pub async fn withdraw_request(
    /// Request id.
    id: String,
) -> Result<PermissionRequestRowDto, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::withdraw_request(&id, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("withdraw the request", e))
}

/// Most groups one picker search returns.
pub const GROUP_SEARCH_LIMIT: usize = 20;

/// Groups whose name or description matches `query`, as picker options, for any signed-in
/// caller. Gauge lists only the groups the caller may see.
#[uf_product_macros::server]
pub async fn search_group_options(
    /// Typed text. Empty lists the first groups by name.
    query: String,
) -> Result<Vec<SearchSourceItem>, ServerFnError> {
    use gauge::search_sources::PermissionSearchSourceId;

    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let q = query.trim();
    let search = (!q.is_empty()).then(|| q.to_string());
    let groups = gauge::service::list_groups(&valence_from_ctx(&ctx)?, search)
        .await
        .map_err(|e| map_service_err("search groups", e))?;
    Ok(groups
        .into_iter()
        .take(GROUP_SEARCH_LIMIT)
        .map(|g| SearchSourceItem {
            source_id: PermissionSearchSourceId::PermissionGroup.as_str().into(),
            id: g.id,
            title: g.name,
            description: Some(g.description).filter(|d| !d.trim().is_empty()),
            kind: "group".into(),
            image_url: None,
        })
        .collect())
}

/// One row of the permissions list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionListRow {
    /// Permission id.
    pub id: String,
    /// The name people read.
    pub display_name: String,
    /// The key code checks.
    pub key: String,
    /// Description, as Markdown.
    pub description: String,
    /// Domain name.
    pub domain_name: String,
    /// The owners group's name.
    pub owned_by: String,
    /// How many people and service users pass the check. Only for Gauge admins.
    pub holders: Option<usize>,
}

/// Every permission the caller can see, with its owners group's name, and for Gauge admins
/// how many pass its check.
#[uf_product_macros::server]
pub async fn list_permission_rows() -> Result<Vec<PermissionListRow>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let v = valence_from_ctx(&ctx)?;
    let permissions = gauge::service::list_permissions(&v, None)
        .await
        .map_err(|e| map_service_err("list permissions", e))?;
    let groups: std::collections::HashMap<String, String> = gauge::service::list_groups(&v, None)
        .await
        .map_err(|e| map_service_err("list groups", e))?
        .into_iter()
        .map(|g| (g.id, g.name))
        .collect();
    let admin = gauge::service::actor_is_gauge_admin(&v)
        .await
        .map_err(|e| map_service_err("list permissions", e))?;
    let mut rows = Vec::with_capacity(permissions.len());
    for p in permissions {
        let holders = if admin {
            gauge::service::list_effective_holders(&p.id, &v)
                .await
                .ok()
                .map(|h| h.len())
        } else {
            None
        };
        rows.push(PermissionListRow {
            display_name: if p.display_name.trim().is_empty() {
                p.name.clone()
            } else {
                p.display_name.clone()
            },
            owned_by: groups.get(&p.owners_group_id).cloned().unwrap_or_default(),
            id: p.id,
            key: p.name,
            description: p.description,
            domain_name: p.domain_name,
            holders,
        });
    }
    rows.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });
    Ok(rows)
}
