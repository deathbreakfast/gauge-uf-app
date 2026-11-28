//! Words and small views the permission and group pages share: why someone holds access,
//! times through Orbital `Timestamp`, and a person's kind.

use gauge::types::{AccessPath, PrincipalKind};
use leptos::prelude::*;
use uf_product::primitives::{Timestamp, TimestampStyle, TimestampValue};

/// Why a principal holds a permission, in words ("Through Release Captains › On-call SRE").
#[must_use]
pub fn path_text(path: &AccessPath) -> String {
    match path {
        AccessPath::Direct => "Allow list".to_string(),
        AccessPath::ThroughGroup(groups) => format!(
            "Through {}",
            groups
                .iter()
                .map(|g| g.name.as_str())
                .collect::<Vec<_>>()
                .join(" › ")
        ),
        AccessPath::SuperUser => "Super User".to_string(),
    }
}

/// Every path, one per line's worth, joined with "; ".
#[must_use]
pub fn paths_text(paths: &[AccessPath]) -> String {
    paths.iter().map(path_text).collect::<Vec<_>>().join("; ")
}

/// A person, service user or group, as the second line of a row.
#[must_use]
pub fn kind_word(kind: &PrincipalKind, owner_label: Option<&str>) -> String {
    match (kind, owner_label) {
        (PrincipalKind::Group, _) => "Group".to_string(),
        (PrincipalKind::ServiceUser, Some(owner)) => format!("Service user · {owner}'s"),
        (PrincipalKind::ServiceUser, None) => "Service user".to_string(),
        _ => "Person".to_string(),
    }
}

/// A first name for sentences ("Ravi loses"): the label's first word.
#[must_use]
pub fn first_name(label: &str) -> String {
    label.split_whitespace().next().unwrap_or(label).to_string()
}

/// An RFC 3339 instant as an Orbital date.
pub fn date_view(rfc3339: String) -> impl IntoView {
    view! { <Timestamp value=TimestampValue::Rfc3339(rfc3339) style=TimestampStyle::Date /> }
}

/// An RFC 3339 instant as an Orbital date and time.
pub fn when_view(rfc3339: String) -> impl IntoView {
    view! { <Timestamp value=TimestampValue::Rfc3339(rfc3339) /> }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gauge::types::GroupRef;

    fn group(name: &str) -> GroupRef {
        GroupRef {
            id: name.to_lowercase(),
            name: name.into(),
        }
    }

    #[test]
    fn nested_group_path_names_each_group_happy() {
        let path = AccessPath::ThroughGroup(vec![group("Release Captains"), group("On-call SRE")]);
        assert_eq!(path_text(&path), "Through Release Captains › On-call SRE");
        assert_eq!(
            paths_text(&[AccessPath::Direct, AccessPath::SuperUser]),
            "Allow list; Super User"
        );
    }

    #[test]
    fn kinds_and_first_names_happy() {
        assert_eq!(kind_word(&PrincipalKind::User, None), "Person");
        assert_eq!(
            kind_word(&PrincipalKind::ServiceUser, Some("Sol Bergström")),
            "Service user · Sol Bergström's"
        );
        assert_eq!(first_name("Ravi Menon"), "Ravi");
        assert_eq!(first_name(""), "");
    }
}
