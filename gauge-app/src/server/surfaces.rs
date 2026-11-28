//! Reads behind the Overview, People, Domains, request and service user pages.
//!
//! Each call runs as the signed-in person; Gauge and Lepton apply their own checks. A service
//! user the caller can't see reads as not found, the same as a deleted one.

use gauge::types::{PrincipalAccess, PrincipalRefDto, ServiceUserGroupDto};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
use super::{map_service_err, require_session, valence_from_ctx};

/// The counts on the Overview for Gauge admins.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverviewStats {
    /// People and service users Gauge knows.
    pub people: usize,
    /// Groups.
    pub groups: usize,
    /// Permissions.
    pub permissions: usize,
    /// Changes in the last seven days.
    pub changes_this_week: usize,
    /// The people or change count stopped at its cap, so the page adds a "+".
    pub capped: bool,
}

/// Most pages one count reads before it stops and reports "+".
#[cfg(feature = "ssr")]
const COUNT_PAGE_CAP: usize = 20;

/// The Overview's counts. Gauge admins only; Gauge refuses everyone else.
#[uf_product_macros::server]
pub async fn get_overview_stats() -> Result<OverviewStats, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let v = valence_from_ctx(&ctx)?;
    let mut stats = OverviewStats::default();
    let mut cursor: Option<String> = None;
    for page_no in 0..COUNT_PAGE_CAP {
        let page = gauge::service::list_people(None, cursor.as_deref(), &v)
            .await
            .map_err(|e| map_service_err("count people", e))?;
        stats.people += page.items.len();
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
        stats.capped |= page_no + 1 == COUNT_PAGE_CAP;
    }
    stats.groups = gauge::service::list_groups(&v, None)
        .await
        .map_err(|e| map_service_err("count groups", e))?
        .len();
    stats.permissions = gauge::service::list_permissions(&v, None)
        .await
        .map_err(|e| map_service_err("count permissions", e))?
        .len();
    let filter = gauge::types::ActivityFilter {
        since: Some(chrono::Utc::now() - chrono::Duration::days(7)),
        ..Default::default()
    };
    let mut cursor: Option<String> = None;
    for page_no in 0..COUNT_PAGE_CAP {
        let page = gauge::service::list_activity(filter.clone(), cursor.as_deref(), &v)
            .await
            .map_err(|e| map_service_err("count changes", e))?;
        stats.changes_this_week += page.items.len();
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
        stats.capped |= page_no + 1 == COUNT_PAGE_CAP;
    }
    Ok(stats)
}

/// One page of Gauge's change log for the Activity page and the Recent changes cards. Gauge
/// admins only.
///
/// Takes plain values rather than an `ActivityFilter`, so the call encodes as a simple form.
/// The `days` cutoff comes from the server's clock.
#[uf_product_macros::server]
pub async fn get_activity_window(
    /// What changed, as the Activity page's `?kind=` value; empty for every kind.
    kind: String,
    /// How many days back to reach; `None` for any time.
    days: Option<i64>,
    /// Text to search summaries, people and record names for; empty for none.
    query: String,
    /// The previous page's `next_cursor`.
    cursor: Option<String>,
) -> Result<gauge::types::Page<gauge::types::ActivityRow>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let query = query.trim().to_string();
    let filter = gauge::types::ActivityFilter {
        kinds: crate::pages::activity::kinds_for(&kind),
        since: days.map(|d| chrono::Utc::now() - chrono::Duration::days(d)),
        query: (!query.is_empty()).then_some(query),
        ..Default::default()
    };
    gauge::service::list_activity(filter, cursor.as_deref(), &valence_from_ctx(&ctx)?)
        .await
        .map_err(|e| map_service_err("load activity", e))
}

/// A request target as [`get_request_reviewers`] takes it: `group:<id>` or `permission:<id>`.
#[must_use]
pub fn review_target(kind: &gauge::types::PermissionRequestTargetKind, id: &str) -> String {
    match kind {
        gauge::types::PermissionRequestTargetKind::Group => format!("group:{id}"),
        gauge::types::PermissionRequestTargetKind::Permission => format!("permission:{id}"),
    }
}

/// Names joined for a sentence: "Sol", "Sol or Ines", "Sol, Ines or Kenji".
#[must_use]
pub fn or_list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}

/// Who reviews requests for each target, in words, in the order given: the owners' names when
/// the caller may see them, otherwise "the owners of {group}".
#[uf_product_macros::server]
pub async fn get_request_reviewers(
    /// Targets from [`review_target`].
    targets: Vec<String>,
) -> Result<Vec<String>, ServerFnError> {
    use crate::pages::shared::words::plain_name;
    use gauge::types::PrincipalKind;

    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let v = valence_from_ctx(&ctx)?;
    let people = |group: &gauge::types::PermissionGroupDetailDto, members_own: bool| {
        let mut names: Vec<String> = Vec::new();
        let owners = group.owner_users.iter();
        let members = group
            .members
            .iter()
            .filter(move |_| members_own)
            .filter(|m| m.kind == PrincipalKind::User);
        for principal in owners.chain(members) {
            let name = plain_name(&principal.label);
            if !names.contains(&name) {
                names.push(name);
            }
        }
        names
    };
    let mut out = Vec::with_capacity(targets.len());
    for target in targets {
        let line = if let Some(id) = target.strip_prefix("group:") {
            match gauge::service::get_group_detail(id, &v)
                .await
                .map_err(|e| map_service_err("load who reviews", e))?
            {
                Some(group) => {
                    let names = people(&group, false);
                    if names.is_empty() {
                        format!("the owners of {}", group.name)
                    } else {
                        or_list(&names)
                    }
                }
                None => "its owners".to_string(),
            }
        } else if let Some(id) = target.strip_prefix("permission:") {
            let owners = match gauge::service::get_permission_detail(id, &v)
                .await
                .map_err(|e| map_service_err("load who reviews", e))?
            {
                Some(permission) => {
                    gauge::service::get_group_detail(&permission.owners_group_id, &v)
                        .await
                        .map_err(|e| map_service_err("load who reviews", e))?
                }
                None => None,
            };
            match owners {
                // The owners group's members own the permission.
                Some(group) => {
                    let names = people(&group, true);
                    if names.is_empty() {
                        group.name
                    } else {
                        or_list(&names)
                    }
                }
                None => "its owners".to_string(),
            }
        } else {
            "its owners".to_string()
        };
        out.push(line);
    }
    Ok(out)
}

/// One person's page: who they are and what they can do.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersonPage {
    /// Who it is, with their display name.
    pub principal: PrincipalRefDto,
    /// What they can do, their groups, what they own and their open requests.
    pub access: PrincipalAccess,
}

/// A person's page by user id, or the caller's own for `me`. `None` when no such person
/// exists. Gauge admins read anyone; everyone else only themselves.
#[uf_product_macros::server]
pub async fn get_person_page(
    /// User id, or `me`.
    id: String,
) -> Result<Option<PersonPage>, ServerFnError> {
    use gauge::types::{PrincipalKey, PrincipalKind};
    use valence::Model;

    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let v = valence_from_ctx(&ctx)?;
    // The session id may carry a `user:` table prefix; User rows are keyed by the bare id.
    let bare = |raw: &str| raw.rsplit(':').next().unwrap_or(raw).to_string();
    let me = bare(&ctx.session_user_id().cloned().unwrap_or_default());
    let user_id = if id == "me" { me.clone() } else { bare(&id) };
    let Some(user) = lepton::generated::User::get(&user_id, &v, valence::use_!(r"When someone opens a **person's page in Gauge**, we **load that person's account** to check they exist and show their name. Gauge admins can open anyone's page; everyone else only their own.")).await.map_err(|e| map_service_err("load the person", e))? else {
        return Ok(None);
    };
    let label = gauge::search_sources::user_principal_label(&user, &user_id, &v).await;
    let access = if user_id == me {
        gauge::service::principal_access_self(&v).await
    } else {
        let key = PrincipalKey {
            kind: PrincipalKind::User,
            id: user_id.clone(),
        };
        gauge::service::principal_access(&key, &v).await
    }
    .map_err(|e| map_service_err("load their access", e))?;
    Ok(Some(PersonPage {
        principal: PrincipalRefDto {
            kind: PrincipalKind::User,
            id: user_id,
            label,
            owner_label: None,
        },
        access,
    }))
}

/// A domain on the Domains list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainRow {
    /// Domain id.
    pub id: String,
    /// The name people read ("Gauge" for Gauge's own domain).
    pub name: String,
    /// Description, as Markdown.
    pub description: String,
    /// How many permissions it holds.
    pub permissions: usize,
    /// Its owners' names. Empty when none are set or the caller can't see them.
    pub owners: Vec<String>,
}

/// The name people read for a domain: Gauge's own `gauge` domain reads "Gauge".
#[must_use]
pub fn domain_display_name(name: &str) -> String {
    if name == "gauge" {
        "Gauge".to_string()
    } else {
        name.to_string()
    }
}

/// Every domain the caller can see, apart from the ones Gauge makes for a resource's
/// permissions (those open from their resource), sorted by name.
#[uf_product_macros::server]
pub async fn list_domain_rows() -> Result<Vec<DomainRow>, ServerFnError> {
    use crate::pages::shared::words::plain_name;

    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let v = valence_from_ctx(&ctx)?;
    let domains = gauge::service::list_domains(&v, None)
        .await
        .map_err(|e| map_service_err("list domains", e))?;
    let permissions = gauge::service::list_permissions(&v, None)
        .await
        .map_err(|e| map_service_err("list permissions", e))?;
    let mut rows = Vec::new();
    for domain in domains.into_iter().filter(|d| !d.resource_scoped) {
        let owners = gauge::service::get_domain_detail(&domain.id, &v)
            .await
            .ok()
            .flatten()
            .map(|d| d.owner_users.iter().map(|o| plain_name(&o.label)).collect())
            .unwrap_or_default();
        rows.push(DomainRow {
            permissions: permissions
                .iter()
                .filter(|p| p.domain_id == domain.id)
                .count(),
            name: domain_display_name(&domain.name),
            id: domain.id,
            description: domain.description,
            owners,
        });
    }
    rows.sort_by_key(|r| r.name.to_lowercase());
    Ok(rows)
}

/// A permission in a domain, for the blocked-delete dialog.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPermission {
    /// Permission id.
    pub id: String,
    /// The name people read.
    pub display_name: String,
    /// The key code checks.
    pub key: String,
}

/// The permissions still in `domain_id`, by name.
#[uf_product_macros::server]
pub async fn list_domain_permissions(
    /// Domain id.
    domain_id: String,
) -> Result<Vec<DomainPermission>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let mut out: Vec<DomainPermission> =
        gauge::service::list_permissions(&valence_from_ctx(&ctx)?, None)
            .await
            .map_err(|e| map_service_err("list the domain's permissions", e))?
            .into_iter()
            .filter(|p| p.domain_id == domain_id)
            .map(|p| DomainPermission {
                display_name: if p.display_name.trim().is_empty() {
                    p.name.clone()
                } else {
                    p.display_name
                },
                id: p.id,
                key: p.name,
            })
            .collect();
    out.sort_by_key(|p| p.display_name.to_lowercase());
    Ok(out)
}

/// A permission a service user holds and where it comes from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceUserPermissionRow {
    /// Permission id, when the caller can see the permission.
    pub id: Option<String>,
    /// The name people read.
    pub display_name: String,
    /// Description, as Markdown.
    pub description: String,
    /// The group it comes through; `None` for a grant made straight to the service user.
    pub via_group: Option<ServiceUserGroupDto>,
}

/// Everything the service user page shows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceUserPage {
    /// Service user id.
    pub id: String,
    /// Handle, without the `@`.
    pub handle: String,
    /// Display name.
    pub display_name: String,
    /// Whether it's active. A paused (suspended) one passes no check.
    pub active: bool,
    /// Its owner's display name, when it has one.
    pub owner_label: Option<String>,
    /// The groups it belongs to.
    pub groups: Vec<ServiceUserGroupDto>,
    /// What it can do, each with the group it comes through.
    pub permissions: Vec<ServiceUserPermissionRow>,
}

/// Whether a Gauge error is its not-found refusal.
#[cfg(feature = "ssr")]
fn is_not_found(e: &anyhow::Error) -> bool {
    matches!(
        e.downcast_ref::<gauge::service::GaugeServiceError>(),
        Some(gauge::service::GaugeServiceError::NotFound { .. })
    )
}

/// One service user's page. Only its owner and people who manage every service user get it;
/// for anyone else, and for a deleted or mistyped id, it's `None`.
#[uf_product_macros::server]
pub async fn get_service_user_page(
    /// Service user id.
    id: String,
) -> Result<Option<ServiceUserPage>, ServerFnError> {
    use lepton::generated::{ServiceUserAccount, ServiceUserAccountStatus};
    use lepton_identity::service_users::ServiceUserId;
    use valence::Model;

    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;
    let v = valence_from_ctx(&ctx)?;
    let Ok(service_user) = ServiceUserId::parse(id.trim()) else {
        return Ok(None);
    };
    let Some(account) = ServiceUserAccount::get(service_user.as_str(), &v, valence::use_!(r"When someone opens a **service user's page in Gauge**, we **load the service user** to show its name, handle, owner and whether it's active. Only its owner and people who manage every service user can load it; anyone else sees not found.")).await.map_err(|e| map_service_err("load the service user", e))? else {
        return Ok(None);
    };
    let groups = match gauge::service_users::service_user_groups(&v, &service_user).await {
        Ok(groups) => groups,
        Err(e) if is_not_found(&e) => return Ok(None),
        Err(e) => return Err(map_service_err("load the service user's groups", e)),
    };
    let held =
        match gauge::service_users::service_user_effective_permissions(&v, &service_user).await {
            Ok(held) => held,
            Err(e) if is_not_found(&e) => return Ok(None),
            Err(e) => return Err(map_service_err("load what the service user can do", e)),
        };
    let catalog = gauge::service::list_permissions(&v, None)
        .await
        .map_err(|e| map_service_err("list permissions", e))?;
    let permissions = held
        .into_iter()
        .map(|p| {
            let known = catalog.iter().find(|c| c.name == p.name);
            ServiceUserPermissionRow {
                id: known.map(|c| c.id.clone()),
                display_name: known
                    .map(|c| c.display_name.trim().to_string())
                    .filter(|d| !d.is_empty())
                    .unwrap_or_else(|| p.name.clone()),
                description: p.description,
                via_group: p.via_group,
            }
        })
        .collect();
    let owner_label = match account
        .user()
        .and_then(|owner| valence::extract_id_from_record(owner).ok())
    {
        Some(owner_id) => match lepton::generated::User::get(&owner_id, &v, valence::use_!(r"When someone opens a **service user's page in Gauge**, we **load its owning user** so the page can say who owns it. Only the owner and people who manage every service user see the page.")).await {
            Ok(Some(owner)) => {
                Some(gauge::search_sources::user_principal_label(&owner, &owner_id, &v).await)
            }
            Ok(None) | Err(_) => None,
        },
        None => None,
    };
    Ok(Some(ServiceUserPage {
        id: service_user.as_str().to_string(),
        handle: account.handle().clone(),
        display_name: account.display_name().clone(),
        active: account.status() == &ServiceUserAccountStatus::Active,
        owner_label,
        groups,
        permissions,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn or_list_reads_as_a_sentence_happy() {
        assert_eq!(or_list(&[]), "");
        assert_eq!(or_list(&["Sol".into()]), "Sol");
        assert_eq!(or_list(&["Sol".into(), "Ines".into()]), "Sol or Ines");
        assert_eq!(
            or_list(&["Sol".into(), "Ines".into(), "Kenji".into()]),
            "Sol, Ines or Kenji"
        );
    }

    #[test]
    fn gauge_domain_reads_gauge_happy() {
        assert_eq!(domain_display_name("gauge"), "Gauge");
        assert_eq!(domain_display_name("Billing"), "Billing");
    }

    #[test]
    fn review_targets_name_their_kind_happy() {
        use gauge::types::PermissionRequestTargetKind;
        assert_eq!(
            review_target(&PermissionRequestTargetKind::Group, "g1"),
            "group:g1"
        );
        assert_eq!(
            review_target(&PermissionRequestTargetKind::Permission, "p1"),
            "permission:p1"
        );
    }
}
