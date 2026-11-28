//! Who reviews each request, loaded once for a list of requests.

use std::collections::HashMap;

use gauge::types::PermissionRequestRowDto;
use leptos::prelude::*;

use crate::server::surfaces::{get_request_reviewers, review_target};

/// Who reviews each request's target, keyed by [`review_target`]. Loads in the browser after
/// the rows, so a failed read leaves the status as "Waiting for review".
pub fn reviewers_for(
    rows: Signal<Vec<PermissionRequestRowDto>>,
) -> LocalResource<HashMap<String, String>> {
    LocalResource::new(move || {
        // The key runs during server rendering too, and `rows` may read another browser-only
        // resource; read nothing there.
        let mut targets: Vec<String> = if cfg!(feature = "ssr") {
            Vec::new()
        } else {
            rows.with(|rows| {
                rows.iter()
                    .map(|r| review_target(&r.target_kind, &r.target_id))
                    .collect()
            })
        };
        targets.sort();
        targets.dedup();
        async move {
            if targets.is_empty() {
                return HashMap::new();
            }
            get_request_reviewers(targets.clone())
                .await
                .map(|lines| targets.into_iter().zip(lines).collect())
                .unwrap_or_default()
        }
    })
}

/// The reviewers line for `row`, once loaded.
pub fn reviewers_of(
    reviewers: LocalResource<HashMap<String, String>>,
    row: &PermissionRequestRowDto,
) -> Option<String> {
    let key = review_target(&row.target_kind, &row.target_id);
    reviewers.get().and_then(|map| map.get(&key).cloned())
}
