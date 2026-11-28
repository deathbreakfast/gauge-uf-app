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
    /// Lets the axum-login backend load users as a System actor, the way lepton hosts do;
    /// the request factory refuses System actors from outside.
    auth_valence_factory: Arc<dyn HiggsValenceFactory>,
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
    /// Active service user owned by `requestor`.
    pub service_user_handle: String,
    /// Active service user owned by `admin`, which `requestor` may not request for.
    pub foreign_service_user_handle: String,
    /// Id of [`Self::service_user_handle`].
    pub service_user_id: String,
    /// Id of the paused service user owned by `requestor`.
    pub paused_service_user_id: String,
    /// Id of [`Self::foreign_service_user_handle`].
    pub foreign_service_user_id: String,
    /// Resource bundle, team groups and resource request.
    pub resource: crate::resource_access_seed::ResourceFixtures,
    /// "Release Captains": owned by `admin` and `outsider` (who isn't a Gauge admin); members
    /// `requestor` and the nested group "On-call SRE", whose member is `kenji`.
    pub release_group_id: String,
    /// "Approve releases" (`ApproveRelease`), on Release Captains' allow list.
    pub release_permission_id: String,
}

static E2E_STATE: OnceLock<Arc<E2eState>> = OnceLock::new();

/// The in-memory backend, except that declaring a unique index succeeds without enforcing it.
///
/// `InMemoryBackend` refuses unique indexes, and lepton's `service_user_account` declares one on
/// `handle_key`. Lepton's create path already probes for a taken handle before inserting, which
/// is enough for the fixed fixture handles here. Everything else goes straight through.
#[derive(Debug)]
struct UniqueTolerantMemBackend(InMemoryBackend);

#[async_trait::async_trait]
impl DatabaseBackend for UniqueTolerantMemBackend {
    fn engine_id(&self) -> &'static str {
        self.0.engine_id()
    }
    fn capabilities(&self) -> valence::BackendCapabilities {
        self.0.capabilities()
    }
    fn as_any_local(&self) -> Option<&dyn std::any::Any> {
        self.0.as_any_local()
    }
    async fn use_namespace(&self, ns: &str, db_name: &str) -> valence::Result<()> {
        self.0.use_namespace(ns, db_name).await
    }
    async fn execute_compiled_query(
        &self,
        compiled: &valence::CompiledQuery,
    ) -> valence::Result<Vec<serde_json::Value>> {
        self.0.execute_compiled_query(compiled).await
    }
    async fn ensure_schemaless_table(&self, table: &str) -> valence::Result<()> {
        self.0.ensure_schemaless_table(table).await
    }
    async fn inspect_typed_layout(
        &self,
        table: &str,
    ) -> valence::Result<Option<valence::storage_layout::StorageLayout>> {
        self.0.inspect_typed_layout(table).await
    }
    async fn ensure_typed_table(
        &self,
        layout: &valence::storage_layout::StorageLayout,
    ) -> valence::Result<()> {
        self.0.ensure_typed_table(layout).await
    }
    async fn sync_typed_table(
        &self,
        layout: &valence::storage_layout::StorageLayout,
    ) -> valence::Result<()> {
        self.0.sync_typed_table(layout).await
    }
    async fn read_schema_version(&self, table: &str) -> valence::Result<Option<String>> {
        self.0.read_schema_version(table).await
    }
    async fn write_schema_version(&self, table: &str, version: &str) -> valence::Result<()> {
        self.0.write_schema_version(table, version).await
    }
    async fn get_record(
        &self,
        table: &str,
        id: &str,
    ) -> valence::Result<Option<serde_json::Value>> {
        self.0.get_record(table, id).await
    }
    async fn create_record(
        &self,
        table: &str,
        content: serde_json::Value,
    ) -> valence::Result<serde_json::Value> {
        self.0.create_record(table, content).await
    }
    async fn update_record(
        &self,
        table: &str,
        id: &str,
        content: serde_json::Value,
    ) -> valence::Result<serde_json::Value> {
        self.0.update_record(table, id, content).await
    }
    async fn merge_record(
        &self,
        table: &str,
        id: &str,
        patch: serde_json::Value,
    ) -> valence::Result<serde_json::Value> {
        self.0.merge_record(table, id, patch).await
    }
    async fn upsert_record(
        &self,
        table: &str,
        id: &str,
        content: serde_json::Value,
    ) -> valence::Result<serde_json::Value> {
        self.0.upsert_record(table, id, content).await
    }
    async fn delete_record(&self, table: &str, id: &str) -> valence::Result<()> {
        self.0.delete_record(table, id).await
    }
    async fn relate_edge(
        &self,
        from: &RecordId,
        edge_table: &str,
        to: &RecordId,
    ) -> valence::Result<()> {
        self.0.relate_edge(from, edge_table, to).await
    }
    async fn unrelate_edge(
        &self,
        from: &RecordId,
        edge_table: &str,
        to: &RecordId,
    ) -> valence::Result<()> {
        self.0.unrelate_edge(from, edge_table, to).await
    }
    async fn get_edge_targets(
        &self,
        from: &RecordId,
        edge_table: &str,
    ) -> valence::Result<Vec<RecordId>> {
        self.0.get_edge_targets(from, edge_table).await
    }
    async fn get_edge_sources(
        &self,
        to: &RecordId,
        edge_table: &str,
    ) -> valence::Result<Vec<RecordId>> {
        self.0.get_edge_sources(to, edge_table).await
    }
    async fn define_unique_index(&self, _table: &str, _field: &str) -> valence::Result<()> {
        Ok(())
    }
    fn ttl_capability(&self) -> valence::ttl::BackendTtlCapability {
        self.0.ttl_capability()
    }
    async fn apply_ttl_policy(
        &self,
        table: &str,
        policy: &valence::ttl::SchemaTtlPolicy,
    ) -> valence::Result<()> {
        self.0.apply_ttl_policy(table, policy).await
    }
}

struct HiggsFactory(RouterValenceFactory);

/// Session actors through the external-actor policy; Higgs's own System builds (Lepton's
/// step-up) through a factory without it.
struct SplitFactory {
    session: Arc<dyn HiggsValenceFactory>,
    system: Arc<dyn HiggsValenceFactory>,
}

impl HiggsValenceFactory for SplitFactory {
    fn build(&self, actor_json: &serde_json::Value) -> anyhow::Result<Valence> {
        if actor_json.get("System").is_some() {
            log::info!("e2e higgs: system valence for {actor_json}");
            return self.system.build(actor_json);
        }
        self.session.build(actor_json)
    }
}

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
        // Lets Lepton re-seal the lab admin's plaintext TOTP secret after a good code.
        std::env::set_var("LEPTON_TOTP_ALLOW_TEST_SEAL_KEY", "1");
        if std::env::var_os("VALENCE_OWNERSHIP_UNIFIED_FETCH").is_none() {
            std::env::set_var("VALENCE_OWNERSHIP_UNIFIED_FETCH", "0");
        }
    }
}

/// RFC 6238 secret (base32) of the lab admin's authenticator. Playwright computes codes
/// from it (`fixtures.ts`, `totpCode`).
pub const HARNESS_TOTP_SECRET: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

/// The usability fixture: a permission held through a nested group, a group with an owner
/// who isn't a Gauge admin, and two Super User owners who aren't Gauge admins (`kenji`, who
/// signs in as the `kenji` seed, and `noor`). Runs while `admin` is
/// still Super User. Returns `(release_group_id, release_permission_id)`.
async fn seed_release_captains(
    admin_ctx: &Valence,
    domain_id: &str,
) -> anyhow::Result<(String, String)> {
    let captains = service::create_group(
        PermissionGroupCreateInput {
            name: "Release Captains".into(),
            description: "People who run the weekly release train.".into(),
        },
        admin_ctx,
    )
    .await?;
    let captains_id = record_pk_id(captains.id());
    let sre = service::create_group(
        PermissionGroupCreateInput {
            name: "On-call SRE".into(),
            description: "Whoever is on call this week.".into(),
        },
        admin_ctx,
    )
    .await?;
    let sre_id = record_pk_id(sre.id());
    service::add_group_member_group(&captains_id, &sre_id, admin_ctx).await?;
    service::add_group_member_user(&sre_id, "kenji", admin_ctx).await?;
    reset_release_captains(admin_ctx, &captains_id).await?;
    let permission = service::create_permission(
        PermissionCreateInput {
            display_name: "Approve releases".into(),
            name: "ApproveRelease".into(),
            description: "Sign off a release **before** it ships.".into(),
            domain_id: domain_id.to_string(),
            owners_group_id: String::new(),
        },
        admin_ctx,
    )
    .await?;
    let permission_id = record_pk_id(permission.id());
    service::grant_permission_to_group(&permission_id, &captains_id, admin_ctx).await?;
    service::add_group_owner_user(gauge::super_user::SUPER_USER_GROUP_ID, "admin", admin_ctx)
        .await?;
    service::add_group_owner_user(gauge::super_user::SUPER_USER_GROUP_ID, "kenji", admin_ctx)
        .await?;
    Ok((captains_id, permission_id))
}

/// "Release bots": holds Approve releases, with `requestor`'s service user as its member, so
/// the service user page has a permission that comes through a group. Runs while `admin` is
/// still Super User.
async fn seed_release_bots(
    admin_ctx: &Valence,
    release_permission_id: &str,
    service_user_id: &str,
) -> anyhow::Result<()> {
    let bots = service::create_group(
        PermissionGroupCreateInput {
            name: "Release bots".into(),
            description: "Service users that post release timings.".into(),
        },
        admin_ctx,
    )
    .await?;
    let bots_id = record_pk_id(bots.id());
    service::grant_permission_to_group(release_permission_id, &bots_id, admin_ctx).await?;
    let service_user = lepton_identity::service_users::ServiceUserId::parse(service_user_id)?;
    service::add_group_member_service_user(&bots_id, &service_user, admin_ctx).await
}

/// Put Release Captains back as specs expect it: `requestor` a member, `outsider` an owner.
pub async fn reset_release_captains(admin_ctx: &Valence, captains_id: &str) -> anyhow::Result<()> {
    service::add_group_member_user(captains_id, "requestor", admin_ctx).await?;
    service::add_group_owner_user(captains_id, "outsider", admin_ctx).await?;
    Ok(())
}

/// Give `admin` an enabled authenticator so specs can type real step-up codes.
async fn enroll_admin_authenticator(system: &Valence) {
    use lepton_host_adapter::generated::TotpFactor;

    let now = Utc::now();
    let factor = TotpFactor::new(
        RecordId::new("user", "admin"),
        HARNESS_TOTP_SECRET.to_string(),
        None,
        None,
        None,
        Some(now),
        Some(now),
        now,
        now,
    )
    .expect("totp factor");
    TotpFactor::upsert("admin_totp", factor, system, valence::use_!(r#"**Test:** Fixture **Totp Factor** save for `e2e_valence` so Playwright can type real step-up codes as the lab admin. CI and developers running the suite only."#))
        .await
        .expect("enroll admin authenticator");
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

/// Display names the person pickers search by, one per seeded user.
const DISPLAY_NAMES: [(&str, &str); 6] = [
    ("admin", "Ada Admin"),
    ("requestor", "Rae Requestor"),
    ("outsider", "Otto Outsider"),
    ("unverified", "Uma Unverified"),
    ("kenji", "Kenji Watanabe"),
    ("noor", "Noor Haddad"),
];

/// Give seeded user `id` a profile, since people search finds people by display name.
async fn seed_profile(id: &str, display_name: &str, valence: &Valence) {
    let now = Utc::now();
    let profile = lepton_identity::generated::UserProfile::new(
        RecordId::new("user", id),
        display_name.to_string(),
        display_name.to_string(),
        now,
        now,
        None,
    )
    .expect("build profile");
    lepton_identity::generated::UserProfile::create(profile, valence, valence::use_!(r#"**Test:** Fixture **UserProfile** save for `e2e_valence` so the person picker specs can find seeded users by display name. CI and developers running the suite only."#))
        .await
        .expect("create profile");
}

/// Handle of the service user `requestor` owns.
const REQUESTOR_SERVICE_USER: &str = "requestor-agent";
/// Make the service user `admin` owns a member of the fixture group. Safe to repeat.
pub async fn ensure_service_user_member(fixtures: &FixtureIds) -> anyhow::Result<()> {
    use lepton_identity::service_users::ServiceUserId;

    let admin_ctx = e2e_system_valence().with_actor(Actor::User {
        user_id: "admin".to_string(),
    });
    let owned = lepton_auth::service_users::list_my_service_users(&admin_ctx).await?;
    let account = owned
        .iter()
        .find(|su| *su.handle() == fixtures.foreign_service_user_handle)
        .ok_or_else(|| anyhow::anyhow!("fixture service user not found"))?;
    let service_user = ServiceUserId::parse(&record_pk_id(account.id()))?;
    service::add_group_member_service_user(&fixtures.group_id, &service_user, &admin_ctx).await
}

/// Handle of the service user `admin` owns.
const ADMIN_SERVICE_USER: &str = "admin-agent";

/// Create one active service user for `requestor` and one for `admin` through lepton, as
/// their owners. Runs while `admin` is still a Super User: `admin` may create through that, and
/// `requestor` joins "Service user creators" only for its create, so the open-by-name specs
/// still see a non-member.
async fn seed_service_users(system: &Valence) {
    use gauge::service_users::SERVICE_USER_CREATORS_GROUP_ID;
    use lepton_auth::service_users::{create_service_user, NewServiceUser};

    let admin_ctx = system.with_actor(Actor::User {
        user_id: "admin".to_string(),
    });
    let requestor_ctx = system.with_actor(Actor::User {
        user_id: "requestor".to_string(),
    });
    service::add_group_member_user(SERVICE_USER_CREATORS_GROUP_ID, "requestor", &admin_ctx)
        .await
        .expect("requestor joins service user creators");
    for (owner_ctx, handle, display_name) in [
        (&requestor_ctx, REQUESTOR_SERVICE_USER, "Requestor's agent"),
        (&admin_ctx, ADMIN_SERVICE_USER, "Admin's agent"),
        (
            &requestor_ctx,
            REQUESTOR_PAUSED_SERVICE_USER,
            "Paused agent",
        ),
    ] {
        create_service_user(
            owner_ctx,
            NewServiceUser {
                handle,
                display_name,
                description: None,
            },
        )
        .await
        .unwrap_or_else(|e| panic!("create service user {handle}: {e}"));
    }
    let paused = service_user_id(&requestor_ctx, REQUESTOR_PAUSED_SERVICE_USER)
        .await
        .expect("paused service user id");
    lepton_auth::service_users::suspend_service_user(
        &requestor_ctx,
        &lepton_identity::service_users::ServiceUserId::parse(&paused)
            .expect("paused service user id parses"),
    )
    .await
    .expect("pause the paused service user");
    service::remove_group_member_user(SERVICE_USER_CREATORS_GROUP_ID, "requestor", &admin_ctx)
        .await
        .expect("requestor leaves service user creators");
}

/// Handle of the paused (suspended) service user `requestor` owns.
const REQUESTOR_PAUSED_SERVICE_USER: &str = "requestor-paused";

/// Id of the service user `owner` owns with `handle`.
async fn service_user_id(owner: &Valence, handle: &str) -> anyhow::Result<String> {
    lepton_auth::service_users::list_my_service_users(owner)
        .await?
        .iter()
        .find(|su| su.handle() == handle)
        .map(|su| record_pk_id(su.id()))
        .ok_or_else(|| anyhow::anyhow!("service user {handle} not found"))
}

async fn seed_super_user_with_member(system: &Valence, member_user_id: &str) {
    let super_group = gauge::generated::PermissionGroup::new(
        None,
        None,
        None,
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

    let backend: Arc<dyn DatabaseBackend> =
        Arc::new(UniqueTolerantMemBackend(InMemoryBackend::new()));
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
    seed_user("kenji", true, &system).await;
    seed_user("noor", true, &system).await;
    for (id, display_name) in DISPLAY_NAMES {
        seed_profile(id, display_name, &system).await;
    }
    // Super User is required to grant GaugeAdmin once; strip afterward so the
    // Needs Review inbox is non-empty (super users get an empty review queue).
    seed_super_user_with_member(&system, "admin").await;

    sync_permission_manifests(&system, &[gauge_admin_manifest()])
        .await
        .expect("sync GaugeAdmin manifest");
    // The "Service user creators" group, which `requestor` is not a member of.
    let seeded = gauge::service_users::seed_service_user_permissions(&system)
        .await
        .expect("seed service user permissions");
    log::debug!("seeded service user permissions: {seeded:?}");
    if gauge::service_users::install_gauge_service_user_gate().is_err() {
        log::debug!("service user gate already installed");
    }
    seed_service_users(&system).await;

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

    let mut fixtures = bootstrap_fixtures(&system, &admin_ctx)
        .await
        .expect("bootstrap fixtures");
    fixtures.resource = crate::resource_access_seed::seed_resource_access(&system)
        .await
        .expect("seed resource access");

    let (release_group_id, release_permission_id) =
        seed_release_captains(&admin_ctx, &fixtures.domain_a_id)
            .await
            .expect("seed release captains");
    fixtures.release_group_id = release_group_id;
    fixtures.release_permission_id = release_permission_id;
    let requestor_ctx = system.with_actor(Actor::User {
        user_id: "requestor".to_string(),
    });
    fixtures.service_user_id = service_user_id(&requestor_ctx, REQUESTOR_SERVICE_USER)
        .await
        .expect("service user id");
    fixtures.paused_service_user_id =
        service_user_id(&requestor_ctx, REQUESTOR_PAUSED_SERVICE_USER)
            .await
            .expect("paused service user id");
    fixtures.foreign_service_user_id = service_user_id(&admin_ctx, ADMIN_SERVICE_USER)
        .await
        .expect("foreign service user id");
    seed_release_bots(
        &admin_ctx,
        &fixtures.release_permission_id,
        &fixtures.service_user_id,
    )
    .await
    .expect("seed release bots");

    demote_admin_from_super_user(&system).await;
    enroll_admin_authenticator(&system).await;

    let factory: Arc<dyn HiggsValenceFactory> = Arc::new(HiggsFactory(RouterValenceFactory::new(
        Arc::clone(&router),
        RouterValenceFactoryConfig::new(default_key.clone())
            .actor_json_policy(external_actor_json_policy()),
    )));
    let auth_valence_factory: Arc<dyn HiggsValenceFactory> =
        Arc::new(HiggsFactory(RouterValenceFactory::new(
            Arc::clone(&router),
            RouterValenceFactoryConfig::new(default_key.clone()),
        )));
    // Lepton's step-up reads TOTP factors through `Higgs::unsafe_system_valence`, which
    // the external-actor policy refuses. System builds go to a plain router factory, as
    // in Lepton hosts; session actors keep the external policy. gauge-app itself never
    // asks Higgs for System (uf-no-actor-elevation).
    let factory: Arc<dyn HiggsValenceFactory> = Arc::new(SplitFactory {
        session: factory,
        system: Arc::new(HiggsFactory(RouterValenceFactory::new(
            Arc::clone(&router),
            RouterValenceFactoryConfig::new(default_key.clone()),
        ))),
    });
    let higgs = Arc::new(
        HiggsConfig::builder()
            .valence_factory_arc(factory)
            .build()
            .expect("e2e HiggsConfig"),
    );

    let state = Arc::new(E2eState {
        router,
        higgs,
        auth_valence_factory,
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
            display_name: String::new(),
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
            on_behalf_of: None,
        },
        &requestor_ctx,
    )
    .await?;
    let pending_group = service::create_permission_request(
        PermissionRequestCreateInput {
            target_kind: PermissionRequestTargetKind::Group,
            target_id: group_id.clone(),
            reason: "need group membership for e2e".into(),
            on_behalf_of: None,
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
        service_user_handle: REQUESTOR_SERVICE_USER.into(),
        foreign_service_user_handle: ADMIN_SERVICE_USER.into(),
        service_user_id: String::new(),
        paused_service_user_id: String::new(),
        foreign_service_user_id: String::new(),
        resource: Default::default(),
        release_group_id: String::new(),
        release_permission_id: String::new(),
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

/// The factory the axum-login backend loads signed-in users through.
pub fn e2e_valence_factory() -> Arc<dyn HiggsValenceFactory> {
    Arc::clone(&state().auth_valence_factory)
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
