//! "Remove Ravi Menon from Release Captains?": what the person loses, what they keep, then
//! the code dialog.
//!
//! The lists come from [`get_principal_access`](crate::server::get_principal_access): a permission is lost when every path to it
//! runs through what's being removed, and kept (with the path that keeps it) otherwise.

use gauge::types::{AccessPath, PermissionRef, PrincipalKey, PrincipalKind, PrincipalRefDto};
use leptos::prelude::*;
use uf_product::components::{Body1, Body1Strong, Caption1};
use uf_product::primitives::{
    Button, ButtonAppearance, Dialog, DialogActions, DialogBody, DialogContent, DialogSurface,
    DialogTitle, Flex, FlexGap, SkeletonItem,
};

use crate::pages::shared::words::{first_name, path_text, plain_name};
use crate::server::{get_principal_access, SUPER_USER_GROUP_ID};

/// What someone is being removed from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RemoveFrom {
    /// A group's members.
    Group {
        /// Group id.
        id: String,
        /// Group name.
        name: String,
    },
    /// A permission's allow list.
    AllowList {
        /// Permission id.
        id: String,
        /// Permission display name.
        name: String,
    },
}

impl RemoveFrom {
    /// The name in "Remove from {name}".
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Group { name, .. } | Self::AllowList { name, .. } => name,
        }
    }
}

/// Who is being removed, and from what.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoveTarget {
    /// The person, service user or group leaving.
    pub principal: PrincipalRefDto,
    /// What they leave.
    pub from: RemoveFrom,
}

/// The code dialog's sentence for removing `target` ("Remove Ravi Menon from Release Captains").
#[must_use]
pub fn remove_action(target: &RemoveTarget) -> String {
    match &target.from {
        RemoveFrom::Group { name, .. } => {
            format!("Remove {} from {name}", plain_name(&target.principal.label))
        }
        RemoveFrom::AllowList { name, .. } => {
            format!(
                "Remove {} from the allow list of {name}",
                target.principal.label
            )
        }
    }
}

fn path_runs_through(path: &AccessPath, from: &RemoveFrom) -> bool {
    match (path, from) {
        (AccessPath::ThroughGroup(groups), RemoveFrom::Group { id, .. }) => {
            groups.last().is_some_and(|g| &g.id == id)
        }
        (AccessPath::Direct, RemoveFrom::AllowList { .. }) => true,
        _ => false,
    }
}

/// Split someone's access into what they lose and what they keep when they leave `from`.
///
/// Lost: permissions whose every path runs through `from`. Kept: those permissions (the ones
/// `from` touches at all) that still have another path, named with it.
#[must_use]
pub fn loses_and_keeps(
    access: &[(PermissionRef, Vec<AccessPath>)],
    from: &RemoveFrom,
) -> (Vec<String>, Vec<String>) {
    let mut loses = Vec::new();
    let mut keeps = Vec::new();
    for (permission, paths) in access {
        let (through, other): (Vec<&AccessPath>, Vec<&AccessPath>) =
            paths.iter().partition(|p| path_runs_through(p, from));
        let touches = !through.is_empty()
            || matches!(from, RemoveFrom::AllowList { id, .. } if id == &permission.id);
        if !touches {
            continue;
        }
        match other.first() {
            None => loses.push(permission.display_name.clone()),
            Some(path) => {
                let how = path_text(path);
                let how = how.strip_prefix("Through ").map_or_else(
                    || format!("from the {}", how.to_lowercase()),
                    |g| format!("through {g}"),
                );
                keeps.push(format!("{}, {how}", permission.display_name));
            }
        }
    }
    (loses, keeps)
}

/// Remove-from-group or remove-from-allow-list confirm. `on_confirm` runs the removal (which
/// asks for the code); the dialog stays open until the caller closes it.
#[component]
pub fn RemoveConfirmDialog(
    /// Open state.
    open: RwSignal<bool>,
    /// Who is leaving, and from what.
    target: RwSignal<Option<RemoveTarget>>,
    /// Runs on the Danger button.
    on_confirm: Callback<()>,
) -> impl IntoView {
    let access = Resource::new(
        move || {
            target.get().filter(|_| open.get()).and_then(|t| {
                matches!(
                    t.principal.kind,
                    PrincipalKind::User | PrincipalKind::ServiceUser
                )
                .then(|| PrincipalKey {
                    kind: t.principal.kind.clone(),
                    id: t.principal.id.clone(),
                })
            })
        },
        |key| async move {
            match key {
                Some(key) => get_principal_access(key).await.map(Some),
                None => Ok(None),
            }
        },
    );
    let title = move || {
        target.get().map_or_else(String::new, |t| {
            format!(
                "Remove {} from {}?",
                plain_name(&t.principal.label),
                t.from.name()
            )
        })
    };
    let button = move || {
        target.get().map_or_else(String::new, |t| match t.from {
            RemoveFrom::Group { name, .. } => format!("Remove from {name}"),
            RemoveFrom::AllowList { .. } => "Remove from the allow list".to_string(),
        })
    };
    let lists = move || {
        let t = target.get()?;
        let first = first_name(&t.principal.label);
        if t.principal.kind == PrincipalKind::Group {
            return Some(view! {
                <Body1>{format!("Everyone in {} loses what {} gives them, unless they hold it another way.", t.principal.label, t.from.name())}</Body1>
            }.into_any());
        }
        Some(match access.get() {
            None => view! { <SkeletonItem width="100%".to_string() /> }.into_any(),
            Some(Err(_)) => view! {
                <Body1>{format!("Gauge couldn't list what {first} would lose. Removing still takes away only what comes from {}.", t.from.name())}</Body1>
            }.into_any(),
            Some(Ok(access)) => {
                let (loses, keeps) = access
                    .map(|a| loses_and_keeps(&a.permissions, &t.from))
                    .unwrap_or_default();
                let lose_items = if loses.is_empty() { vec!["Nothing".to_string()] } else { loses };
                let keep_items = if keeps.is_empty() { vec!["Nothing else from this".to_string()] } else { keeps };
                view! {
                    <Flex vertical=true gap=FlexGap::Small>
                        <div data-testid="gauge-remove-loses">
                            <Body1Strong>{format!("{first} loses")}</Body1Strong>
                            <ul>{lose_items.into_iter().map(|x| view! { <li>{x}</li> }).collect_view()}</ul>
                        </div>
                        <div data-testid="gauge-remove-keeps">
                            <Body1Strong>{format!("{first} keeps")}</Body1Strong>
                            <ul>{keep_items.into_iter().map(|x| view! { <li>{x}</li> }).collect_view()}</ul>
                        </div>
                    </Flex>
                }.into_any()
            }
        })
    };
    let caption = move || {
        target.get().map_or_else(String::new, |t| {
            let first = first_name(&t.principal.label);
            let code = match &t.from {
                RemoveFrom::Group { id, .. } if id == SUPER_USER_GROUP_ID => {
                    "Super User changes need a fresh authenticator code."
                }
                _ => "You'll confirm with your authenticator code.",
            };
            format!("{first} gets a notification. {code}")
        })
    };
    view! {
        <Dialog open=open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>{title}</DialogTitle>
                    <DialogContent>
                        <div data-testid="gauge-remove-confirm">
                            <Flex vertical=true gap=FlexGap::Medium>
                                {lists}
                                <Caption1>{caption}</Caption1>
                            </Flex>
                        </div>
                    </DialogContent>
                    <DialogActions>
                        <Button appearance=ButtonAppearance::Subtle on_click=Callback::new(move |_| open.set(false))>
                            "Cancel"
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Danger
                            attr:data-testid="gauge-remove-confirm-button"
                            on_click=Callback::new(move |_| on_confirm.run(()))
                        >
                            {button}
                        </Button>
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
        </Dialog>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gauge::types::GroupRef;

    fn perm(id: &str, name: &str) -> PermissionRef {
        PermissionRef {
            id: id.into(),
            key: name.replace(' ', ""),
            display_name: name.into(),
        }
    }

    fn through(names: &[&str]) -> AccessPath {
        AccessPath::ThroughGroup(
            names
                .iter()
                .map(|n| GroupRef {
                    id: n.to_lowercase(),
                    name: (*n).into(),
                })
                .collect(),
        )
    }

    fn captains() -> RemoveFrom {
        RemoveFrom::Group {
            id: "captains".into(),
            name: "Captains".into(),
        }
    }

    #[test]
    fn leaving_a_group_loses_only_what_it_alone_gives_happy() {
        let access = vec![
            (perm("p1", "Approve releases"), vec![through(&["Captains"])]),
            (
                perm("p2", "Read metrics"),
                vec![through(&["Captains"]), through(&["Readers"])],
            ),
            (perm("p3", "Unrelated"), vec![through(&["Readers"])]),
        ];
        let (loses, keeps) = loses_and_keeps(&access, &captains());
        assert_eq!(loses, ["Approve releases"]);
        assert_eq!(keeps, ["Read metrics, through Readers"]);
    }

    #[test]
    fn nested_membership_counts_by_innermost_group_happy() {
        // Allow list names Admins; the person is in Captains, which sits inside Admins.
        let access = vec![(perm("p1", "Deploy"), vec![through(&["Admins", "Captains"])])];
        let (loses, _) = loses_and_keeps(&access, &captains());
        assert_eq!(loses, ["Deploy"]);
    }

    #[test]
    fn allow_list_removal_keeps_group_path_sad() {
        let from = RemoveFrom::AllowList {
            id: "p1".into(),
            name: "Deploy".into(),
        };
        let access = vec![(
            perm("p1", "Deploy"),
            vec![AccessPath::Direct, through(&["Captains"])],
        )];
        let (loses, keeps) = loses_and_keeps(&access, &from);
        assert!(loses.is_empty());
        assert_eq!(keeps, ["Deploy, through Captains"]);
    }

    #[test]
    fn remove_action_names_both_sides_happy() {
        let target = RemoveTarget {
            principal: PrincipalRefDto {
                kind: PrincipalKind::User,
                id: "u1".into(),
                label: "Ravi Menon".into(),
                owner_label: None,
            },
            from: captains(),
        };
        assert_eq!(remove_action(&target), "Remove Ravi Menon from Captains");
    }
}
