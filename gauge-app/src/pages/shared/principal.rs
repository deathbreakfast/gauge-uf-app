//! Display helpers for principal rows (users, groups, service users).

use gauge::types::{PrincipalKind, PrincipalRefDto};

/// Lowercase kind word shown in a principal row.
#[must_use]
pub const fn kind_label(kind: &PrincipalKind) -> &'static str {
    match kind {
        PrincipalKind::Group => "group",
        PrincipalKind::ServiceUser => "service user",
        _ => "user",
    }
}

/// Icon shown beside a principal row.
#[must_use]
pub const fn kind_icon(kind: &PrincipalKind) -> icondata::Icon {
    match kind {
        PrincipalKind::Group => icondata::AiTeamOutlined,
        PrincipalKind::ServiceUser => icondata::AiApiOutlined,
        _ => icondata::AiUserOutlined,
    }
}

/// Second line of a principal row: kind and id, plus the owner for a service user.
#[must_use]
pub fn detail_line(principal: &PrincipalRefDto) -> String {
    let base = format!("{} ({})", kind_label(&principal.kind), principal.id);
    match (&principal.kind, principal.owner_label.as_deref()) {
        (PrincipalKind::ServiceUser, Some(owner)) => format!("{base}, owner {owner}"),
        _ => base,
    }
}
