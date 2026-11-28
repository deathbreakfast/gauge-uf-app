//! Give access, from a permission or from a person.
//!
//! From a permission: pick a person, service user or group. When a group on the allow list
//! already holds the permission (and the viewer can change that group), the dialog suggests
//! adding the person to it instead of a direct grant, so access stays in one place.
//!
//! From a person: search groups and permissions by name, groups first.
//!
//! Every change goes through the code dialog, which names what it confirms.

use gauge::types::PrincipalKind as GaugeKind;
use gauge::types::PrincipalRefDto;
use lepton_shell::people::{PersonPicker, PickedPrincipal};
use leptos::prelude::*;
use uf_product::components::Caption1;
use uf_product::primitives::{
    Button, ButtonAppearance, Dialog, DialogActions, DialogBody, DialogContent, DialogSurface,
    DialogTitle, Field, Flex, FlexGap, Input, InputAppearance, MessageBar, MessageBarBody,
    MessageBarIntent, MessageBarLayout, MessageBarTitle, Radio, RadioGroup,
};

use crate::components::error_bar::MaybeErrorBar;
use crate::pages::principal_pick::{
    pick_target, to_picked, use_group_search, PickTarget, GROUPS_LABEL,
};
use crate::pages::shared::words::first_name;
use crate::pages::step_up::{spawn_with_fresh_totp_to, spawn_with_step_up_to};
use crate::server::{
    add_group_service_user, add_group_user, add_permission_group, add_permission_user,
    list_permissions, search_group_options, SUPER_USER_GROUP_ID,
};

/// The permission access is given to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GivePermission {
    /// Permission id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// The key code checks.
    pub key: String,
    /// Groups on its allow list the viewer may add members to, `(id, name)`.
    pub suggest_groups: Vec<(String, String)>,
    /// Who is on the allow list now, shown disabled in the picker.
    pub allow_list: Vec<PrincipalRefDto>,
}

/// Radio value for a direct grant.
const DIRECT: &str = "direct";

/// What Give does for a pick and a radio choice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GiveAction {
    /// Add a user to a group.
    UserToGroup { user: String, group: String },
    /// Add a service user to a group.
    ServiceUserToGroup { service_user: String, group: String },
    /// Put a user on the allow list.
    UserToAllowList { user: String },
    /// Put a group on the allow list.
    GroupToAllowList { group: String },
}

/// The action for `target` under `choice` (`"direct"` or a group id). Service users join a
/// group; they can't go on an allow list from here.
#[must_use]
pub fn give_action(target: &PickTarget, choice: Option<&str>) -> Option<GiveAction> {
    let group = choice.filter(|c| *c != DIRECT).map(str::to_string);
    match (target, group) {
        (PickTarget::User(user), Some(group)) => Some(GiveAction::UserToGroup {
            user: user.clone(),
            group,
        }),
        (PickTarget::User(user), None) => Some(GiveAction::UserToAllowList { user: user.clone() }),
        (PickTarget::ServiceUser(id), Some(group)) => Some(GiveAction::ServiceUserToGroup {
            service_user: id.clone(),
            group,
        }),
        (PickTarget::ServiceUser(_), None) => None,
        (PickTarget::Group(group), _) => Some(GiveAction::GroupToAllowList {
            group: group.clone(),
        }),
    }
}

/// Run `action`, asking for the code; `confirm` names it in the code dialog.
fn run_give(
    action: GiveAction,
    permission_id: String,
    confirm: String,
    error: RwSignal<Option<String>>,
    done: Callback<()>,
) {
    match action {
        GiveAction::UserToGroup { user, group } if group == SUPER_USER_GROUP_ID => {
            spawn_with_fresh_totp_to(confirm, error, done, move |code| {
                add_group_user(group.clone(), user.clone(), code)
            });
        }
        GiveAction::UserToGroup { user, group } => {
            spawn_with_step_up_to(confirm, error, done, move || {
                add_group_user(group.clone(), user.clone(), String::new())
            });
        }
        GiveAction::ServiceUserToGroup {
            service_user,
            group,
        } => spawn_with_step_up_to(confirm, error, done, move || {
            add_group_service_user(group.clone(), service_user.clone(), String::new())
        }),
        GiveAction::UserToAllowList { user } => {
            spawn_with_step_up_to(confirm, error, done, move || {
                add_permission_user(permission_id.clone(), user.clone())
            });
        }
        GiveAction::GroupToAllowList { group } => {
            spawn_with_step_up_to(confirm, error, done, move || {
                add_permission_group(permission_id.clone(), group.clone())
            });
        }
    }
}

/// "Give Approve releases": person picker, then the group suggestion.
#[component]
pub fn GiveAccessDialog(
    /// Open state.
    open: RwSignal<bool>,
    /// The permission, its allow list and the groups to suggest.
    #[prop(into)]
    permission: Signal<GivePermission>,
    /// Runs after a change lands, with a sentence for the success bar.
    on_done: Callback<String>,
) -> impl IntoView {
    let picked = RwSignal::new(Vec::<PickedPrincipal>::new());
    let query = RwSignal::new(String::new());
    let choice = RwSignal::new(None::<String>);
    let error = RwSignal::new(None::<String>);
    let groups = use_group_search(query, Signal::derive(|| None));
    let pick = Memo::new(move |_| {
        picked
            .get()
            .last()
            .and_then(|p| pick_target(p).map(|t| (t, p.label.clone())))
    });
    // A person or service user gets the suggestion; preselect its first group.
    Effect::new(move |_| {
        let suggest = permission.with(|p| p.suggest_groups.first().map(|g| g.0.clone()));
        match pick.get() {
            Some((PickTarget::User(_) | PickTarget::ServiceUser(_), _)) => {
                choice.set(suggest.or_else(|| Some(DIRECT.to_string())));
            }
            _ => choice.set(None),
        }
    });
    let reset = move || {
        picked.set(Vec::new());
        query.set(String::new());
        choice.set(None);
        error.set(None);
    };
    let show_suggestion = move || {
        matches!(
            pick.get(),
            Some((PickTarget::User(_) | PickTarget::ServiceUser(_), _))
        ) && permission.with(|p| !p.suggest_groups.is_empty())
    };
    let service_user_without_group = move || {
        matches!(pick.get(), Some((PickTarget::ServiceUser(_), _)))
            && permission.with(|p| p.suggest_groups.is_empty())
    };
    let button_label = move || {
        let group_name = choice.get().and_then(|c| {
            permission.with(|p| {
                p.suggest_groups
                    .iter()
                    .find(|g| g.0 == c)
                    .map(|g| g.1.clone())
            })
        });
        match (pick.get(), group_name) {
            (Some((PickTarget::User(_) | PickTarget::ServiceUser(_), _)), Some(name)) => {
                format!("Add to {name}")
            }
            _ => "Give access".to_string(),
        }
    };
    let give = move |_| {
        let Some((target, label)) = pick.get() else {
            return;
        };
        let Some(action) = give_action(&target, choice.get().as_deref()) else {
            return;
        };
        let p = permission.get();
        let confirm = match &action {
            GiveAction::UserToGroup { group, .. }
            | GiveAction::ServiceUserToGroup { group, .. } => {
                let name = p
                    .suggest_groups
                    .iter()
                    .find(|g| &g.0 == group)
                    .map_or_else(|| "the group".to_string(), |g| g.1.clone());
                format!("Add {label} to {name}")
            }
            GiveAction::UserToAllowList { .. } | GiveAction::GroupToAllowList { .. } => {
                format!("Give {label} {}", p.name)
            }
        };
        let notice = format!("{} done.", confirm);
        let done = Callback::new(move |()| {
            open.set(false);
            reset();
            on_done.run(notice.clone());
        });
        run_give(action, p.id, confirm, error, done);
    };

    view! {
        <Dialog open=open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>{move || format!("Give {}", permission.with(|p| p.name.clone()))}</DialogTitle>
                    <DialogContent>
                        <div data-testid="gauge-give-access">
                        <Flex vertical=true gap=FlexGap::Medium>
                            <div id="gauge-give-access-picker">
                                <PersonPicker
                                    value=picked
                                    single=true
                                    service_users=true
                                    query=query
                                    extra_groups=groups
                                    exclude=Signal::derive(move || permission.with(|p| p.allow_list.iter().map(to_picked).collect::<Vec<_>>()))
                                    label="Person, service user or group"
                                    placeholder=format!("Search by name, @handle or {}", GROUPS_LABEL.to_lowercase())
                                />
                            </div>
                            <Show when=move || pick.get().is_none()>
                                <Caption1>{move || format!("They pass the {} check once you confirm.", permission.with(|p| p.key.clone()))}</Caption1>
                            </Show>
                            <Show when=show_suggestion>
                                <div data-testid="gauge-give-access-suggest">
                                    <MessageBar intent=MessageBarIntent::Info layout=MessageBarLayout::Multiline>
                                        <MessageBarBody>
                                            <MessageBarTitle>{move || {
                                                let names = permission.with(|p| p.suggest_groups.iter().map(|g| g.1.clone()).collect::<Vec<_>>().join(" and "));
                                                format!("{names} already has this permission")
                                            }}</MessageBarTitle>
                                            {move || format!("Most people get {} by joining a group that holds it.", permission.with(|p| p.name.clone()))}
                                        </MessageBarBody>
                                    </MessageBar>
                                </div>
                                <RadioGroup bind=choice>
                                    {move || {
                                        let first = pick.get().map(|(_, l)| first_name(&l)).unwrap_or_default();
                                        let is_person = matches!(pick.get(), Some((PickTarget::User(_), _)));
                                        let p = permission.get();
                                        let mut radios = p.suggest_groups.iter().enumerate().map(|(i, (id, name))| {
                                            let label = if i == 0 {
                                                format!("Add {first} to {name} (suggested)")
                                            } else {
                                                format!("Add {first} to {name}")
                                            };
                                            view! { <Radio value=id.clone() label=label /> }.into_any()
                                        }).collect::<Vec<_>>();
                                        if is_person {
                                            radios.push(view! {
                                                <Radio value=DIRECT label=format!("Put {first} on the allow list") />
                                            }.into_any());
                                        }
                                        radios
                                    }}
                                </RadioGroup>
                                <Caption1>{move || {
                                    if choice.get().as_deref() == Some(DIRECT) {
                                        format!("Only {}. Remember to take it off when it's no longer needed.", permission.with(|p| p.name.clone()))
                                    } else {
                                        "They also get everything else the group gives.".to_string()
                                    }
                                }}</Caption1>
                            </Show>
                            <Show when=service_user_without_group>
                                <Caption1>"Service users get permissions by joining a group. Add a group to the allow list first."</Caption1>
                            </Show>
                            <MaybeErrorBar error=error what="this permission" />
                        </Flex>
                        </div>
                    </DialogContent>
                    <DialogActions>
                        <Button appearance=ButtonAppearance::Subtle on_click=Callback::new(move |_| { open.set(false); reset(); })>
                            "Cancel"
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Primary
                            attr:data-testid="gauge-give-access-submit"
                            disabled=Signal::derive(move || pick.get().is_none() || service_user_without_group())
                            on_click=Callback::new(give)
                        >
                            {button_label}
                        </Button>
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
        </Dialog>
    }
}

/// One search result in the from-a-person dialog.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct AccessOption {
    /// `group:<id>` or `perm:<id>`.
    value: String,
    /// "Billing Analysts (group)".
    label: String,
    /// The caption under it.
    caption: String,
}

/// "Give Ravi Menon access": search groups and permissions by name, groups first.
#[component]
pub fn GiveAccessToPersonDialog(
    /// Open state.
    open: RwSignal<bool>,
    /// The person or service user receiving access.
    #[prop(into)]
    principal: Signal<PrincipalRefDto>,
    /// Runs after a change lands, with a sentence for the success bar.
    on_done: Callback<String>,
) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let choice = RwSignal::new(None::<String>);
    let error = RwSignal::new(None::<String>);
    let options = Resource::new(
        move || query.get().trim().to_string(),
        |q| async move {
            if q.chars().count() < 2 {
                return Ok::<_, ServerFnError>(Vec::new());
            }
            let groups = search_group_options(q.clone()).await?;
            let perms = list_permissions(Some(q)).await?;
            let mut out: Vec<AccessOption> = groups
                .into_iter()
                .map(|g| AccessOption {
                    value: format!("group:{}", g.id),
                    label: format!("{} (group)", g.title),
                    caption: g.description.unwrap_or_default(),
                })
                .collect();
            out.extend(perms.into_iter().map(|p| AccessOption {
                value: format!("perm:{}", p.id),
                label: format!(
                    "{} (permission)",
                    if p.display_name.is_empty() {
                        p.name.clone()
                    } else {
                        p.display_name.clone()
                    }
                ),
                caption: p.description,
            }));
            Ok(out)
        },
    );
    let give = move |_| {
        let Some(value) = choice.get() else { return };
        let who = principal.get();
        let found = options
            .get()
            .and_then(Result::ok)
            .and_then(|opts| opts.into_iter().find(|o| o.value == value));
        let Some(option) = found else { return };
        let name = option
            .label
            .rsplit_once(" (")
            .map_or(option.label.clone(), |(n, _)| n.to_string());
        let target = match who.kind {
            GaugeKind::ServiceUser => PickTarget::ServiceUser(who.id.clone()),
            _ => PickTarget::User(who.id.clone()),
        };
        let (action, permission_id, confirm) = if let Some(group) = value.strip_prefix("group:") {
            (
                give_action(&target, Some(group)),
                String::new(),
                format!("Add {} to {name}", who.label),
            )
        } else {
            let perm = value.trim_start_matches("perm:").to_string();
            (
                give_action(&target, None),
                perm,
                format!("Give {} {name}", who.label),
            )
        };
        let Some(action) = action else {
            error.set(Some(
                "validation: Service users get permissions by joining a group".to_string(),
            ));
            return;
        };
        let notice = format!("{confirm} done.");
        let done = Callback::new(move |()| {
            open.set(false);
            choice.set(None);
            query.set(String::new());
            on_done.run(notice.clone());
        });
        run_give(action, permission_id, confirm, error, done);
    };
    view! {
        <Dialog open=open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>{move || format!("Give {} access", principal.with(|p| p.label.clone()))}</DialogTitle>
                    <DialogContent>
                        <div data-testid="gauge-give-person-access">
                        <Flex vertical=true gap=FlexGap::Medium>
                            <Field label="Access to give">
                                <Input bind=query appearance=InputAppearance::with_placeholder("Search groups and permissions by name") />
                            </Field>
                            <Suspense fallback=|| ()>
                                {move || options.get().map(|r| match r {
                                    Ok(opts) if opts.is_empty() => view! {
                                        <Caption1>"Type two letters or more to search."</Caption1>
                                    }.into_any(),
                                    Ok(opts) => view! {
                                        <RadioGroup bind=choice>
                                            {opts.into_iter().map(|o| view! {
                                                <Radio value=o.value label=format!("{} — {}", o.label, o.caption) />
                                            }).collect_view()}
                                        </RadioGroup>
                                    }.into_any(),
                                    Err(e) => view! { <MaybeErrorBar error=RwSignal::new(Some(e.to_string())) what="groups and permissions" /> }.into_any(),
                                })}
                            </Suspense>
                            <MessageBar intent=MessageBarIntent::Info layout=MessageBarLayout::Multiline>
                                <MessageBarBody>
                                    <MessageBarTitle>"Groups first"</MessageBarTitle>
                                    {move || format!("Giving a group keeps access in one place: when the group changes, {}'s access follows.", first_name(&principal.with(|p| p.label.clone())))}
                                </MessageBarBody>
                            </MessageBar>
                            <MaybeErrorBar error=error what="their access" />
                        </Flex>
                        </div>
                    </DialogContent>
                    <DialogActions>
                        <Button appearance=ButtonAppearance::Subtle on_click=Callback::new(move |_| open.set(false))>
                            "Cancel"
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Primary
                            disabled=Signal::derive(move || choice.get().is_none())
                            on_click=Callback::new(give)
                        >
                            "Give access"
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

    #[test]
    fn person_follows_the_suggested_group_happy() {
        let user = PickTarget::User("u1".into());
        assert_eq!(
            give_action(&user, Some("g1")),
            Some(GiveAction::UserToGroup {
                user: "u1".into(),
                group: "g1".into()
            })
        );
        assert_eq!(
            give_action(&user, Some(DIRECT)),
            Some(GiveAction::UserToAllowList { user: "u1".into() })
        );
        assert_eq!(
            give_action(&PickTarget::Group("g2".into()), Some("g1")),
            Some(GiveAction::GroupToAllowList { group: "g2".into() })
        );
    }

    #[test]
    fn service_user_needs_a_group_sad() {
        let bot = PickTarget::ServiceUser("s1".into());
        assert_eq!(give_action(&bot, None), None);
        assert_eq!(give_action(&bot, Some(DIRECT)), None);
        assert!(give_action(&bot, Some("g1")).is_some());
    }
}
