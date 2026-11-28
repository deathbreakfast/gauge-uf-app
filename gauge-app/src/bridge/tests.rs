//! Unit tests for the resource-access bridge.

use std::collections::HashMap;

use gauge::resource_permissions::{
    owners_group_id, permission_record_id, KindDefaultGroups, ResourceAction,
    ResourceKindDescriptor, UmbrellaPolicy,
};
use gauge::types::{PermissionRequestTargetKind, PrincipalKind as GaugeKind};
use uf_product::access::{PrincipalKind, PrincipalRef, Role};

use super::kinds::{register_resource_kind, registered_resource_kind, ResourceKindEntry};
use super::resource_access::{
    from_gauge_role, gauge_kind, is_umbrella_group, request_targets, role_from_grants,
    split_groups, to_gauge_role,
};

const JOB: ResourceKindDescriptor = ResourceKindDescriptor {
    prefix: "test_job",
    display_label: "Test job",
    actions: &[
        ResourceAction::View,
        ResourceAction::Run,
        ResourceAction::Edit,
        ResourceAction::Maintain,
    ],
    umbrella: UmbrellaPolicy::KindWide,
    groups: KindDefaultGroups {
        creators: "test.job.creators",
        viewers: "test.job.viewers",
        editors: "test.job.editors",
        operators: "test.job.operators",
    },
    create_permission: "CreateTestJobs",
};

#[test]
fn roles_round_trip_between_gauge_and_uf_product() {
    for role in [
        Role::View,
        Role::Run,
        Role::ReadUnmasked,
        Role::Edit,
        Role::Owner,
    ] {
        assert_eq!(to_gauge_role(&role).map(from_gauge_role), Some(role));
    }
    assert_eq!(to_gauge_role(&Role::Custom(vec!["Edit".into()])), None);
}

#[test]
fn grants_become_roles() {
    use ResourceAction::{Edit, Maintain, Run, View};
    assert_eq!(role_from_grants(JOB, &[View]), Some(Role::View));
    assert_eq!(role_from_grants(JOB, &[View, Run]), Some(Role::Run));
    assert_eq!(role_from_grants(JOB, &[View, Run, Edit]), Some(Role::Edit));
    assert_eq!(
        role_from_grants(JOB, &[View, Run, Edit, Maintain]),
        Some(Role::Owner)
    );
    assert_eq!(
        role_from_grants(JOB, &[Edit]),
        Some(Role::Custom(vec!["Edit".into()]))
    );
    assert_eq!(role_from_grants(JOB, &[]), None);
}

#[test]
fn umbrella_groups_are_the_kinds_defaults() {
    assert!(is_umbrella_group(JOB, "test.job.viewers"));
    assert!(is_umbrella_group(JOB, "test.job.operators"));
    assert!(!is_umbrella_group(JOB, "test.job.creators"));
    let closed = ResourceKindDescriptor {
        umbrella: UmbrellaPolicy::None,
        ..JOB
    };
    assert!(!is_umbrella_group(closed, "test.job.viewers"));
}

#[test]
fn owner_requests_target_the_owners_group() {
    assert_eq!(
        request_targets(JOB, "nightly", &Role::Owner, &[]).ok(),
        Some(vec![(
            PermissionRequestTargetKind::Group,
            owners_group_id(JOB, "nightly")
        )])
    );
}

#[test]
fn role_requests_target_only_missing_permissions() {
    assert_eq!(
        request_targets(JOB, "nightly", &Role::Edit, &[ResourceAction::View]).ok(),
        Some(vec![
            (
                PermissionRequestTargetKind::Permission,
                permission_record_id(JOB, "nightly", ResourceAction::Run)
            ),
            (
                PermissionRequestTargetKind::Permission,
                permission_record_id(JOB, "nightly", ResourceAction::Edit)
            ),
        ])
    );
}

#[test]
fn requests_refuse_roles_the_kind_lacks() {
    assert!(request_targets(JOB, "nightly", &Role::ReadUnmasked, &[]).is_err());
    assert!(request_targets(JOB, "nightly", &Role::Custom(vec![]), &[]).is_err());
}

#[test]
fn search_splits_teams_from_groups() {
    let managed = HashMap::from([
        ("polaron.team.t1".to_string(), (12, false)),
        ("polaron.team.t2".to_string(), (0, true)),
    ]);
    let groups = vec![
        (
            "polaron.team.t1".to_string(),
            "Billing engineering".to_string(),
        ),
        (
            "polaron.team.t2".to_string(),
            "Payments on-call".to_string(),
        ),
        ("deployers".to_string(), "Deployers".to_string()),
        (
            "rp_owners_test_job_x_1".to_string(),
            "Owners of x".to_string(),
        ),
    ];
    let (teams, plain) = split_groups(groups, &managed);
    assert_eq!(
        teams,
        vec![PrincipalRef {
            kind: PrincipalKind::Team {
                org: None,
                member_count: 12
            },
            id: "polaron.team.t1".into(),
            label: "Billing engineering".into(),
        }]
    );
    assert_eq!(plain.len(), 1);
    assert_eq!(plain[0].id, "deployers");
    assert_eq!(plain[0].kind, PrincipalKind::Group);
}

#[test]
fn teams_and_groups_write_as_gauge_groups() {
    let team = PrincipalRef {
        kind: PrincipalKind::Team {
            org: None,
            member_count: 3,
        },
        id: "polaron.team.t1".into(),
        label: "Team".into(),
    };
    assert_eq!(gauge_kind(&team), GaugeKind::Group);
    let bot = PrincipalRef {
        kind: PrincipalKind::ServiceUser { owner: None },
        id: "bot".into(),
        label: "bot".into(),
    };
    assert_eq!(gauge_kind(&bot), GaugeKind::ServiceUser);
}

#[test]
fn registering_a_kind_again_replaces_it() {
    register_resource_kind(ResourceKindEntry {
        descriptor: JOB,
        app: "First",
        href: |id| format!("/first/{id}"),
        default_owner_group: None,
    });
    register_resource_kind(ResourceKindEntry {
        descriptor: JOB,
        app: "Second",
        href: |id| format!("/second/{id}"),
        default_owner_group: Some("admins"),
    });
    let entry = registered_resource_kind("test_job");
    assert_eq!(entry.map(|e| e.app), Some("Second"));
    assert_eq!(entry.map(|e| (e.href)("x")).as_deref(), Some("/second/x"));
    assert!(registered_resource_kind("unknown_kind").is_none());
}
