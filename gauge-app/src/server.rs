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
//! - Admin mutations and principal search require `GaugeAdmin`.
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
#[cfg(feature = "ssr")]
fn map_service_err(op: &'static str, e: impl std::fmt::Display) -> ServerFnError {
    tracing::error!(error = %e, operation = op, "gauge service failed");
    ServerFnError::new(format!("Failed to {op}: {e}"))
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
    /// New display name.
    pub name: String,
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
    gauge::service::update_permission(
        &input.id,
        input.name,
        input.description,
        input.owners_group_id,
        input.domain_id,
        &valence_from_ctx(&ctx)?,
    )
    .await
    .map_err(|e| map_service_err("update permission", e))?;
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
#[uf_product_macros::server(permission = "GaugeAdmin", step_up)]
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
#[uf_product_macros::server(permission = "GaugeAdmin", step_up)]
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
#[uf_product_macros::server(permission = "GaugeAdmin", step_up)]
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
#[uf_product_macros::server(permission = "GaugeAdmin", step_up)]
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
#[uf_product_macros::server(permission = "GaugeAdmin")]
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
#[uf_product_macros::server(permission = "GaugeAdmin")]
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
#[uf_product_macros::server(permission = "GaugeAdmin")]
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
#[uf_product_macros::server(permission = "GaugeAdmin")]
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

/// Add a child group relationship to a parent group.
#[uf_product_macros::server(permission = "GaugeAdmin", step_up)]
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
#[uf_product_macros::server(permission = "GaugeAdmin", step_up)]
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

/// Paginated Gauge history for the Show History dialog (newest first).
///
/// Stricter than stock [`record_history_leptos::get_record_history_page`]: requires
/// session **and** can-edit on the `permission` / `permission_group` source (via
/// [`gauge::service::actor_can_view_history_subject`]), because parent Read is
/// `AUTHENTICATED` and would otherwise widen history to every signed-in user.
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
        use record_history::RecordHistoryFields;
        use record_history_leptos::server::into_history_row_view;
        use record_history_leptos::{
            clamp_history_page_limit, clamp_history_page_offset, sanitize_kind_filter,
        };

        const MAX_HISTORY_SCAN: u32 = 10_000;

        let ctx = higgs::Higgs::from_request().await?;
        require_session(&ctx)?;
        let v = valence_from_ctx(&ctx)?;

        let table = source.table();
        let record_id = source.id();
        match table {
            "permission" | "permission_group" => {}
            _ => {
                return Err(ServerFnError::new(record_history::HISTORY_ACCESS_DENIED));
            }
        }

        let can_view = gauge::service::actor_can_view_history_subject(table, record_id, &v)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, operation = "get_gauge_history_page", "can-edit check");
                ServerFnError::new("Failed to load record history")
            })?;
        if !can_view {
            return Err(ServerFnError::new(record_history::HISTORY_ACCESS_DENIED));
        }

        // Parent Read ACL + every RecordHistory implementor (including permission_history).
        let mut rows = match record_history::history_for_source(&source, &v).await {
            Ok(rows) => rows,
            Err(record_history::HistoryError::AccessDenied { .. }) => {
                return Err(ServerFnError::new(record_history::HISTORY_ACCESS_DENIED));
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    operation = "get_gauge_history_page",
                    "history_for_source"
                );
                return Err(ServerFnError::new("Failed to load record history"));
            }
        };

        let limit = clamp_history_page_limit(limit);
        let offset = clamp_history_page_offset(offset);
        let kinds = sanitize_kind_filter(kinds);

        rows.sort_by(|a, b| b.changed_at().cmp(a.changed_at()));
        if let Some(kinds) = kinds.as_deref() {
            rows.retain(|r| {
                let Some(id) = r.id.as_ref() else {
                    return false;
                };
                let (kind, _) = record_history::history_row_identity(id);
                kinds.iter().any(|k| k == kind)
            });
        }
        if rows.len() > MAX_HISTORY_SCAN as usize {
            rows.truncate(MAX_HISTORY_SCAN as usize);
        }

        let total_rows = rows.len() as u64;
        let start = offset as usize;
        let fetch_n = (limit as usize).saturating_add(1);
        let page_rows: Vec<_> = rows.into_iter().skip(start).take(fetch_n).collect();
        let db_rows_fetched = page_rows.len() as u32;

        let mut views = Vec::with_capacity(page_rows.len());
        for model in page_rows {
            views.push(into_history_row_view(model, &v).await.map_err(|e| {
                tracing::error!(
                    error = %e,
                    operation = "get_gauge_history_page",
                    "map history row"
                );
                ServerFnError::new("Failed to load record history")
            })?);
        }

        let limit_usize = limit as usize;
        if views.len() > limit_usize {
            views.truncate(limit_usize);
        }

        let items_returned = views.len() as u32;
        let next_request_offset = offset.saturating_add(db_rows_fetched);
        let has_more =
            db_rows_fetched > limit || offset.saturating_add(items_returned) < total_rows as u32;

        Ok(orbital_paging::Page {
            items: views,
            has_more,
            total_count: Some(total_rows),
            next_request_offset: Some(next_request_offset),
        })
    }

    #[cfg(not(feature = "ssr"))]
    {
        let _ = (offset, limit, source, kinds);
        Err(ServerFnError::ServerError(
            "GetGaugeHistoryPage requires SSR".into(),
        ))
    }
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

/// List permission requests created by the current actor.
#[uf_product_macros::server]
pub async fn list_my_permission_requests() -> Result<Vec<PermissionRequestRowDto>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::list_permission_requests_for_actor(&valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("list my permission requests", e))
}

/// List permission requests awaiting current actor review.
#[uf_product_macros::server]
pub async fn list_review_permission_requests() -> Result<Vec<PermissionRequestRowDto>, ServerFnError>
{
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::list_permission_requests_for_review(&valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("list review permission requests", e))
}

/// Get one permission request by id.
#[uf_product_macros::server]
pub async fn get_permission_request(
    /// Unique identifier of the permission request to fetch.
    request_id: String,
) -> Result<Option<PermissionRequestRowDto>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::get_permission_request_detail(&request_id, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("load permission request", e))
}

/// Approve or reject a permission request.
#[uf_product_macros::server(step_up)]
pub async fn decide_permission_request(
    /// Decision (approve/reject) and target request id.
    input: PermissionRequestDecisionInput,
) -> Result<PermissionRequestRowDto, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    gauge::service::decide_permission_request(input, &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("decide permission request", e))
}
