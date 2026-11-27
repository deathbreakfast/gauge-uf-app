//! Resource bundle and team group fixtures for the resource-access specs.
//!
//! Seeds, as the boot System actor: a Chronon-job-like kind registered with Gauge, one
//! code-declared bundle owned by the "Chronon admins" group (which `admin` owns), an active
//! Polaron team group with Can edit and an archived one with Can run, and a pending request
//! from `requestor` for Can view.

use gauge::resource_permissions::{
    domain_id, ensure_resource_permission_bundle_for_group, permission_record_id, roles::Role,
    set_principal_role, KindDefaultGroups, ResourceAction, ResourceKindDescriptor, UmbrellaPolicy,
};
use gauge::service;
use gauge::types::{
    ManagedStatus, PermissionRequestCreateInput, PermissionRequestTargetKind, PrincipalKind,
};
use valence::{Actor, Valence};

/// A job kind shaped like Chronon's: View, Run, Edit, Maintain.
pub const E2E_JOB: ResourceKindDescriptor = ResourceKindDescriptor {
    prefix: "chronon_job",
    display_label: "Chronon job",
    actions: &[
        ResourceAction::View,
        ResourceAction::Run,
        ResourceAction::Edit,
        ResourceAction::Maintain,
    ],
    umbrella: UmbrellaPolicy::None,
    groups: KindDefaultGroups {
        creators: "chronon.job.creators",
        viewers: "chronon.job.viewers",
        editors: "chronon.job.editors",
        operators: "chronon.job.operators",
    },
    create_permission: "CreateChrononJobs",
};

/// The seeded job's id.
pub const JOB_ID: &str = "billing-rollup";
/// The group that owns the job by default.
pub const ADMIN_GROUP_ID: &str = "chronon.admins";
/// An active team group with Can edit on the job.
pub const TEAM_GROUP_ID: &str = "polaron.team.t81kq2";
/// An archived team group with Can run on the job.
pub const ARCHIVED_TEAM_GROUP_ID: &str = "polaron.team.t2xw9d";

/// Ids the resource-access specs read from the seed response.
#[derive(Clone, Debug, Default)]
pub struct ResourceFixtures {
    pub resource_domain_id: String,
    pub team_group_id: String,
    pub archived_team_group_id: String,
    pub resource_request_id: String,
}

/// Register the job kind so Gauge's pages can link to it.
pub fn register_job_kind() {
    gauge_app::register_resource_kind(gauge_app::ResourceKindEntry {
        descriptor: E2E_JOB,
        app: "Chronon",
        href: |id| format!("/chronon/jobs/{id}"),
        default_owner_group: Some(ADMIN_GROUP_ID),
    });
}

/// Seed the bundle, the team groups and the request. Runs once at host start, as System.
pub async fn seed_resource_access(system: &Valence) -> anyhow::Result<ResourceFixtures> {
    register_job_kind();

    service::create_managed_group(
        ADMIN_GROUP_ID,
        "Chronon admins",
        "Default owners of Chronon jobs",
        Some("admin"),
        None,
        None,
        system,
    )
    .await?;
    ensure_resource_permission_bundle_for_group(
        system,
        E2E_JOB,
        JOB_ID,
        "Billing rollup",
        &[],
        ADMIN_GROUP_ID,
    )
    .await?;

    service::create_managed_group(
        TEAM_GROUP_ID,
        "Billing engineering",
        "Mirrors the Billing engineering team",
        Some("admin"),
        Some("polaron"),
        Some("t81kq2"),
        system,
    )
    .await?;
    for user in ["requestor", "outsider"] {
        service::add_group_member_user(TEAM_GROUP_ID, user, system).await?;
    }
    set_principal_role(
        system,
        E2E_JOB,
        JOB_ID,
        (PrincipalKind::Group, TEAM_GROUP_ID),
        Role::Edit,
    )
    .await?;

    service::create_managed_group(
        ARCHIVED_TEAM_GROUP_ID,
        "Payments on-call",
        "Mirrors the Payments on-call team",
        Some("admin"),
        Some("polaron"),
        Some("t2xw9d"),
        system,
    )
    .await?;
    set_principal_role(
        system,
        E2E_JOB,
        JOB_ID,
        (PrincipalKind::Group, ARCHIVED_TEAM_GROUP_ID),
        Role::Run,
    )
    .await?;
    service::set_managed_group_status(ARCHIVED_TEAM_GROUP_ID, ManagedStatus::Archived, system)
        .await?;

    let requestor = system.with_actor(Actor::User {
        user_id: "requestor".to_string(),
    });
    let request = service::create_permission_request(
        PermissionRequestCreateInput {
            target_kind: PermissionRequestTargetKind::Permission,
            target_id: permission_record_id(E2E_JOB, JOB_ID, ResourceAction::View),
            reason: "need to read billing-rollup's runs".into(),
            on_behalf_of: None,
        },
        &requestor,
    )
    .await?;

    Ok(ResourceFixtures {
        resource_domain_id: domain_id(E2E_JOB, JOB_ID),
        team_group_id: TEAM_GROUP_ID.into(),
        archived_team_group_id: ARCHIVED_TEAM_GROUP_ID.into(),
        resource_request_id: request.id,
    })
}
