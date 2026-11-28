//! Create a permission: a name people read makes the key code checks, the owners default to
//! a new group named for it, and the description is Markdown.
//!
//! Only Gauge admins create permissions; anyone else gets the denied page before the form.

use gauge::search_sources::PermissionSearchSourceId;
use gauge::types::PermissionCreateInput;
use leptos::prelude::*;
use leptos::task::spawn_local_scoped;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;
use uf_integrations::SearchSourcePicker;
use uf_product::components::{Body1Strong, Caption1, Card, CardContent, ContentContainer, Title3};
use uf_product::primitives::{
    Breadcrumb, BreadcrumbButton, BreadcrumbDivider, BreadcrumbItem, Button, ButtonAppearance,
    Field, Flex, FlexGap, Input, MarkdownEditor, Select, SelectAppearance,
};
use uf_search_core::{SearchSourceItem, SearchSourceKey};

use crate::components::error_bar::{ErrorBar, MaybeErrorBar};
use crate::pages::shared::denied::{DeniedPage, ASK_FOR_GAUGE_ADMIN_HREF};
use crate::pages::step_up::spawn_with_step_up_to;
use crate::server::viewer::use_viewer_access;
use crate::server::{create_permission, list_domains, search_principals};

/// The key made from a name: each word capitalised, letters and digits only
/// ("Page on-call" becomes `PageOnCall`).
#[must_use]
pub fn key_from_name(name: &str) -> String {
    name.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect::<String>()
            })
        })
        .collect()
}

/// The Owned by default for a new permission called `name`.
#[must_use]
pub fn default_owner_text(name: &str) -> String {
    let name = name.trim();
    if name.is_empty() {
        "A new owners group (you are its owner)".to_string()
    } else {
        format!("Owners of {name} (new group, you are its owner)")
    }
}

/// Create permission page.
#[component]
pub fn PermissionCreatePage() -> impl IntoView {
    let viewer = use_viewer_access();
    view! {
        <ContentContainer max_width="900px">
            <Suspense fallback=|| ()>
                <Show
                    when=move || viewer.get().and_then(Result::ok).is_some_and(|v| v.gauge_admin)
                    fallback=|| view! {
                        <DeniedPage
                            heading="Creating permissions needs GaugeAdmin"
                            text="Ask a Gauge admin to create it, or ask for GaugeAdmin if you look after permissions for your team."
                            action="Ask for GaugeAdmin"
                            action_href=ASK_FOR_GAUGE_ADMIN_HREF
                            parent=("Permissions", crate::paths::PERMISSIONS)
                            current="Create permission"
                        />
                    }
                >
                    <CreatePermissionForm />
                </Show>
            </Suspense>
        </ContentContainer>
    }
}

#[component]
fn CreatePermissionForm() -> impl IntoView {
    let navigate = use_navigate();
    let navigate_cancel = navigate.clone();
    let name = RwSignal::new(String::new());
    let key = RwSignal::new(String::new());
    // The key follows the name until someone edits it by hand.
    let key_edited = RwSignal::new(false);
    let description = RwSignal::new(String::new());
    let domain_id = RwSignal::new(String::new());
    let owners_group = RwSignal::new(None::<SearchSourceItem>);
    let owners_options = RwSignal::new(Vec::<SearchSourceItem>::new());
    let owners_seq = RwSignal::new(0u64);
    let error = RwSignal::new(None::<String>);
    let domains = Resource::new(|| (), |()| list_domains(None));

    Effect::new(move |_| {
        let made = key_from_name(&name.get());
        if !key_edited.get_untracked() {
            key.set(made);
        }
    });
    Effect::new(move |prev: Option<String>| {
        let k = key.get();
        if prev.is_some() && k != key_from_name(&name.get_untracked()) {
            key_edited.set(true);
        }
        k
    });

    let search = move |query: Option<String>| {
        let seq = owners_seq.get_untracked().saturating_add(1);
        owners_seq.set(seq);
        spawn_local_scoped(async move {
            let sources: Vec<SearchSourceKey> =
                vec![PermissionSearchSourceId::PermissionGroup.into()];
            if let Ok(rows) = search_principals(sources, query, 10).await {
                if owners_seq.get_untracked() == seq {
                    owners_options.set(rows);
                }
            }
        });
    };

    let on_submit = move |_| {
        if name.get().trim().is_empty() {
            error.set(Some("validation: Give the permission a name".to_string()));
            return;
        }
        if domain_id.get().trim().is_empty() {
            error.set(Some("validation: Pick a domain".to_string()));
            return;
        }
        let navigate = navigate.clone();
        let payload = PermissionCreateInput {
            display_name: name.get().trim().to_string(),
            name: key.get().trim().to_string(),
            description: description.get(),
            owners_group_id: owners_group.get().map(|g| g.id).unwrap_or_default(),
            domain_id: domain_id.get(),
        };
        let confirm = format!("Create {}", payload.display_name);
        let finish_ok = Callback::new(move |new_id: String| {
            navigate(
                &format!("/permission/permissions/{new_id}"),
                NavigateOptions::default(),
            );
        });
        spawn_with_step_up_to(confirm, error, finish_ok, move || {
            create_permission(payload.clone())
        });
    };

    view! {
        <div id="gauge-create-perm-page">
            <Flex vertical=true gap=FlexGap::Medium>
                <Breadcrumb>
                    <BreadcrumbItem>
                        <BreadcrumbButton href=crate::paths::PERMISSIONS.to_string()>"Permissions"</BreadcrumbButton>
                    </BreadcrumbItem>
                    <BreadcrumbDivider />
                    <BreadcrumbItem>
                        <BreadcrumbButton current=true>"Create permission"</BreadcrumbButton>
                    </BreadcrumbItem>
                </Breadcrumb>
                <Title3 tag=uf_product::components::TextTag::H1>"Create permission"</Title3>
                <Card>
                    <CardContent>
                        <div id="gauge-create-perm-form">
                            <Flex vertical=true gap=FlexGap::Medium>
                                <Body1Strong>"New permission"</Body1Strong>
                                <div id="gauge-create-perm-name">
                                    <Field label="Name" required=true>
                                        <Input bind=name />
                                    </Field>
                                </div>
                                <div id="gauge-create-perm-key">
                                    <Field label="Key" required=true>
                                        <Input bind=key />
                                    </Field>
                                    <Caption1>"Code checks this key. It's made from the name and can't change once saved."</Caption1>
                                </div>
                                <Field label="Domain" required=true>
                                    <Select
                                        bind=domain_id
                                        appearance=SelectAppearance {
                                            disabled: Signal::derive(move || !matches!(domains.get(), Some(Ok(ref d)) if !d.is_empty())),
                                            ..Default::default()
                                        }
                                    >
                                        <option value="">"Select a domain"</option>
                                        {move || domains.get().and_then(Result::ok).unwrap_or_default().into_iter().map(|row| view! {
                                            <option value=row.id>{row.name}</option>
                                        }).collect_view()}
                                    </Select>
                                    <Show when=move || matches!(domains.get(), Some(Ok(ref d)) if d.is_empty())>
                                        <Caption1>"No domains yet. Create one first."</Caption1>
                                    </Show>
                                    {move || match domains.get() {
                                        Some(Err(err)) => Some(view! { <ErrorBar error=err.to_string() what="the domains" /> }),
                                        _ => None,
                                    }}
                                </Field>
                                <div id="gauge-create-perm-owners">
                                    <Field label="Owned by">
                                        <SearchSourcePicker
                                            search_sources=Signal::derive(|| vec![PermissionSearchSourceId::PermissionGroup.into()])
                                            options=owners_options
                                            placeholder=Signal::derive(move || default_owner_text(&name.get()))
                                            on_request_initial=Callback::new(move |_| search(None))
                                            on_search=Callback::new(move |(_, q): (Vec<SearchSourceKey>, String)| search(Some(q)))
                                            on_select=Callback::new(move |item: SearchSourceItem| owners_group.set(Some(item)))
                                        />
                                    </Field>
                                    <Caption1 attr:data-testid="gauge-create-perm-owner-default">{move || match owners_group.get() {
                                        Some(group) => format!("Owned by {}. Owners review requests and manage the allow list.", group.title),
                                        None => format!("{}. Owners review requests and manage the allow list. Pick an existing group to share that work.", default_owner_text(&name.get())),
                                    }}</Caption1>
                                </div>
                                <Field label="Description">
                                    <MarkdownEditor value=description aria_label="Description" testid="gauge-create-perm-description" />
                                </Field>
                                <MaybeErrorBar error=error what="the new permission" />
                                <Flex gap=FlexGap::Small>
                                    <div id="gauge-create-perm-submit">
                                        <Button appearance=ButtonAppearance::Primary on_click=Callback::new(on_submit)>
                                            "Create permission"
                                        </Button>
                                    </div>
                                    <div id="gauge-create-perm-cancel">
                                        <Button
                                            appearance=ButtonAppearance::Subtle
                                            on_click=Callback::new(move |_| navigate_cancel(crate::paths::PERMISSIONS, NavigateOptions::default()))
                                        >
                                            "Cancel"
                                        </Button>
                                    </div>
                                </Flex>
                            </Flex>
                        </div>
                    </CardContent>
                </Card>
            </Flex>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_makes_a_pascal_case_key_happy() {
        assert_eq!(key_from_name("Page on-call"), "PageOnCall");
        assert_eq!(key_from_name("approve releases"), "ApproveReleases");
    }

    #[test]
    fn empty_name_makes_empty_key_sad() {
        assert_eq!(key_from_name("  -- "), "");
        assert_eq!(
            default_owner_text(""),
            "A new owners group (you are its owner)"
        );
        assert_eq!(
            default_owner_text("Page on-call"),
            "Owners of Page on-call (new group, you are its owner)"
        );
    }
}
