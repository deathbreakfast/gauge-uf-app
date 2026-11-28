//! Words and small views the permission and group pages share: why someone holds access,
//! times through Orbital `Timestamp`, and a person's kind.

use gauge::types::{
    AccessPath, PermissionRequestRowDto, PermissionRequestStatusDto, PermissionRequestTargetKind,
    PrincipalKind, PrincipalRefDto,
};
use leptos::prelude::*;
use uf_product::components::Body1Strong;
use uf_product::primitives::{Link, Timestamp, TimestampStyle, TimestampValue};

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

/// A label without Gauge's short-id suffix, for sentences: "Ravi Menon (h3jx6p)" reads
/// "Ravi Menon". Lists keep the suffix so two people with one name stay apart.
#[must_use]
pub fn plain_name(label: &str) -> String {
    let trimmed = label.trim();
    if let Some(open) = trimmed.rfind(" (") {
        let inner = &trimmed[open + 2..];
        if let Some(id) = inner.strip_suffix(')') {
            if !id.is_empty()
                && id.len() <= 6
                && id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                return trimmed[..open].to_string();
            }
        }
    }
    trimmed.to_string()
}

/// A first name for sentences ("Ravi loses"): the plain name's first word.
#[must_use]
pub fn first_name(label: &str) -> String {
    let name = plain_name(label);
    name.split_whitespace().next().unwrap_or(&name).to_string()
}

/// Who a request is for, after the requester's name: "for @release-bot, owned by Sol
/// Bergström", or the handle alone when the owner isn't known.
#[must_use]
pub fn for_line(service_user: &PrincipalRefDto) -> String {
    match service_user.owner_label.as_deref().map(plain_name) {
        Some(owner) if !owner.is_empty() => {
            format!("for @{}, owned by {owner}", service_user.label)
        }
        _ => format!("for @{}", service_user.label),
    }
}

/// What a request asks for, as a phrase after the requester ("asks to join Release
/// Captains", "asks for Approve releases").
#[must_use]
pub fn asks_for(row: &PermissionRequestRowDto) -> String {
    match row.target_kind {
        PermissionRequestTargetKind::Group => format!("asks to join {}", row.target_label),
        PermissionRequestTargetKind::Permission => format!("asks for {}", row.target_label),
    }
}

/// A request's status in words: "Waiting for Sol Bergström" while pending (with who reviews
/// it, when known), then Approved, Denied or Withdrawn.
#[must_use]
pub fn status_words(status: &PermissionRequestStatusDto, reviewers: Option<&str>) -> String {
    match (status, reviewers) {
        (PermissionRequestStatusDto::Pending, Some(who)) if !who.is_empty() => {
            format!("Waiting for {who}")
        }
        (PermissionRequestStatusDto::Pending, _) => "Waiting for review".to_string(),
        (PermissionRequestStatusDto::Approved, _) => "Approved".to_string(),
        (PermissionRequestStatusDto::Denied, _) => "Denied".to_string(),
        (PermissionRequestStatusDto::Withdrawn, _) => "Withdrawn".to_string(),
    }
}

/// The start of a reason, cut at a word near `max` characters with "…".
#[must_use]
pub fn reason_start(reason: &str, max: usize) -> String {
    let reason = reason.trim();
    if reason.chars().count() <= max {
        return reason.to_string();
    }
    let cut: String = reason.chars().take(max).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head);
    format!("{}…", cut.trim_end_matches([',', '.', ';', ':']))
}

/// The service user id in a table row key made as `format!("{:?}:{}", kind, id)`.
#[must_use]
pub fn service_user_of_row(row_id: &str) -> Option<String> {
    row_id.strip_prefix("ServiceUser:").map(str::to_string)
}

/// A member's or holder's name in a list. A service user's name links to its page.
pub fn principal_name_view(label: String, service_user_id: Option<String>) -> AnyView {
    match service_user_id {
        Some(id) => view! {
            <Link href=format!("/permission/service-users/{id}")>
                <Body1Strong>{label}</Body1Strong>
            </Link>
        }
        .into_any(),
        None => view! { <Body1Strong>{label}</Body1Strong> }.into_any(),
    }
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
    fn service_user_rows_are_found_by_key_happy() {
        assert_eq!(
            service_user_of_row("ServiceUser:su1"),
            Some("su1".to_string())
        );
        assert_eq!(service_user_of_row("User:ravi"), None);
    }

    #[test]
    fn for_line_names_the_owner_happy() {
        let mut bot = PrincipalRefDto {
            kind: PrincipalKind::ServiceUser,
            id: "su1".into(),
            label: "release-bot".into(),
            owner_label: Some("Sol Bergström (h3jx6p)".into()),
        };
        assert_eq!(for_line(&bot), "for @release-bot, owned by Sol Bergström");
        bot.owner_label = None;
        assert_eq!(for_line(&bot), "for @release-bot");
    }

    #[test]
    fn status_and_reason_in_words_happy() {
        assert_eq!(
            status_words(&PermissionRequestStatusDto::Pending, Some("Sol")),
            "Waiting for Sol"
        );
        assert_eq!(
            status_words(&PermissionRequestStatusDto::Pending, None),
            "Waiting for review"
        );
        assert_eq!(
            status_words(&PermissionRequestStatusDto::Denied, Some("Sol")),
            "Denied"
        );
        assert_eq!(reason_start("short", 10), "short");
        assert_eq!(
            reason_start("I cover Sol's on-call shifts next week", 20),
            "I cover Sol's…"
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
        assert_eq!(plain_name("Ravi Menon (h3jx6p)"), "Ravi Menon");
        assert_eq!(plain_name("Release (2026 train)"), "Release (2026 train)");
        assert_eq!(first_name(""), "");
    }
}
