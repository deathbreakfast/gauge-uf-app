//! The Activity tab for a record's owners, who can't read Gauge's whole change log.
//!
//! Gauge admins get [`gauge::service::list_activity`] filtered to the record. Owners read the
//! record's history rows ([`gauge::service::list_history`]) and this module words each one the
//! way the change log does ("Added Ravi Menon to the group"), naming people and groups.

use std::collections::HashMap;

use gauge::types::{ActivityKind, ActivityRow, HistoryEntryDto, RecordKind, RecordRef};
use leptos::prelude::ServerFnError;
use valence::{Model, Valence};

use super::map_service_err;

/// History rows for `record`, in words, newest first.
pub(super) async fn owner_record_activity(
    record: RecordRef,
    v: &Valence,
) -> Result<Vec<ActivityRow>, ServerFnError> {
    let table = match record.kind {
        RecordKind::Permission => "permission",
        RecordKind::Group => "permission_group",
        // Gauge keeps no history rows for domains or requests.
        RecordKind::Domain | RecordKind::Request => return Ok(Vec::new()),
    };
    let entries = gauge::service::list_history(v, Some(table.to_string()), Some(record.id.clone()))
        .await
        .map_err(|e| map_service_err("load activity", e))?;
    let mut labels = Labels::default();
    let mut rows = Vec::with_capacity(entries.len());
    for entry in &entries {
        rows.extend(entry_rows(entry, &record, &mut labels, v).await);
    }
    Ok(rows)
}

async fn entry_rows(
    entry: &HistoryEntryDto,
    record: &RecordRef,
    labels: &mut Labels,
    v: &Valence,
) -> Vec<ActivityRow> {
    let Ok(at) = chrono::DateTime::parse_from_rfc3339(&entry.changed_at) else {
        return Vec::new();
    };
    let at = at.with_timezone(&chrono::Utc);
    let actor_label = match &entry.actor_user_id {
        Some(id) => labels.user(id, v).await,
        None => "Gauge".to_string(),
    };
    let mut out = Vec::new();
    for diff in &entry.diffs {
        let old = value_text(&diff.old_value);
        let new = value_text(&diff.new_value);
        let (kind, summary) = describe(record, &diff.field, &old, &new, labels, v).await;
        out.push(ActivityRow {
            at,
            actor_label: actor_label.clone(),
            summary,
            record: record.clone(),
            kind,
        });
    }
    out
}

fn value_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// One change in words, matching Gauge's change log.
async fn describe(
    record: &RecordRef,
    field: &str,
    old: &str,
    new: &str,
    labels: &mut Labels,
    v: &Valence,
) -> (ActivityKind, String) {
    let base_kind = match record.kind {
        RecordKind::Permission => ActivityKind::Permission,
        _ => ActivityKind::Group,
    };
    let noun = if record.kind == RecordKind::Permission {
        "permission"
    } else {
        "group"
    };
    let name = &record.label;
    let added = old.is_empty() && !new.is_empty();
    let value = if added { new } else { old };
    match field {
        "created" => (base_kind, format!("Created the {noun} {name}")),
        "deleted" => (base_kind, format!("Deleted the {noun} {name}")),
        "name" | "display_name" => (base_kind, format!("Renamed the {noun} {old} to {new}")),
        "description" => (base_kind, format!("Edited the description of {name}")),
        "owners_group" => (base_kind, format!("Changed who owns {name}")),
        "domain" => (base_kind, format!("Moved {name} to another domain")),
        "granted_users" | "granted_groups" | "granted_service_users" => {
            let who = labels.principal(field, value, v).await;
            let summary = if added {
                format!("Gave {who} {name}")
            } else {
                format!("Removed {who} from {name}")
            };
            (ActivityKind::Grant, summary)
        }
        "member_users" | "member_groups" | "member_service_users" => {
            let who = labels.principal(field, value, v).await;
            let summary = if added {
                format!("Added {who} to the group")
            } else {
                format!("Removed {who} from the group")
            };
            (ActivityKind::Membership, summary)
        }
        "owner_users" | "owner_groups" => {
            let who = labels.principal(field, value, v).await;
            let summary = if added {
                format!("Made {who} an owner of the group")
            } else {
                format!("Removed {who} as an owner of the group")
            };
            (ActivityKind::Membership, summary)
        }
        other => (
            base_kind,
            format!("Changed {} on {name}", other.replace('_', " ")),
        ),
    }
}

/// Names already looked up in this request.
#[derive(Default)]
struct Labels {
    users: HashMap<String, String>,
    groups: HashMap<String, String>,
}

impl Labels {
    async fn user(&mut self, user_id: &str, v: &Valence) -> String {
        let bare = user_id.rsplit(':').next().unwrap_or(user_id).to_string();
        if let Some(label) = self.users.get(&bare) {
            return label.clone();
        }
        let label = if bare == "system" {
            "Gauge".to_string()
        } else {
            match lepton::generated::User::get(&bare, v, valence::use_!(r"When a **record's owner** opens its **Activity** tab in **Gauge**, we **load each person named in its history** so the tab shows their name instead of an id. Only the record's owners and Gauge admins see the tab.")).await {
                Ok(Some(user)) => gauge::search_sources::user_principal_label(&user, &bare, v).await,
                Ok(None) | Err(_) => "someone".to_string(),
            }
        };
        self.users.insert(bare, label.clone());
        label
    }

    async fn group(&mut self, group_id: &str, v: &Valence) -> String {
        if let Some(label) = self.groups.get(group_id) {
            return label.clone();
        }
        let label = match gauge::service::get_group_detail(group_id, v).await {
            Ok(Some(group)) => group.name,
            Ok(None) | Err(_) => "a group".to_string(),
        };
        self.groups.insert(group_id.to_string(), label.clone());
        label
    }

    async fn principal(&mut self, field: &str, id: &str, v: &Valence) -> String {
        if field.ends_with("_groups") {
            self.group(id, v).await
        } else if field.ends_with("_service_users") {
            "a service user".to_string()
        } else {
            self.user(id, v).await
        }
    }
}
