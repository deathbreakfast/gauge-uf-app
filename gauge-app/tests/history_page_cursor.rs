#![cfg(feature = "ssr")]
#![allow(missing_docs)]
#![allow(clippy::expect_used, clippy::unwrap_used)]
//! Cursor and offset paging through `load_gauge_history_page`, the body of the
//! Show History server fns after the session check.
//!
//! Runs on an in-memory Valence with user actors, so every page goes through
//! Gauge's can-edit check and Record History's parent Read, as a real request
//! does.

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use gauge::service;
use gauge::types::PermissionGroupCreateInput;
use gauge_app::server::load_gauge_history_page;
use orbital_paging::Page;
use record_history::HISTORY_ACCESS_DENIED;
use record_history_leptos::HistoryRowView;
use valence::{
    register_backend_logical_names, router_key, Actor, CompiledQuery, DatabaseBackend,
    DatabaseRouter, InMemoryBackend, Model, RecordId, RegisterBackendLogicalNamesOptions, Valence,
    MEM_ENGINE_ID, SQLITE_ENGINE_ID,
};

const OWNER: &str = "history-owner";
const OUTSIDER: &str = "history-outsider";
/// Description edits after the create; each one writes a history entry.
const EDITS: usize = 60;
const LIMIT: u32 = 25;

/// [`InMemoryBackend`] that treats unique-index DDL as success, so lepton's
/// `AccountEmail` seed works.
#[derive(Debug)]
struct TolerantMemBackend(InMemoryBackend);

#[async_trait]
impl DatabaseBackend for TolerantMemBackend {
    fn engine_id(&self) -> &'static str {
        self.0.engine_id()
    }
    fn capabilities(&self) -> valence::BackendCapabilities {
        self.0.capabilities()
    }
    async fn use_namespace(&self, ns: &str, db_name: &str) -> valence::Result<()> {
        self.0.use_namespace(ns, db_name).await
    }
    async fn execute_compiled_query(
        &self,
        compiled: &CompiledQuery,
    ) -> valence::Result<Vec<serde_json::Value>> {
        self.0.execute_compiled_query(compiled).await
    }
    async fn ensure_schemaless_table(&self, table: &str) -> valence::Result<()> {
        self.0.ensure_schemaless_table(table).await
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
    async fn define_unique_index(&self, table: &str, field: &str) -> valence::Result<()> {
        match self.0.define_unique_index(table, field).await {
            Err(valence::Error::Internal(msg))
                if msg.contains("define_unique_index")
                    || msg.contains("unique indexes not supported") =>
            {
                Ok(())
            }
            other => other,
        }
    }
}

fn system_valence() -> Valence {
    valence::deletion::register_noop_deletion_dispatcher_for_tests();
    valence::clear_for_test();
    let backend: Arc<dyn DatabaseBackend> = Arc::new(TolerantMemBackend(InMemoryBackend::new()));
    let mut router = DatabaseRouter::new();
    register_backend_logical_names(
        &mut router,
        Arc::clone(&backend),
        &["default"],
        RegisterBackendLogicalNamesOptions {
            // Lepton identity schemas route via SQLITE_ENGINE_ID.
            register_alias_engine_id: Some(SQLITE_ENGINE_ID),
        },
    );
    gauge::embedded_surreal::register_storage(&mut router, backend);
    Valence::builder()
        .database_router(Arc::new(router))
        .default_backend_key(router_key("default", MEM_ENGINE_ID))
        .with_actor(Actor::System {
            operation: "gauge_app_history_cursor_test_setup".to_string(),
        })
        .build()
        .expect("valence build")
}

fn as_user(base: &Valence, user_id: &str) -> Valence {
    base.with_actor(Actor::User {
        user_id: user_id.to_string(),
    })
}

/// A lepton user with an account and a verified primary email.
async fn seed_user(id: &str, valence: &Valence) {
    use lepton::generated::{
        Account, AccountEmail, AccountPlan, AccountStatus, User, UserStatus, UserUserType,
    };
    let now = Utc::now();
    let user = User::new(
        Some(UserUserType::Person),
        Some("test-password-hash".to_string()),
        Some(UserStatus::Active),
        None,
        None,
        Some(now),
        None,
        None,
        now,
        now,
    )
    .expect("build user");
    let user = User::upsert(id, user, valence, valence::use_!(r"**Test:** Fixture **User** save for `history_page_cursor` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."))
        .await
        .expect("upsert user");
    let account = Account::new(
        "Test Account".to_string(),
        RecordId::new("user", id),
        Some(AccountPlan::Free),
        Some(AccountStatus::Active),
        None,
        None,
        now,
        now,
    )
    .expect("build account");
    let account = Account::upsert(&format!("acct_{id}"), account, valence, valence::use_!(r"**Test:** Fixture **Account** save for `history_page_cursor` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."))
        .await
        .expect("upsert account");
    let email = AccountEmail::new(
        account.id().cloned().expect("account id"),
        format!("{id}@example.com"),
        Some(now),
        now,
        now,
    )
    .expect("build email");
    let email = AccountEmail::upsert(&format!("email_{id}"), email, valence, valence::use_!(r"**Test:** Fixture **Account Email** save for `history_page_cursor` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."))
        .await
        .expect("upsert email");
    user.get_mutable(valence, valence::use_!(r"**Test:** Fixture **User** primary email update in `history_page_cursor` so the suite can arrange and assert persistence. CI and developers running the suite only."))
        .set_primary_email(email.id().cloned().expect("email id"))
        .expect("set primary email")
        .set_updated_at(now)
        .expect("set updated_at")
        .commit()
        .await
        .expect("commit primary email");
}

/// Owner and outsider users, and a group the owner created and edited
/// [`EDITS`] times. Returns the system handle and the group's history source.
async fn seed_group_with_history() -> (Valence, RecordId) {
    let system = system_valence();
    seed_user(OWNER, &system).await;
    seed_user(OUTSIDER, &system).await;
    let owner = as_user(&system, OWNER);
    let group = service::create_group(
        PermissionGroupCreateInput {
            name: "Cursor History Group".to_string(),
            description: "edit 0".to_string(),
        },
        &owner,
    )
    .await
    .expect("create group");
    let group_id = group
        .id()
        .and_then(|t| valence::extract_id_from_record(t).ok())
        .expect("group id");
    for i in 1..=EDITS {
        service::update_group(
            &group_id,
            "Cursor History Group".to_string(),
            format!("edit {i}"),
            &owner,
        )
        .await
        .expect("update group");
    }
    (system, RecordId::new("permission_group", &group_id))
}

fn keys(page: &Page<HistoryRowView>) -> Vec<String> {
    page.items
        .iter()
        .map(|row| format!("{}:{}", row.kind, row.row_id))
        .collect()
}

/// Follow `next_cursor` (sending `next_request_offset` too, as the timeline
/// does) until `has_more` is false. Returns every entry and the page count.
async fn read_by_cursor(v: &Valence, source: &RecordId) -> (Vec<String>, usize) {
    let mut got = Vec::new();
    let mut offset = 0;
    let mut cursor = None;
    let mut pages = 0;
    loop {
        let page = load_gauge_history_page(source, v, offset, LIMIT, None, cursor)
            .await
            .expect("history page");
        pages += 1;
        assert!(page.items.len() <= LIMIT as usize);
        got.extend(keys(&page));
        if !page.has_more {
            assert_eq!(page.next_cursor, None, "last page has no cursor");
            assert_eq!(page.total_count, Some(got.len() as u64));
            return (got, pages);
        }
        cursor = Some(
            page.next_cursor
                .clone()
                .expect("next_cursor while has_more"),
        );
        offset = page.next_request_offset.expect("next_request_offset");
        assert!(pages < 100, "paging never ended");
    }
}

/// Read every entry by `offset` alone, with no cursor.
async fn read_by_offset(v: &Valence, source: &RecordId) -> Vec<String> {
    let mut got = Vec::new();
    let mut offset = 0;
    loop {
        let page = load_gauge_history_page(source, v, offset, LIMIT, None, None)
            .await
            .expect("offset page");
        got.extend(keys(&page));
        if !page.has_more {
            return got;
        }
        offset = page.next_request_offset.expect("next_request_offset");
    }
}

/// The group owner pages the whole history by cursor: every entry once, in the
/// same newest-first order the offset path returns, over several pages.
#[tokio::test]
async fn owner_pages_history_by_cursor_happy_path() {
    let (system, source) = seed_group_with_history().await;
    let owner = as_user(&system, OWNER);

    let (by_cursor, pages) = read_by_cursor(&owner, &source).await;
    assert!(
        by_cursor.len() > EDITS,
        "expected the create plus {EDITS} edits, got {}",
        by_cursor.len()
    );
    assert_eq!(pages, by_cursor.len().div_ceil(LIMIT as usize));
    let unique: HashSet<&String> = by_cursor.iter().collect();
    assert_eq!(
        unique.len(),
        by_cursor.len(),
        "an entry repeated across pages"
    );
    assert!(by_cursor
        .iter()
        .all(|key| key.starts_with("permission_history:")));

    assert_eq!(by_cursor, read_by_offset(&owner, &source).await);

    // The newest entry leads the first page.
    let first = load_gauge_history_page(&source, &owner, 0, LIMIT, None, None)
        .await
        .expect("first page");
    assert_eq!(first.items[0].new_value, format!("edit {EDITS}"));
}

/// Someone who can't edit the group is denied on every page, and the owner's
/// cursor doesn't change that.
#[tokio::test]
async fn outsider_denied_with_or_without_cursor_sad() {
    let (system, source) = seed_group_with_history().await;
    let owner = as_user(&system, OWNER);
    let outsider = as_user(&system, OUTSIDER);

    let err = load_gauge_history_page(&source, &outsider, 0, LIMIT, None, None)
        .await
        .expect_err("outsider must not read the group's history");
    assert!(err.to_string().contains(HISTORY_ACCESS_DENIED), "{err}");

    let first = load_gauge_history_page(&source, &owner, 0, LIMIT, None, None)
        .await
        .expect("owner first page");
    assert!(first.next_cursor.is_some());
    let err = load_gauge_history_page(&source, &outsider, LIMIT, LIMIT, None, first.next_cursor)
        .await
        .expect_err("a cursor grants nothing");
    assert!(err.to_string().contains(HISTORY_ACCESS_DENIED), "{err}");
}

/// Sources Show History doesn't serve are denied before any read: another
/// table, and a group id that doesn't exist.
#[tokio::test]
async fn unsupported_or_missing_source_denied_sad() {
    let (system, _source) = seed_group_with_history().await;
    let owner = as_user(&system, OWNER);

    for source in [
        RecordId::new("user", OWNER),
        RecordId::new("permission_group", "no-such-group"),
    ] {
        let err = load_gauge_history_page(&source, &owner, 0, LIMIT, None, None)
            .await
            .expect_err("source must be denied");
        assert!(
            err.to_string().contains(HISTORY_ACCESS_DENIED),
            "{source:?}: {err}"
        );
    }
}

/// A cursor from another page size, another record, or a garbage token falls
/// back to `offset`.
#[tokio::test]
async fn stale_or_garbage_cursor_falls_back_to_offset_sad() {
    let (system, source) = seed_group_with_history().await;
    let owner = as_user(&system, OWNER);
    let all = read_by_offset(&owner, &source).await;

    let first = load_gauge_history_page(&source, &owner, 0, LIMIT, None, None)
        .await
        .expect("first page");
    let cursor = first.next_cursor.clone().expect("cursor");

    // Same query: the cursor continues after the first page, whatever the offset.
    let next = load_gauge_history_page(&source, &owner, 0, LIMIT, None, Some(cursor.clone()))
        .await
        .expect("cursor page");
    assert_eq!(keys(&next), all[25..50]);

    // Another page size: offset 30, ten entries.
    let page = load_gauge_history_page(&source, &owner, 30, 10, None, Some(cursor.clone()))
        .await
        .expect("resized page");
    assert_eq!(keys(&page), all[30..40]);

    for token in ["", "not-a-cursor", "e30"] {
        let page = load_gauge_history_page(&source, &owner, 40, LIMIT, None, Some(token.into()))
            .await
            .expect("garbage cursor page");
        assert_eq!(keys(&page), all[40..all.len().min(65)], "token {token:?}");
    }

    // A second group's history ignores the first group's cursor.
    let owner_ctx = as_user(&system, OWNER);
    let other = service::create_group(
        PermissionGroupCreateInput {
            name: "Other History Group".to_string(),
            description: "other".to_string(),
        },
        &owner_ctx,
    )
    .await
    .expect("create other group");
    let other_id = other
        .id()
        .and_then(|t| valence::extract_id_from_record(t).ok())
        .expect("other group id");
    let other_source = RecordId::new("permission_group", &other_id);
    let other_all = read_by_offset(&owner, &other_source).await;
    let page = load_gauge_history_page(&other_source, &owner, 0, LIMIT, None, Some(cursor))
        .await
        .expect("other record page");
    assert_eq!(keys(&page), other_all[..other_all.len().min(25)]);
}
