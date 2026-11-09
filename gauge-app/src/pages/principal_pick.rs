//! Glue between Lepton's [`PersonPicker`](lepton_shell::people::PersonPicker) and Gauge
//! principals: what a pick grants, which rows show disabled, and the permission groups
//! offered beside people.

use gauge::search_sources::PermissionSearchSourceId;
use gauge::types::{PrincipalKind as GaugeKind, PrincipalRefDto};
use lepton_shell::people::{PickedPrincipal, PickerGroup, PickerOption, PrincipalKind};
use leptos::prelude::*;
use uf_product::primitives::{Badge, BadgeAppearance, BadgeColor, BadgeSize};
use uf_search_core::SearchSourceItem;

use crate::server::search_principals;

/// Header of the caller-supplied group that lists permission groups.
pub const GROUPS_LABEL: &str = "Permission groups";

/// Most permission groups a picker offers.
const MAX_GROUP_OPTIONS: u32 = 50;

/// The Gauge principal a pick names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PickTarget {
    /// A Lepton user id.
    User(String),
    /// A Lepton service user id.
    ServiceUser(String),
    /// A permission group id.
    Group(String),
}

/// What `pick` grants, or `None` for kinds Gauge can't hold (teams, other extra groups).
#[must_use]
pub fn pick_target(pick: &PickedPrincipal) -> Option<PickTarget> {
    let id = pick.id.clone();
    match &pick.kind {
        PrincipalKind::User => Some(PickTarget::User(id)),
        PrincipalKind::ServiceUser => Some(PickTarget::ServiceUser(id)),
        PrincipalKind::Extra(label) if label == GROUPS_LABEL => Some(PickTarget::Group(id)),
        PrincipalKind::Team | PrincipalKind::Extra(_) => None,
    }
}

/// `principal` as a picker row key, so existing members and owners show disabled.
#[must_use]
pub fn to_picked(principal: &PrincipalRefDto) -> PickedPrincipal {
    let kind = match principal.kind {
        GaugeKind::Group => PrincipalKind::Extra(GROUPS_LABEL.to_string()),
        GaugeKind::ServiceUser => PrincipalKind::ServiceUser,
        _ => PrincipalKind::User,
    };
    PickedPrincipal {
        kind,
        id: principal.id.clone(),
        label: principal.label.clone(),
        secondary: None,
        image_url: None,
    }
}

/// Permission group search results as one picker group, leaving out `skip` (the group
/// being edited). No results gives no group at all.
#[must_use]
pub fn group_options(items: Vec<SearchSourceItem>, skip: Option<&str>) -> Vec<PickerGroup> {
    let options: Vec<PickerOption> = items
        .into_iter()
        .filter(|item| Some(item.id.as_str()) != skip)
        .map(|item| {
            let label = item.title;
            let chip_text = label.clone();
            PickerOption {
                id: item.id,
                label,
                secondary: item.description.filter(|d| !d.trim().is_empty()),
                chip: ViewFn::from(move || {
                    let chip_text = chip_text.clone();
                    view! {
                        <Badge
                            appearance=BadgeAppearance::Tint
                            color=BadgeColor::Brand
                            size=BadgeSize::Small
                        >
                            {chip_text}
                        </Badge>
                    }
                }),
            }
        })
        .collect();
    if options.is_empty() {
        Vec::new()
    } else {
        vec![PickerGroup {
            label: GROUPS_LABEL.to_string(),
            options,
        }]
    }
}

/// Loads the permission groups a picker offers, again whenever `refresh` changes. `skip`
/// leaves one group out.
pub fn use_group_options(
    refresh: RwSignal<u64>,
    skip: Signal<Option<String>>,
) -> Signal<Vec<PickerGroup>> {
    let groups = Resource::new(
        move || refresh.get(),
        |_| async move {
            search_principals(
                vec![PermissionSearchSourceId::PermissionGroup.into()],
                None,
                MAX_GROUP_OPTIONS,
            )
            .await
        },
    );
    Signal::derive(move || {
        let items = groups.get().and_then(Result::ok).unwrap_or_default();
        group_options(items, skip.get().as_deref())
    })
}

/// `on_change` for a one-shot picker: runs `act` on the newest pick, then empties the
/// picker so it's ready for the next one.
///
/// The picker calls `on_change` from inside an effect that re-runs at once, which would
/// cancel any task `act` scopes to it. `act` runs under the owner that called
/// `act_on_pick` (the page) instead, so its server call lives as long as the page.
pub fn act_on_pick(
    value: RwSignal<Vec<PickedPrincipal>>,
    act: impl Fn(PickTarget) + Send + Sync + 'static,
) -> Callback<Vec<PickedPrincipal>> {
    let owner = Owner::current();
    Callback::new(move |picks: Vec<PickedPrincipal>| {
        let target = picks.last().and_then(pick_target);
        value.set(Vec::new());
        if let Some(target) = target {
            match &owner {
                Some(owner) => owner.with(|| act(target)),
                None => act(target),
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pick(kind: PrincipalKind, id: &str) -> PickedPrincipal {
        PickedPrincipal {
            kind,
            id: id.into(),
            label: id.into(),
            secondary: None,
            image_url: None,
        }
    }

    fn group_item(id: &str, title: &str) -> SearchSourceItem {
        SearchSourceItem {
            source_id: PermissionSearchSourceId::PermissionGroup.as_str().into(),
            id: id.into(),
            title: title.into(),
            description: Some("Readers".into()),
            kind: "group".into(),
            image_url: None,
        }
    }

    #[test]
    fn people_service_users_and_groups_map_to_targets_happy() {
        assert_eq!(
            pick_target(&pick(PrincipalKind::User, "u1")),
            Some(PickTarget::User("u1".into()))
        );
        assert_eq!(
            pick_target(&pick(PrincipalKind::ServiceUser, "s1")),
            Some(PickTarget::ServiceUser("s1".into()))
        );
        assert_eq!(
            pick_target(&pick(PrincipalKind::Extra(GROUPS_LABEL.into()), "g1")),
            Some(PickTarget::Group("g1".into()))
        );
    }

    #[test]
    fn teams_and_other_extras_map_to_nothing_sad() {
        assert_eq!(pick_target(&pick(PrincipalKind::Team, "t1")), None);
        assert_eq!(
            pick_target(&pick(PrincipalKind::Extra("From this event".into()), "x")),
            None
        );
    }

    #[test]
    fn principal_refs_key_like_picker_rows_happy() {
        let group = PrincipalRefDto {
            kind: GaugeKind::Group,
            id: "g1".into(),
            label: "Readers".into(),
            owner_label: None,
        };
        let service_user = PrincipalRefDto {
            kind: GaugeKind::ServiceUser,
            id: "s1".into(),
            label: "agent".into(),
            owner_label: Some("Admin".into()),
        };
        assert_eq!(to_picked(&group).key(), "extra:g1");
        assert_eq!(
            pick_target(&to_picked(&group)),
            Some(PickTarget::Group("g1".into()))
        );
        assert_eq!(to_picked(&service_user).key(), "service_user:s1");
    }

    #[test]
    fn user_ref_is_never_a_group_sad() {
        let user = PrincipalRefDto {
            kind: GaugeKind::User,
            id: "g1".into(),
            label: "Gus".into(),
            owner_label: None,
        };
        assert_eq!(to_picked(&user).key(), "user:g1");
        assert_ne!(to_picked(&user).key(), "extra:g1");
    }

    #[test]
    fn group_options_keep_order_and_skip_current_happy() {
        let groups = group_options(
            vec![
                group_item("a", "Alpha"),
                group_item("b", "Beta"),
                group_item("c", "Gamma"),
            ],
            Some("b"),
        );
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].label, GROUPS_LABEL);
        let ids: Vec<_> = groups[0].options.iter().map(|o| o.id.as_str()).collect();
        assert_eq!(ids, ["a", "c"]);
        assert_eq!(groups[0].options[0].secondary.as_deref(), Some("Readers"));
    }

    #[test]
    fn no_groups_gives_no_picker_group_sad() {
        assert!(group_options(Vec::new(), None).is_empty());
        assert!(group_options(vec![group_item("only", "Only")], Some("only")).is_empty());
    }
}
