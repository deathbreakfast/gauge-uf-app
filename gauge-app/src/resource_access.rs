//! Resource bundles and managed groups as the Gauge pages show them.
//!
//! A resource bundle is the domain, owners group and per-action permissions Gauge keeps for
//! one resource of another app (a Chronon job, a Photon topic). A managed group is one whose
//! members another product syncs, such as a Polaron team. The DTOs here carry what the domain,
//! group and request pages need; the helpers turn Gauge's ids and action names into the
//! words those pages use ("Chronon job billing-rollup · Can view").

use serde::{Deserialize, Serialize};

/// Prefix of a resource bundle's domain id: `rp_domain_{kind}_{resource}`.
pub const DOMAIN_PREFIX: &str = "rp_domain_";
/// Prefix of a resource bundle's owners group id: `rp_owners_{kind}_{resource}`.
pub const OWNERS_PREFIX: &str = "rp_owners_";

/// Actions in the order the pages list them.
pub const ACTION_ORDER: [&str; 7] = [
    "View", "Run", "Unmask", "Reveal", "Edit", "Delete", "Maintain",
];

/// The resource a bundle belongs to, with the link back to it when the host knows one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceLink {
    /// Kind prefix, such as `chronon_job`.
    pub kind: String,
    /// Kind in words, such as `Chronon job`.
    pub kind_label: String,
    /// Raw resource id, such as `billing-rollup`.
    pub resource_id: String,
    /// Domain id of the bundle.
    pub domain_id: String,
    /// App that owns the resource, such as `Chronon`, when the host registered the kind.
    pub app: Option<String>,
    /// Link to the resource in its app, when the host registered the kind.
    pub href: Option<String>,
}

impl ResourceLink {
    /// "Chronon job billing-rollup".
    #[must_use]
    pub fn title(&self) -> String {
        format!("{} {}", self.kind_label, self.resource_id)
    }
}

/// One action permission of a bundle and who holds it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundlePermissionRow {
    /// Action name, such as `Run`.
    pub action: String,
    /// Permission record id.
    pub permission_id: String,
    /// Permission name, such as `chronon_job.billing_rollup_5c1e9a07.Run`.
    pub permission_name: String,
    /// Labels of the principals granted this permission ("Owners" for the owners group).
    pub held_by: Vec<String>,
}

/// One role a bundle offers and the grants behind it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleRoleRow {
    /// "Can view", "Can run", "Owner".
    pub role: String,
    /// "View, Run", or the owners group line for Owner.
    pub grants: String,
}

/// A resource-scoped domain as its domain page shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceBundleDto {
    /// The resource and the link back to it.
    pub resource: ResourceLink,
    /// The bundle's owners group id.
    pub owners_group_id: String,
    /// The app's admin group the host named as default owner, when it named one.
    pub default_owner: Option<String>,
    /// Each action permission, in [`ACTION_ORDER`].
    pub permissions: Vec<BundlePermissionRow>,
    /// The roles the bundle offers.
    pub roles: Vec<BundleRoleRow>,
    /// False when Gauge withheld the grants (the reader isn't an owner or Super User).
    pub holders_visible: bool,
}

/// What another product's management of a group means for its page.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedGroupDto {
    /// Product id that syncs the members, such as `polaron`.
    pub managed_by: String,
    /// That product's name, such as `Polaron`.
    pub source_label: String,
    /// The product's own id for what the group mirrors (a team id).
    pub external_ref: Option<String>,
    /// Link to the team (or org) in the managing product, when known.
    pub source_href: Option<String>,
    /// True while the mirrored team is archived; the group then has no members.
    pub archived: bool,
}

/// A row on the Groups page: the group, and its management when another product syncs it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupRowDto {
    /// The group as Gauge lists it.
    pub group: gauge::types::PermissionGroupDetailDto,
    /// Set for a managed (team) group.
    pub managed: Option<ManagedGroupDto>,
}

/// A resource a group holds a role on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupResourceAccessRow {
    /// The resource.
    pub resource: ResourceLink,
    /// The group's role there, such as "Can edit".
    pub role: String,
}

/// "chronon_job" → "Chronon job".
#[must_use]
pub fn humanize_kind(prefix: &str) -> String {
    let words = prefix.replace('_', " ");
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The kind prefix in a bundle's domain id, given the normalized resource id fragment.
///
/// `rp_domain_chronon_job_billing_rollup_5c1e9a07` with fragment `billing_rollup_5c1e9a07`
/// gives `chronon_job`.
#[must_use]
pub fn kind_of_domain<'a>(domain_id: &'a str, fragment: &str) -> Option<&'a str> {
    let rest = domain_id.strip_prefix(DOMAIN_PREFIX)?;
    let kind = rest.strip_suffix(fragment)?.strip_suffix('_')?;
    (!kind.is_empty()).then_some(kind)
}

/// The domain id behind a bundle's owners group id, or `None` for any other group.
#[must_use]
pub fn domain_of_owners_group(group_id: &str) -> Option<String> {
    group_id
        .strip_prefix(OWNERS_PREFIX)
        .filter(|rest| !rest.is_empty())
        .map(|rest| format!("{DOMAIN_PREFIX}{rest}"))
}

/// The action at the end of a bundle permission name (`chronon_job.x.Run` → `Run`).
#[must_use]
pub fn action_of_permission_name(name: &str) -> Option<&str> {
    name.rsplit_once('.')
        .map(|(_, action)| action)
        .filter(|action| ACTION_ORDER.contains(action))
}

/// The role a request for one action asks for: "Can view", "Can run", "Owner" for Maintain.
#[must_use]
pub fn role_label_for_action(action: &str) -> String {
    match action {
        "View" => "Can view".to_string(),
        "Run" => "Can run".to_string(),
        "Unmask" => "Can read unmasked".to_string(),
        "Edit" => "Can edit".to_string(),
        "Maintain" => "Owner".to_string(),
        other => other.to_string(),
    }
}

/// `actions` sorted into [`ACTION_ORDER`], each once.
fn in_order<'a>(actions: impl IntoIterator<Item = &'a str>) -> Vec<&'static str> {
    let actions: Vec<&str> = actions.into_iter().collect();
    ACTION_ORDER
        .into_iter()
        .filter(|a| actions.contains(a))
        .collect()
}

/// The grants each non-owner role stands for on a bundle with `bundle_actions`, as Gauge
/// computes them: a role is offered only when the bundle has its actions.
fn role_grants(bundle_actions: &[&str]) -> Vec<(&'static str, Vec<&'static str>)> {
    let has = |a: &str| bundle_actions.contains(&a);
    if !has("View") {
        return Vec::new();
    }
    let mut roles = vec![("Can view", vec!["View"])];
    if has("Run") {
        roles.push(("Can run", vec!["View", "Run"]));
    }
    if has("Unmask") {
        roles.push(("Can read unmasked", vec!["View", "Unmask"]));
    }
    if has("Edit") {
        let mut grants = vec!["View"];
        if has("Run") {
            grants.push("Run");
        }
        grants.push("Edit");
        roles.push(("Can edit", grants));
    }
    roles
}

/// The roles table for a bundle: each role with its grants, then Owner.
#[must_use]
pub fn bundle_roles(bundle_actions: &[&str], owners_group_id: &str) -> Vec<BundleRoleRow> {
    let mut rows: Vec<BundleRoleRow> = role_grants(bundle_actions)
        .into_iter()
        .map(|(role, grants)| BundleRoleRow {
            role: role.to_string(),
            grants: grants.join(", "),
        })
        .collect();
    rows.push(BundleRoleRow {
        role: "Owner".to_string(),
        grants: format!("Member of {owners_group_id}: every action, including Maintain"),
    });
    rows
}

/// The role a principal's direct grants amount to on a bundle with `bundle_actions`.
///
/// Matches Gauge's role mapping: every action including Maintain is Owner, a set equal to a
/// role's grants is that role, anything else is "Custom: …". `None` for no grants.
#[must_use]
pub fn role_label_for_grants(held: &[&str], bundle_actions: &[&str]) -> Option<String> {
    let held = in_order(held.iter().copied());
    if held.is_empty() {
        return None;
    }
    if held.contains(&"Maintain") && bundle_actions.iter().all(|a| held.contains(a)) {
        return Some("Owner".to_string());
    }
    for (role, grants) in role_grants(bundle_actions) {
        if grants == held {
            return Some(role.to_string());
        }
    }
    Some(format!("Custom: {}", held.join(", ")))
}

/// "Chronon job billing-rollup · Can view", the line a resource request reads as.
#[must_use]
pub fn request_label(resource: &ResourceLink, role: &str) -> String {
    format!("{} · {role}", resource.title())
}

/// The name and link of the product that manages a group, for the products Gauge knows.
///
/// Polaron team groups (`polaron.team.<id>`) link to the team, org groups to the org.
#[must_use]
pub fn managed_source(
    managed_by: &str,
    group_id: &str,
    external_ref: Option<&str>,
) -> (String, Option<String>) {
    match managed_by {
        "polaron" => {
            let href = external_ref.and_then(|ext| {
                if group_id.starts_with("polaron.org.") {
                    Some(format!("/polaron/orgs/{ext}"))
                } else if group_id.starts_with("polaron.team.") {
                    Some(format!("/polaron/teams/{ext}"))
                } else {
                    None
                }
            });
            ("Polaron".to_string(), href)
        }
        other => (humanize_kind(other), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link() -> ResourceLink {
        ResourceLink {
            kind: "chronon_job".into(),
            kind_label: "Chronon job".into(),
            resource_id: "billing-rollup".into(),
            domain_id: "rp_domain_chronon_job_billing_rollup_5c1e9a07".into(),
            app: Some("Chronon".into()),
            href: Some("/chronon/jobs/billing-rollup".into()),
        }
    }

    #[test]
    fn humanize_kind_reads_as_words() {
        assert_eq!(humanize_kind("chronon_job"), "Chronon job");
        assert_eq!(humanize_kind("spectra_table"), "Spectra table");
        assert_eq!(humanize_kind(""), "");
    }

    #[test]
    fn kind_of_domain_strips_prefix_and_fragment() {
        let id = "rp_domain_chronon_job_billing_rollup_5c1e9a07";
        assert_eq!(
            kind_of_domain(id, "billing_rollup_5c1e9a07"),
            Some("chronon_job")
        );
        assert_eq!(kind_of_domain(id, "other_00000000"), None);
        assert_eq!(kind_of_domain("ops", "billing_rollup_5c1e9a07"), None);
        assert_eq!(
            kind_of_domain(
                "rp_domain_billing_rollup_5c1e9a07",
                "billing_rollup_5c1e9a07"
            ),
            None
        );
    }

    #[test]
    fn owners_group_maps_to_its_domain() {
        assert_eq!(
            domain_of_owners_group("rp_owners_chronon_job_x_1").as_deref(),
            Some("rp_domain_chronon_job_x_1")
        );
        assert_eq!(domain_of_owners_group("deployers"), None);
        assert_eq!(domain_of_owners_group("rp_owners_"), None);
    }

    #[test]
    fn action_of_permission_name_takes_known_suffix() {
        assert_eq!(
            action_of_permission_name("chronon_job.x_1.Run"),
            Some("Run")
        );
        assert_eq!(action_of_permission_name("CanDeploy"), None);
        assert_eq!(action_of_permission_name("a.b.Launch"), None);
    }

    #[test]
    fn role_for_one_action() {
        assert_eq!(role_label_for_action("View"), "Can view");
        assert_eq!(role_label_for_action("Unmask"), "Can read unmasked");
        assert_eq!(role_label_for_action("Maintain"), "Owner");
        assert_eq!(role_label_for_action("Delete"), "Delete");
    }

    #[test]
    fn roles_follow_the_bundle_actions() {
        let job = ["View", "Run", "Edit", "Maintain"];
        let roles = bundle_roles(&job, "rp_owners_x");
        let pairs: Vec<(&str, &str)> = roles
            .iter()
            .map(|r| (r.role.as_str(), r.grants.as_str()))
            .collect();
        assert_eq!(
            pairs[..3],
            [
                ("Can view", "View"),
                ("Can run", "View, Run"),
                ("Can edit", "View, Run, Edit")
            ]
        );
        assert_eq!(roles[3].role, "Owner");
        assert!(roles[3].grants.contains("rp_owners_x"));

        let table = ["View", "Unmask", "Edit", "Maintain"];
        let names: Vec<String> = bundle_roles(&table, "o")
            .into_iter()
            .map(|r| r.role)
            .collect();
        assert_eq!(
            names,
            ["Can view", "Can read unmasked", "Can edit", "Owner"]
        );
    }

    #[test]
    fn grants_map_to_roles_like_gauge() {
        let job = ["View", "Run", "Edit", "Maintain"];
        assert_eq!(
            role_label_for_grants(&["View"], &job).as_deref(),
            Some("Can view")
        );
        assert_eq!(
            role_label_for_grants(&["Run", "View"], &job).as_deref(),
            Some("Can run")
        );
        assert_eq!(
            role_label_for_grants(&["View", "Run", "Edit"], &job).as_deref(),
            Some("Can edit")
        );
        assert_eq!(role_label_for_grants(&job, &job).as_deref(), Some("Owner"));
        assert_eq!(
            role_label_for_grants(&["Edit"], &job).as_deref(),
            Some("Custom: Edit")
        );
        assert_eq!(role_label_for_grants(&[], &job), None);
        // Without Run on the kind, Can edit is View and Edit.
        assert_eq!(
            role_label_for_grants(&["View", "Edit"], &["View", "Edit", "Maintain"]).as_deref(),
            Some("Can edit")
        );
    }

    #[test]
    fn request_reads_as_resource_and_role() {
        assert_eq!(
            request_label(&link(), "Can view"),
            "Chronon job billing-rollup · Can view"
        );
    }

    #[test]
    fn polaron_groups_link_back() {
        assert_eq!(
            managed_source("polaron", "polaron.team.t81kq2", Some("t81kq2")),
            (
                "Polaron".to_string(),
                Some("/polaron/teams/t81kq2".to_string())
            )
        );
        assert_eq!(
            managed_source("polaron", "polaron.org.o1", Some("o1"))
                .1
                .as_deref(),
            Some("/polaron/orgs/o1")
        );
        assert_eq!(managed_source("polaron", "polaron.team.t1", None).1, None);
        assert_eq!(
            managed_source("hr_sync", "g", Some("x")),
            ("Hr sync".to_string(), None)
        );
    }
}
