//! [`uf_product::access::ResourceAccessBackend`] over Gauge resource bundles.

use std::collections::{BTreeMap, HashMap};

use gauge::resource_permissions::{
    owners_group_id, permission_name, permission_record_id, remove_principal, roles,
    set_principal_role, ResourceAction, ResourceKindDescriptor, ResourcePermissionError,
    UmbrellaPolicy,
};
use gauge::service::GaugeServiceError;
use gauge::types::{
    PermissionRequestCreateInput, PermissionRequestTargetKind, PrincipalKind as GaugeKind,
    PrincipalRefDto,
};
use uf_product::access::{
    AccessEntry, AccessError, AccessView, PrincipalKind, PrincipalRef, PrincipalResults, RequestId,
    ResourceAccessBackend, ResourceRef, Role,
};
use valence::Valence;

use super::kinds::{registered_resource_kind, ResourceKindEntry};

/// Most principals of each kind a search returns.
const SEARCH_LIMIT: u32 = 20;

/// Gauge's implementation of the shared Access card, picker and request flow.
///
/// Reads and writes a resource's bundle as the signed-in caller (no System elevation):
/// the view comes from the bundle's grants and owners group, role changes go through
/// [`set_principal_role`] / [`remove_principal`], search covers people, service users,
/// Gauge groups and managed team groups, and requests go through Gauge's request flow.
/// The resource's kind must be registered with [`super::register_resource_kind`].
///
/// ```rust,ignore
/// uf_product::access::provide_resource_access_backend(std::sync::Arc::new(
///     gauge_app::GaugeResourceAccessBackend,
/// ));
/// ```
pub struct GaugeResourceAccessBackend;

/// A failure inside Gauge or Valence. The detail is logged here; the Access card shows the
/// reader only what was being done, never the internal error text.
fn backend_err(op: &str, e: impl std::fmt::Display) -> AccessError {
    tracing::error!(error = %e, operation = op, "gauge resource access failed");
    AccessError::Backend {
        op: op.to_string(),
        message: "Gauge couldn't finish this. Try again, or ask a Gauge admin if it keeps failing."
            .to_string(),
    }
}

/// A refusal the reader can act on; `message` is written for them and shown as is.
fn user_err(op: &str, message: impl Into<String>) -> AccessError {
    AccessError::Backend {
        op: op.to_string(),
        message: message.into(),
    }
}

fn kind_entry(resource: &ResourceRef) -> Result<ResourceKindEntry, AccessError> {
    registered_resource_kind(&resource.kind).ok_or_else(|| {
        user_err(
            "resolve resource kind",
            format!("Gauge doesn't know resource kind {}", resource.kind),
        )
    })
}

pub(super) fn to_gauge_role(role: &Role) -> Option<roles::Role> {
    match role {
        Role::View => Some(roles::Role::View),
        Role::Run => Some(roles::Role::Run),
        Role::ReadUnmasked => Some(roles::Role::ReadUnmasked),
        Role::Edit => Some(roles::Role::Edit),
        Role::Owner => Some(roles::Role::Owner),
        Role::Custom(_) => None,
    }
}

pub(super) fn from_gauge_role(role: roles::Role) -> Role {
    match role {
        roles::Role::View => Role::View,
        roles::Role::Run => Role::Run,
        roles::Role::ReadUnmasked => Role::ReadUnmasked,
        roles::Role::Edit => Role::Edit,
        roles::Role::Owner => Role::Owner,
        roles::Role::Custom(actions) => Role::Custom(actions),
    }
}

/// The role a principal's direct grants amount to; `None` for no grants.
pub(super) fn role_from_grants(
    kind: ResourceKindDescriptor,
    grants: &[ResourceAction],
) -> Option<Role> {
    roles::role_of(kind, grants).map(from_gauge_role)
}

/// The kind's viewers, editors and operators groups, which a kind-wide bundle grants.
pub(super) fn is_umbrella_group(kind: ResourceKindDescriptor, group_id: &str) -> bool {
    kind.umbrella == UmbrellaPolicy::KindWide
        && [
            kind.groups.viewers,
            kind.groups.editors,
            kind.groups.operators,
        ]
        .contains(&group_id)
}

/// What a request for `role` targets: the owners group for Owner, otherwise each of the
/// role's permissions the caller doesn't hold yet (`held`).
pub(super) fn request_targets(
    kind: ResourceKindDescriptor,
    resource_id: &str,
    role: &Role,
    held: &[ResourceAction],
) -> Result<Vec<(PermissionRequestTargetKind, String)>, AccessError> {
    if *role == Role::Owner {
        return Ok(vec![(
            PermissionRequestTargetKind::Group,
            owners_group_id(kind, resource_id),
        )]);
    }
    let wanted = to_gauge_role(role)
        .and_then(|r| roles::grants_for(kind, &r))
        .ok_or_else(|| user_err("request access", "this resource doesn't offer that role"))?;
    Ok(wanted
        .into_iter()
        .filter(|action| !held.contains(action))
        .map(|action| {
            (
                PermissionRequestTargetKind::Permission,
                permission_record_id(kind, resource_id, action),
            )
        })
        .collect())
}

fn principal_from_dto(dto: &PrincipalRefDto) -> PrincipalRef {
    let kind = match dto.kind {
        GaugeKind::Group => PrincipalKind::Group,
        GaugeKind::ServiceUser => PrincipalKind::ServiceUser {
            owner: dto.owner_label.clone(),
        },
        _ => PrincipalKind::User,
    };
    PrincipalRef {
        kind,
        id: dto.id.clone(),
        label: dto.label.clone(),
    }
}

pub(super) fn gauge_kind(principal: &PrincipalRef) -> GaugeKind {
    match principal.kind {
        PrincipalKind::User => GaugeKind::User,
        PrincipalKind::Group | PrincipalKind::Team { .. } => GaugeKind::Group,
        PrincipalKind::ServiceUser { .. } => GaugeKind::ServiceUser,
    }
}

fn map_role_err(resource: &ResourceRef, op: &str, e: ResourcePermissionError) -> AccessError {
    match e {
        ResourcePermissionError::LastOwner { .. } => AccessError::LastOwner {
            resource: resource.label_with_id(),
        },
        ResourcePermissionError::GaugeService { ref source, .. }
            if matches!(
                source.downcast_ref::<GaugeServiceError>(),
                Some(GaugeServiceError::NotAuthorized { .. })
            ) =>
        {
            AccessError::Denied {
                action: "Maintain".to_string(),
                resource: resource.label_with_id(),
            }
        }
        other => backend_err(op, other),
    }
}

async fn session() -> Result<Valence, AccessError> {
    let ctx = higgs::Higgs::from_request()
        .await
        .map_err(|e| backend_err("read the session", e))?;
    ctx.valence().map_err(|e| backend_err("build Valence", e))
}

async fn actor_holds(
    v: &Valence,
    kind: ResourceKindDescriptor,
    resource_id: &str,
    action: ResourceAction,
) -> Result<bool, AccessError> {
    let _caller = gauge::instrumentation::PermissionCheckCallerGuard::new("resource_access");
    gauge::service::actor_can(v, &permission_name(kind, resource_id, action))
        .await
        .map_err(|e| backend_err("check access", e))
}

async fn held_actions(
    v: &Valence,
    kind: ResourceKindDescriptor,
    resource_id: &str,
) -> Result<Vec<ResourceAction>, AccessError> {
    let mut held = Vec::new();
    for action in kind.actions {
        if actor_holds(v, kind, resource_id, *action).await? {
            held.push(*action);
        }
    }
    Ok(held)
}

/// Managed (team) groups by id: (member count, archived).
///
/// Reads every group and counts each team's members one query at a time; Gauge has no
/// query that filters managed groups or counts members yet. Follow-up rag-gauge-app-fu2 adds one.
async fn managed_groups(v: &Valence) -> Result<HashMap<String, (u32, bool)>, AccessError> {
    let groups = gauge::generated::PermissionGroup::query(v, valence::use_!(r"When someone **searches for who to give access to** in **Gauge**, we **list permission groups** to find the **team groups another product manages** and show them as Teams with their member count. Only group names and counts are shown."))
        .await
        .map_err(|e| backend_err("search teams", e))?;
    let mut out = HashMap::new();
    for group in groups {
        if group.managed_by().is_none() {
            continue;
        }
        let Some(id) = group
            .id()
            .and_then(|t| valence::extract_id_from_record(t).ok())
        else {
            continue;
        };
        let archived = group.managed_status().map(String::as_str) == Some("archived");
        let members = group
            .get_members_record_ids(v, valence::use_!(r"When someone **searches for who to give access to** in **Gauge**, we **count a team group's members** so the picker can show the team's size. Only the count is shown."))
            .await
            .map_err(|e| backend_err("search teams", e))?;
        let count = u32::try_from(members.len()).unwrap_or(u32::MAX);
        out.insert(id, (count, archived));
    }
    Ok(out)
}

async fn search_people(v: &Valence, query: &str) -> Result<Vec<PrincipalRef>, AccessError> {
    use uf_search_core::{SearchSourceKey, SearchSourceRegistry};

    if query.trim().chars().count() < 2 {
        return Ok(Vec::new());
    }
    let registry = SearchSourceRegistry::auto_discover();
    let mut people = Vec::new();
    for (source, is_service_user) in [
        (lepton_shell::people::sources::PEOPLE, false),
        (lepton_shell::people::sources::SERVICE_USERS, true),
    ] {
        let items = registry
            .query_many(
                &[SearchSourceKey::new(source, "")],
                v,
                query.trim(),
                SEARCH_LIMIT,
            )
            .await
            .map_err(|e| backend_err("search people", e))?;
        people.extend(items.into_iter().map(|item| PrincipalRef {
            kind: if is_service_user {
                PrincipalKind::ServiceUser { owner: None }
            } else {
                PrincipalKind::User
            },
            id: item.id,
            label: item.title.trim().to_string(),
        }));
    }
    Ok(people)
}

/// Split Gauge groups into Teams (managed, not archived) and Groups, leaving out resource
/// owners groups, which are plumbing.
pub(super) fn split_groups(
    groups: Vec<(String, String)>,
    managed: &HashMap<String, (u32, bool)>,
) -> (Vec<PrincipalRef>, Vec<PrincipalRef>) {
    let mut teams = Vec::new();
    let mut plain = Vec::new();
    for (id, label) in groups {
        if id.starts_with(crate::resource_access::OWNERS_PREFIX) {
            continue;
        }
        match managed.get(&id) {
            Some((_, true)) => {}
            Some((member_count, false)) => teams.push(PrincipalRef {
                kind: PrincipalKind::Team {
                    org: None,
                    member_count: *member_count,
                },
                id,
                label,
            }),
            None => plain.push(PrincipalRef {
                kind: PrincipalKind::Group,
                id,
                label,
            }),
        }
    }
    (teams, plain)
}

#[async_trait::async_trait]
impl ResourceAccessBackend for GaugeResourceAccessBackend {
    async fn access(&self, resource: &ResourceRef) -> Result<AccessView, AccessError> {
        let entry = kind_entry(resource)?;
        let kind = entry.descriptor;
        let v = session().await?;
        let held = held_actions(&v, kind, &resource.id).await?;

        // Grants per principal. Gauge fills allow lists only for maintainers, so other
        // readers see their own role and an empty table.
        let owners = owners_group_id(kind, &resource.id);
        let mut grants: BTreeMap<String, (PrincipalRef, Vec<ResourceAction>, bool)> =
            BTreeMap::new();
        for action in kind.actions {
            let record = permission_record_id(kind, &resource.id, *action);
            let detail = gauge::service::get_permission_detail(&record, &v)
                .await
                .map_err(|e| backend_err("load access", e))?;
            for dto in detail.map(|d| d.allow_list).unwrap_or_default() {
                if dto.kind == GaugeKind::Group && dto.id == owners {
                    continue;
                }
                let principal = principal_from_dto(&dto);
                grants
                    .entry(principal.key())
                    .or_insert_with(|| (principal, Vec::new(), false))
                    .1
                    .push(*action);
            }
        }
        if let Some(group) = gauge::service::get_group_detail(&owners, &v)
            .await
            .map_err(|e| backend_err("load owners", e))?
        {
            for dto in group.owner_users.iter().chain(&group.members) {
                let principal = principal_from_dto(dto);
                grants
                    .entry(principal.key())
                    .or_insert_with(|| (principal, Vec::new(), false))
                    .2 = true;
            }
        }

        let managed = managed_groups(&v).await?;
        let entries = grants
            .into_values()
            .filter_map(|(mut principal, actions, owner)| {
                let role = if owner {
                    Role::Owner
                } else {
                    role_from_grants(kind, &actions)?
                };
                let is_group = principal.kind == PrincipalKind::Group;
                if let Some((member_count, _)) = managed.get(&principal.id).filter(|_| is_group) {
                    principal.kind = PrincipalKind::Team {
                        org: None,
                        member_count: *member_count,
                    };
                }
                let default_owner =
                    is_group && entry.default_owner_group == Some(principal.id.as_str());
                let umbrella = is_group && is_umbrella_group(kind, &principal.id);
                Some(AccessEntry {
                    principal,
                    role,
                    default_owner,
                    umbrella,
                })
            })
            .collect();

        Ok(AccessView {
            entries,
            roles: roles::Role::offered(kind)
                .into_iter()
                .map(from_gauge_role)
                .collect(),
            viewer_role: role_from_grants(kind, &held),
            can_maintain: held.contains(&ResourceAction::Maintain),
        })
    }

    async fn set_role(
        &self,
        resource: &ResourceRef,
        principal: &PrincipalRef,
        role: Role,
    ) -> Result<(), AccessError> {
        let kind = kind_entry(resource)?.descriptor;
        let role = to_gauge_role(&role)
            .ok_or_else(|| user_err("change access", "custom grants are changed in Gauge"))?;
        let v = session().await?;
        set_principal_role(
            &v,
            kind,
            &resource.id,
            (gauge_kind(principal), &principal.id),
            role,
        )
        .await
        .map_err(|e| map_role_err(resource, "change access", e))
    }

    async fn remove(
        &self,
        resource: &ResourceRef,
        principal: &PrincipalRef,
    ) -> Result<(), AccessError> {
        let kind = kind_entry(resource)?.descriptor;
        let v = session().await?;
        remove_principal(
            &v,
            kind,
            &resource.id,
            (gauge_kind(principal), &principal.id),
        )
        .await
        .map_err(|e| map_role_err(resource, "remove access", e))
    }

    async fn search_principals(&self, query: &str) -> Result<PrincipalResults, AccessError> {
        let v = session().await?;
        let groups = gauge::service::list_groups(&v, Some(query.to_string()))
            .await
            .map_err(|e| backend_err("search groups", e))?
            .into_iter()
            .map(|g| (g.id, g.name))
            .collect();
        let managed = managed_groups(&v).await?;
        let (teams, groups) = split_groups(groups, &managed);
        let people = search_people(&v, query).await?;
        Ok(PrincipalResults {
            teams,
            groups,
            people,
        })
    }

    async fn request(
        &self,
        resource: &ResourceRef,
        role: Role,
        reason: &str,
    ) -> Result<RequestId, AccessError> {
        let kind = kind_entry(resource)?.descriptor;
        let v = session().await?;
        let held = held_actions(&v, kind, &resource.id).await?;
        let targets = request_targets(kind, &resource.id, &role, &held)?;
        let mut first = None;
        for (target_kind, target_id) in targets {
            let row = gauge::service::create_permission_request(
                PermissionRequestCreateInput {
                    target_kind,
                    target_id,
                    reason: reason.to_string(),
                    on_behalf_of: None,
                },
                &v,
            )
            .await
            .map_err(|e| backend_err("send the request", e))?;
            first.get_or_insert(row.id);
        }
        first
            .map(RequestId)
            .ok_or_else(|| user_err("send the request", "you already hold that role"))
    }

    async fn can(&self, resource: &ResourceRef, action: &str) -> Result<bool, AccessError> {
        let kind = kind_entry(resource)?.descriptor;
        let Some(action) = kind.actions.iter().find(|a| a.as_str() == action) else {
            return Ok(false);
        };
        let v = session().await?;
        actor_holds(&v, kind, &resource.id, *action).await
    }
}
