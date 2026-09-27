//! Process-wide Valence + Higgs for Playwright (gauge mem + sqlite alias).

use std::sync::{Arc, Mutex, OnceLock};

use chrono::Utc;
use gauge::manifest_sync::{
    sync_permission_manifests, PermissionDomainInput, PermissionInput, PermissionManifestInput,
};
use gauge::service;
use gauge::super_user::SUPER_USER_GROUP_NAME;
use gauge::types::{
    PermissionCreateInput, PermissionDomainCreateInput, PermissionGroupCreateInput,
    PermissionRequestCreateInput, PermissionRequestTargetKind,
};
use higgs::actor_policy::external_actor_json_policy;
use higgs::{HiggsConfig, HiggsValenceFactory};
use valence::{
    register_backend_logical_names, router_key, Actor, DatabaseBackend, DatabaseRouter,
    InMemoryBackend, Model, RecordId, RegisterBackendLogicalNamesOptions, RouterValenceFactory,
    RouterValenceFactoryConfig, Valence, ValenceFactory, MEM_ENGINE_ID, SQLITE_ENGINE_ID,
};

struct E2eState {
    router: Arc<DatabaseRouter>,
    higgs: Arc<HiggsConfig>,
    default_backend_key: String,
    fixtures: Mutex<FixtureIds>,
}

/// Stable fixture ids exposed to seed JSON / Playwright.
#[derive(Clone, Debug, Default)]
pub struct FixtureIds {
    pub domain_a_id: String,
    pub domain_b_id: String,
    pub permission_id: String,
    pub permission_name: String,
    pub group_id: String,
    pub group_name: String,
    pub pending_perm_request_id: String,
    pub pending_group_request_id: String,
    pub child_group_id: String,
}

static E2E_STATE: OnceLock<Arc<E2eState>> = OnceLock::new();

struct HiggsFactory(RouterValenceFactory);

impl HiggsValenceFactory for HiggsFactory {
    fn build(&self, actor_json: &serde_json::Value) -> anyhow::Result<Valence> {
        self.0.build(actor_json).map_err(|e| anyhow::anyhow!("{e}"))
    }
}

fn prepare_env() {
    valence::deletion::register_noop_deletion_dispatcher_for_tests();
    valence::clear_for_test();
    // SAFETY: host boot only.
    unsafe {
        if std::env::var_os("VALENCE_OWNERSHIP_UNIFIED_FETCH").is_none() {
            std::env::set_var("VALENCE_OWNERSHIP_UNIFIED_FETCH", "0");
        }
    }
}

fn record_pk_id(rid: Option<&RecordId>) -> String {
    rid.and_then(|r| valence::extract_id_from_record(r).ok())
        .unwrap_or_default()
}

async fn seed_user(id: &str, email_verified: bool, valence: &Valence) {
    let now = Utc::now();
    let confirmed_at = email_verified.then_some(now);
    let user = lepton::generated::User::new(
        Some(lepton::generated::UserUserType::Person),
        Some("e2e-password-hash".to_string()),
        Some(lepton::generated::UserStatus::Active),
        None,
        None,
        confirmed_at,
        None,
        None,
        now,
        now,
    )
    .expect("build user");
    lepton::generated::User::upsert(id, user, valence, valence::use_!(r#"**Test:** Fixture **User** save for `e2e_valence` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."#))
        .await
        .expect("upsert user");
}

async fn seed_super_user_with_member(system: &Valence, member_user_id: &str) {
    let super_group = gauge::generated::PermissionGroup::new(
        SUPER_USER_GROUP_NAME.to_string(),
        Some("super users".to_string()),
        Utc::now(),
        Utc::now(),
    )
    .expect("build super user group");
    let created =
        gauge::generated::PermissionGroup::upsert("super_user_group", super_group, system, valence::use_!(r#"**Test:** Fixture **Permission Group** save for `e2e_valence` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."#))
            .await
            .expect("upsert super user group");

    let member = lepton::generated::User::get(member_user_id, system, valence::use_!(r#"**Test:** Fixture **User** load for `e2e_valence` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."#))
        .await
        .expect("query member")
        .expect("member exists");
    let principal = gauge::generated::PermissionUserPrincipal::upsert(
        &format!("user:{member_user_id}"),
        gauge::generated::PermissionUserPrincipal::new(
            member.id().expect("member id").clone(),
            member_user_id.to_string(),
        )
        .expect("new principal"),
        system,
        valence::use_!(r#"**Test:** Fixture **Permission User Principal** save for `e2e_valence` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."#),
    )
    .await
    .expect("upsert principal");
    created
        .relate_to_owner_record(
            principal.id().expect("principal id"),
            system,
            valence::use_!(r#"**Test:** Fixture **owner edge** write in `e2e_valence` so the suite can arrange Gauge super-user membership. CI and developers running the suite only."#),
        )
        .await
        .expect("relate super owner");
    created
        .relate_to_member_record(
            principal.id().expect("principal id"),
            system,
            valence::use_!(r#"**Test:** Fixture **member edge** write in `e2e_valence` so the suite can arrange Gauge super-user membership. CI and developers running the suite only."#),
        )
        .await
        .expect("relate super member");
}

async fn demote_admin_from_super_user(system: &Valence) {
    let Some(super_group) = gauge::generated::PermissionGroup::get("super_user_group", system, valence::use_!(r#"**Test:** Fixture **Permission Group** load for `e2e_valence` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."#))
        .await
        .expect("get super user group")
    else {
        return;
    };
    let Some(principal) = gauge::generated::PermissionUserPrincipal::get("user:admin", system, valence::use_!(r#"**Test:** Fixture **Permission User Principal** load for `e2e_valence` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."#))
        .await
        .expect("get admin principal")
    else {
        return;
    };
    let pid = principal.id().expect("principal id").clone();
    let _ = super_group
        .unrelate_from_member_record(
            &pid,
            system,
            valence::use_!(r#"**Test:** Fixture **member edge** remove in `e2e_valence` so the suite can demote a user from super-user after grants. CI and developers running the suite only."#),
        )
        .await;
    let _ = super_group
        .unrelate_from_owner_record(
            &pid,
            system,
            valence::use_!(r#"**Test:** Fixture **owner edge** remove in `e2e_valence` so the suite can demote a user from super-user after grants. CI and developers running the suite only."#),
        )
        .await;
}

fn gauge_admin_manifest() -> PermissionManifestInput {
    PermissionManifestInput {
        app_id: "permission".into(),
        domains: vec![PermissionDomainInput {
            key: "gauge".into(),
            name: "Gauge".into(),
            description: "Permission administration".into(),
            permissions: vec![PermissionInput {
                name: "GaugeAdmin".into(),
                description: "Administer permissions, groups, and domains".into(),
            }],
        }],
    }
}

/// Build shared Valence/Higgs once and seed baseline fixtures.
pub async fn init_e2e_valence() {
    if E2E_STATE.get().is_some() {
        return;
    }

    prepare_env();

    let backend: Arc<dyn DatabaseBackend> = Arc::new(InMemoryBackend::new());
    let mut router = DatabaseRouter::new();
    register_backend_logical_names(
        &mut router,
        Arc::clone(&backend),
        gauge::embedded_surreal::EMBEDDED_SURREAL_LOGICAL_NAMES,
        RegisterBackendLogicalNamesOptions {
            register_alias_engine_id: Some(SQLITE_ENGINE_ID),
        },
    );
    router.register(
        router_key(gauge::embedded_surreal::LOGICAL_NAME, SQLITE_ENGINE_ID),
        backend,
    );
    let router = Arc::new(router);
    let default_key = router_key(gauge::embedded_surreal::LOGICAL_NAME, MEM_ENGINE_ID);

    let system = Valence::builder()
        .database_router(Arc::clone(&router))
        .default_backend_key(default_key.clone())
        .with_actor(Actor::System {
            operation: "e2e_gauge_host".into(),
        })
        .build()
        .expect("e2e Valence");

    seed_user("admin", true, &system).await;
    seed_user("requestor", true, &system).await;
    seed_user("outsider", true, &system).await;
    seed_user("unverified", false, &system).await;
    // Super User is required to grant GaugeAdmin once; strip afterward so the
    // Needs Review inbox is non-empty (super users get an empty review queue).
    seed_super_user_with_member(&system, "admin").await;

    sync_permission_manifests(&system, &[gauge_admin_manifest()])
        .await
        .expect("sync GaugeAdmin manifest");

    let admin_ctx = system.with_actor(Actor::User {
        user_id: "admin".to_string(),
    });

    let perms = service::list_permissions(&admin_ctx, None)
        .await
        .expect("list permissions");
    let gauge_admin = perms
        .into_iter()
        .find(|p| p.name == "GaugeAdmin")
        .expect("GaugeAdmin after sync");
    service::grant_permission_to_user(&gauge_admin.id, "admin", &admin_ctx)
        .await
        .expect("grant GaugeAdmin to admin");

    let fixtures = bootstrap_fixtures(&system, &admin_ctx)
        .await
        .expect("bootstrap fixtures");

    demote_admin_from_super_user(&system).await;

    let factory: Arc<dyn HiggsValenceFactory> = Arc::new(HiggsFactory(RouterValenceFactory::new(
        Arc::clone(&router),
        RouterValenceFactoryConfig::new(default_key.clone())
            .actor_json_policy(external_actor_json_policy()),
    )));
    let higgs = Arc::new(
        HiggsConfig::builder()
            .valence_factory_arc(factory)
            .build()
            .expect("e2e HiggsConfig"),
    );

    let state = Arc::new(E2eState {
        router,
        higgs,
        default_backend_key: default_key,
        fixtures: Mutex::new(fixtures),
    });
    let _ = E2E_STATE.set(state);
}

async fn bootstrap_fixtures(system: &Valence, admin_ctx: &Valence) -> anyhow::Result<FixtureIds> {
    let domain_a = service::create_domain(
        PermissionDomainCreateInput {
            name: "Ops".into(),
            description: "e2e domain A".into(),
        },
        admin_ctx,
    )
    .await?;
    let domain_b = service::create_domain(
        PermissionDomainCreateInput {
            name: "Platform".into(),
            description: "e2e domain B".into(),
        },
        admin_ctx,
    )
    .await?;

    let group = service::create_group(
        PermissionGroupCreateInput {
            name: "Deployers".into(),
            description: "e2e deployers group".into(),
        },
        admin_ctx,
    )
    .await?;
    let group_id = record_pk_id(group.id());

    let child = service::create_group(
        PermissionGroupCreateInput {
            name: "NestedDeployers".into(),
            description: "child group for nested member tests".into(),
        },
        admin_ctx,
    )
    .await?;

    let permission = service::create_permission(
        PermissionCreateInput {
            name: "CanDeploy".into(),
            description: "e2e deploy permission".into(),
            domain_id: record_pk_id(domain_a.id()),
            owners_group_id: String::new(),
        },
        admin_ctx,
    )
    .await?;
    let permission_id = record_pk_id(permission.id());

    let requestor_ctx = system.with_actor(Actor::User {
        user_id: "requestor".to_string(),
    });
    let pending_perm = service::create_permission_request(
        PermissionRequestCreateInput {
            target_kind: PermissionRequestTargetKind::Permission,
            target_id: permission_id.clone(),
            reason: "need deploy access for e2e".into(),
        },
        &requestor_ctx,
    )
    .await?;
    let pending_group = service::create_permission_request(
        PermissionRequestCreateInput {
            target_kind: PermissionRequestTargetKind::Group,
            target_id: group_id.clone(),
            reason: "need group membership for e2e".into(),
        },
        &requestor_ctx,
    )
    .await?;

    Ok(FixtureIds {
        domain_a_id: record_pk_id(domain_a.id()),
        domain_b_id: record_pk_id(domain_b.id()),
        permission_id,
        permission_name: "CanDeploy".into(),
        group_id,
        group_name: "Deployers".into(),
        pending_perm_request_id: pending_perm.id.clone(),
        pending_group_request_id: pending_group.id.clone(),
        child_group_id: record_pk_id(child.id()),
    })
}

fn state() -> Arc<E2eState> {
    E2E_STATE
        .get()
        .expect("init_e2e_valence must run first")
        .clone()
}

pub fn e2e_router() -> Arc<DatabaseRouter> {
    Arc::clone(&state().router)
}

pub fn e2e_higgs_config() -> Arc<HiggsConfig> {
    Arc::clone(&state().higgs)
}

pub fn e2e_fixtures() -> FixtureIds {
    state().fixtures.lock().expect("fixtures").clone()
}

pub fn store_fixtures(fixtures: FixtureIds) {
    *state().fixtures.lock().expect("fixtures") = fixtures;
}

pub fn e2e_system_valence() -> Valence {
    Valence::builder()
        .database_router(e2e_router())
        .default_backend_key(state().default_backend_key.clone())
        .with_actor(Actor::System {
            operation: "e2e_seed".into(),
        })
        .build()
        .expect("system valence")
}
