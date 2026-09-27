//! Harness-only seed endpoint for Playwright.

use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use gauge::service;
use gauge::types::{PermissionRequestCreateInput, PermissionRequestTargetKind};
use serde::Deserialize;
use valence::Actor;

use crate::e2e_valence::{e2e_fixtures, e2e_system_valence, store_fixtures, FixtureIds};
use crate::gate_demos::{write_e2e_auth_kind, E2eAuthKind};

#[derive(Debug, Deserialize)]
pub struct SeedRequest {
    /// `anonymous` | `admin` | `requestor` | `outsider` | `unverified`
    #[serde(default = "default_auth")]
    pub auth: String,
    /// When true, mint fresh pending requests for isolation.
    #[serde(default = "default_refresh")]
    pub refresh_requests: bool,
    /// Lab-only: `empty` | `error` forces `list_domains` via `GAUGE_E2E_LIST_DOMAINS`.
    /// Omit or null clears the override.
    #[serde(default)]
    pub list_domains_mode: Option<String>,
    /// When true (default for authenticated seeds), open a valid TOTP sudo window.
    /// Prefer [`Self::step_up_window`] when you need `expired` / `none`.
    #[serde(default)]
    pub grant_step_up_window: Option<bool>,
    /// `valid` | `expired` | `none` | `other_user` — overrides [`Self::grant_step_up_window`].
    #[serde(default)]
    pub step_up_window: Option<String>,
}

fn default_auth() -> String {
    E2eAuthKind::Anonymous.as_str().to_string()
}

fn default_refresh() -> bool {
    false
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StepUpWindowKind {
    None,
    Valid,
    Expired,
    OtherUser,
}

fn resolve_window_kind(body: &SeedRequest, kind: E2eAuthKind) -> StepUpWindowKind {
    if let Some(raw) = body.step_up_window.as_deref() {
        return match raw.trim() {
            "valid" | "ok" | "true" => StepUpWindowKind::Valid,
            "expired" => StepUpWindowKind::Expired,
            "other_user" => StepUpWindowKind::OtherUser,
            "none" | "false" | "" => StepUpWindowKind::None,
            _ => StepUpWindowKind::None,
        };
    }
    match body.grant_step_up_window {
        Some(true) => StepUpWindowKind::Valid,
        Some(false) => StepUpWindowKind::None,
        None => {
            if matches!(
                kind,
                E2eAuthKind::Admin | E2eAuthKind::Requestor | E2eAuthKind::Outsider
            ) {
                StepUpWindowKind::Valid
            } else {
                StepUpWindowKind::None
            }
        }
    }
}

fn bound_user_for_window(kind: StepUpWindowKind, session_user_id: &str) -> String {
    match kind {
        StepUpWindowKind::OtherUser => {
            if session_user_id.contains("admin") {
                "user:requestor".to_string()
            } else {
                "user:admin".to_string()
            }
        }
        _ => session_user_id.to_string(),
    }
}

async fn clear_step_up_window(session: &tower_sessions::Session) {
    let _ = session
        .remove::<i64>(uf_product::permissions::STEP_UP_VERIFIED_AT_KEY)
        .await;
    let _ = session
        .remove::<i64>(uf_product::permissions::STEP_UP_EXPIRES_AT_KEY)
        .await;
    let _ = session
        .remove::<String>(uf_product::permissions::STEP_UP_USER_ID_KEY)
        .await;
    let _ = session
        .remove::<Vec<u8>>(uf_product::permissions::STEP_UP_AUTH_HASH_KEY)
        .await;
    let _ = session
        .remove::<String>(uf_product::permissions::STEP_UP_SCOPE_KEY)
        .await;
}

async fn write_step_up_window(
    session: &tower_sessions::Session,
    session_user_id: &str,
    kind: StepUpWindowKind,
) -> Result<(), StatusCode> {
    let now = Utc::now();
    let (verified_at, expires_at) = match kind {
        StepUpWindowKind::None => return Ok(()),
        StepUpWindowKind::Valid | StepUpWindowKind::OtherUser => (
            now,
            now + chrono::Duration::seconds(uf_product::permissions::STEP_UP_TTL_SECS),
        ),
        StepUpWindowKind::Expired => {
            let expired =
                now - chrono::Duration::seconds(uf_product::permissions::STEP_UP_TTL_SECS + 30);
            (expired, expired)
        }
    };
    let bound_user = bound_user_for_window(kind, session_user_id);
    session
        .insert(
            uf_product::permissions::STEP_UP_VERIFIED_AT_KEY,
            verified_at.timestamp(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    session
        .insert(
            uf_product::permissions::STEP_UP_EXPIRES_AT_KEY,
            expires_at.timestamp(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    session
        .insert(uf_product::permissions::STEP_UP_USER_ID_KEY, bound_user)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    session
        .insert(
            uf_product::permissions::STEP_UP_AUTH_HASH_KEY,
            b"e2e-auth-hash".to_vec(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    session
        .insert(
            uf_product::permissions::STEP_UP_SCOPE_KEY,
            uf_product::permissions::STEP_UP_SCOPE_SENSITIVE.to_string(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(())
}

pub async fn seed_data(
    session: tower_sessions::Session,
    Json(body): Json<SeedRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let kind = E2eAuthKind::parse(&body.auth);
    write_e2e_auth_kind(&session, kind)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    clear_step_up_window(&session).await;
    let window_kind = resolve_window_kind(&body, kind);
    let mut step_up_window = "none";
    if window_kind != StepUpWindowKind::None {
        if let Some(uid) = kind.session_user_id() {
            write_step_up_window(&session, uid, window_kind).await?;
            step_up_window = match window_kind {
                StepUpWindowKind::Valid => "valid",
                StepUpWindowKind::Expired => "expired",
                StepUpWindowKind::OtherUser => "other_user",
                StepUpWindowKind::None => "none",
            };
        }
    }

    match body.list_domains_mode.as_deref() {
        Some("empty") | Some("error") => {
            gauge_app::e2e_lab::set_list_domains_override(body.list_domains_mode.as_deref());
        }
        _ => gauge_app::e2e_lab::set_list_domains_override(None),
    }

    let mut fixtures = e2e_fixtures();
    if body.refresh_requests {
        fixtures = refresh_pending_requests(fixtures).await.map_err(|e| {
            log::error!("seed refresh_requests failed: {e:#}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        store_fixtures(fixtures.clone());
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "auth": kind.as_str(),
        "step_up_window": step_up_window,
        "fixtures": {
            "domain_a_id": fixtures.domain_a_id,
            "domain_b_id": fixtures.domain_b_id,
            "permission_id": fixtures.permission_id,
            "permission_name": fixtures.permission_name,
            "group_id": fixtures.group_id,
            "group_name": fixtures.group_name,
            "pending_perm_request_id": fixtures.pending_perm_request_id,
            "pending_group_request_id": fixtures.pending_group_request_id,
            "child_group_id": fixtures.child_group_id,
        }
    })))
}

async fn refresh_pending_requests(mut fixtures: FixtureIds) -> anyhow::Result<FixtureIds> {
    let system = e2e_system_valence();
    let admin_ctx = system.with_actor(Actor::User {
        user_id: "admin".to_string(),
    });
    let requestor_ctx = system.with_actor(Actor::User {
        user_id: "requestor".to_string(),
    });

    // Grant/membership/owner specs leave requestor on CanDeploy / Deployers;
    // clear those so create_permission_request can mint fresh PENDING rows.
    if let Err(e) =
        service::revoke_permission_from_user(&fixtures.permission_id, "requestor", &admin_ctx).await
    {
        log::debug!("refresh revoke perm user (ok if absent): {e}");
    }
    if let Err(e) = service::revoke_permission_from_group(
        &fixtures.permission_id,
        &fixtures.group_id,
        &admin_ctx,
    )
    .await
    {
        log::debug!("refresh revoke perm group (ok if absent): {e}");
    }
    if let Err(e) =
        service::remove_group_member_user(&fixtures.group_id, "requestor", &admin_ctx).await
    {
        log::debug!("refresh remove group member (ok if absent): {e}");
    }
    if let Err(e) =
        service::remove_group_owner_user(&fixtures.group_id, "requestor", &admin_ctx).await
    {
        log::debug!("refresh remove group owner (ok if absent): {e}");
    }

    // Mint as requestor so outsider unauthorized_viewer can assert deny-by-viewer.
    let perm_row = service::create_permission_request(
        PermissionRequestCreateInput {
            target_kind: PermissionRequestTargetKind::Permission,
            target_id: fixtures.permission_id.clone(),
            reason: format!("e2e refresh perm {}", chrono::Utc::now().timestamp_millis()),
        },
        &requestor_ctx,
    )
    .await?;
    fixtures.pending_perm_request_id = perm_row.id;

    let group_row = service::create_permission_request(
        PermissionRequestCreateInput {
            target_kind: PermissionRequestTargetKind::Group,
            target_id: fixtures.group_id.clone(),
            reason: format!(
                "e2e refresh group {}",
                chrono::Utc::now().timestamp_millis()
            ),
        },
        &requestor_ctx,
    )
    .await?;
    fixtures.pending_group_request_id = group_row.id;

    Ok(fixtures)
}
