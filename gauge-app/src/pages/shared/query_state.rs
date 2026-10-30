//! List state kept in the URL query, so a search or filter can be linked and survives reload,
//! and the `?name=` lookup that opens a group or permission straight from its name.

use std::future::Future;

use leptos::prelude::*;
use leptos::task::spawn_local_scoped;
use leptos_router::hooks::{use_location, use_navigate, use_query_map};
use leptos_router::params::ParamsMap;
use leptos_router::NavigateOptions;

/// Query keys a name lookup forwards to the detail page it opens.
const FORWARDED_QUERY_KEYS: [&str; 2] = ["request", "for"];

fn replace_options() -> NavigateOptions {
    NavigateOptions {
        replace: true,
        scroll: false,
        ..NavigateOptions::default()
    }
}

/// A string signal bound to the query key `key`: it starts from the URL, and each change
/// replaces the key in the URL (removing it when empty) without adding a history entry.
///
/// Nothing is written while the URL still carries `?name=`; the name lookup owns the URL
/// until it resolves.
pub fn use_query_signal(key: &'static str, initial: Option<String>) -> RwSignal<String> {
    let query = use_query_map();
    let location = use_location();
    let navigate = use_navigate();
    let value = RwSignal::new(
        initial
            .or_else(|| query.read_untracked().get(key))
            .unwrap_or_default(),
    );
    Effect::new(move |_| {
        let wanted = value.get();
        let current = query.get_untracked();
        if current.get_str("name").is_some() || current.get_str(key).unwrap_or("") == wanted {
            return;
        }
        let mut next = current;
        if wanted.is_empty() {
            next.remove(key);
        } else {
            next.replace(key, wanted);
        }
        let path = location.pathname.get_untracked();
        navigate(
            &format!("{path}{}", next.to_query_string()),
            replace_options(),
        );
    });
    value
}

/// Id of the one row whose name equals `name`, ignoring case and surrounding spaces.
/// `None` when no row matches or several do.
#[must_use]
pub fn exact_name_match<'a>(
    rows: impl IntoIterator<Item = (&'a str, &'a str)>,
    name: &str,
) -> Option<String> {
    let wanted = name.trim().to_lowercase();
    let mut matches = rows
        .into_iter()
        .filter(|(_, row_name)| row_name.trim().to_lowercase() == wanted);
    let (id, _) = matches.next()?;
    matches.next().is_none().then(|| id.to_string())
}

/// `?request=…&for=…` from `query`, ready to append to a detail path; empty when neither is set.
#[must_use]
pub fn forwarded_query(query: &ParamsMap) -> String {
    let mut forwarded = ParamsMap::new();
    for key in FORWARDED_QUERY_KEYS {
        if let Some(value) = query.get(key) {
            forwarded.insert(key, value);
        }
    }
    forwarded.to_query_string()
}

/// Index-page search kept in `?q=`, plus the `?name=<exact name>` lookup.
///
/// With `?name=`, `lookup(name)` returns `(id, name)` rows. One exact match (ignoring case)
/// replaces the URL with `{index_path}/{id}`, keeping `request` and `for`. Otherwise the page
/// stays, the URL becomes `?q=<name>`, and the returned `missing` signal holds the name.
///
/// Returns `(search, missing)`.
pub fn use_index_search<F, Fut>(
    index_path: &'static str,
    lookup: F,
) -> (RwSignal<String>, RwSignal<Option<String>>)
where
    F: Fn(String) -> Fut + 'static,
    Fut: Future<Output = Result<Vec<(String, String)>, ServerFnError>> + 'static,
{
    let query = use_query_map();
    let navigate = use_navigate();
    let name = query
        .read_untracked()
        .get("name")
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty());
    let search = use_query_signal("q", name.clone());
    let missing = RwSignal::new(None::<String>);

    if let Some(name) = name {
        let forwarded = forwarded_query(&query.read_untracked());
        // Reads nothing reactive, so it runs once, in the browser.
        Effect::new(move |_| {
            let name = name.clone();
            let forwarded = forwarded.clone();
            let navigate = navigate.clone();
            let rows = lookup(name.clone());
            spawn_local_scoped(async move {
                let found = rows.await.ok().and_then(|rows| {
                    exact_name_match(rows.iter().map(|(id, n)| (id.as_str(), n.as_str())), &name)
                });
                match found {
                    Some(id) => {
                        navigate(&format!("{index_path}/{id}{forwarded}"), replace_options())
                    }
                    None => {
                        missing.set(Some(name.clone()));
                        let mut q = ParamsMap::new();
                        q.insert("q", name);
                        navigate(
                            &format!("{index_path}{}", q.to_query_string()),
                            replace_options(),
                        );
                    }
                }
            });
        });
    }

    (search, missing)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_name_match_ignores_case_and_spaces() {
        let rows = [
            ("g1", "Service user creators"),
            ("g2", "Service user creators extra"),
        ];
        assert_eq!(
            exact_name_match(rows, "  service USER creators "),
            Some("g1".to_string())
        );
    }

    #[test]
    fn exact_name_match_rejects_partial_and_duplicate_names() {
        assert_eq!(exact_name_match([("g1", "Deployers")], "Deploy"), None);
        assert_eq!(
            exact_name_match([("g1", "Ops"), ("g2", "ops")], "Ops"),
            None
        );
        assert_eq!(exact_name_match(std::iter::empty(), "Ops"), None);
    }

    #[test]
    fn forwarded_query_keeps_only_request_and_for() {
        let mut query = ParamsMap::new();
        assert_eq!(forwarded_query(&query), "");
        query.insert("name", "Deployers".to_string());
        query.insert("request", "1".to_string());
        query.insert("for", "spectra-agent".to_string());
        // The router escapes `-` too; browsers decode it back to `spectra-agent`.
        assert_eq!(forwarded_query(&query), "?request=1&for=spectra%2Dagent");
    }
}
