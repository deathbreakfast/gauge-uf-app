//! Request Access dialog shared by the group and permission detail pages, and the readers for
//! the query keys that let other apps link straight to it.
//!
//! A detail page opened with `?request=1` shows the dialog on arrival; `&for=<handle>`
//! preselects one of the caller's active service users in the "Request for" choice.

use gauge::types::{
    PermissionRequestCreateInput, PermissionRequestRowDto, PermissionRequestStatusDto,
    PermissionRequestTargetKind,
};
use leptos::prelude::*;
use leptos::task::spawn_local_scoped;
use leptos_router::hooks::use_navigate;
use leptos_router::params::ParamsMap;
use leptos_router::NavigateOptions;
use uf_product::components::{Body1, Caption1, Caption1Strong};
use uf_product::primitives::{
    Button, ButtonAppearance, Dialog, DialogActions, DialogBody, DialogContent, DialogSurface,
    DialogTitle, Field, Flex, FlexGap, FlexWrap, MessageBar, MessageBarBody, MessageBarIntent,
    MessageBarLayout, MessageBarTitle, Select, Textarea,
};

use crate::components::error_bar::MaybeErrorBar;
use crate::pages::shared::words::when_view;
use crate::server::surfaces::{get_request_reviewers, review_target};
use crate::server::{
    create_permission_request, get_group, get_group_access, get_permission,
    list_my_active_service_users, list_my_permission_requests, ServiceUserOption,
};

/// Whether the URL asks for the Request Access dialog to open on arrival (`request=1`).
#[must_use]
pub fn wants_request_dialog(query: &ParamsMap) -> bool {
    query.get_str("request") == Some("1")
}

/// The service user handle named by `for=`, without a leading `@`. `None` when absent or blank.
#[must_use]
pub fn requested_for_handle(query: &ParamsMap) -> Option<String> {
    let handle = query.get_str("for")?.trim().trim_start_matches('@').trim();
    (!handle.is_empty()).then(|| handle.to_string())
}

/// What the "Request for" choice starts on: the `for=` handle when it is one of the caller's
/// active service users, otherwise "Yourself" (empty), plus the handle to warn about.
fn initial_choice(
    service_users: &[ServiceUserOption],
    wanted: Option<&str>,
) -> (String, Option<String>) {
    match wanted {
        Some(handle) => match service_users
            .iter()
            .find(|su| su.handle.eq_ignore_ascii_case(handle))
        {
            Some(su) => (su.handle.clone(), None),
            None => (String::new(), Some(handle.to_string())),
        },
        None => (String::new(), None),
    }
}

/// What the dialog shows about its target: its name, what members get and who reviews it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct TargetFacts {
    name: String,
    members_get: Vec<String>,
    reviewers: String,
}

async fn target_facts(
    kind: PermissionRequestTargetKind,
    id: String,
) -> Result<TargetFacts, ServerFnError> {
    let reviewers = get_request_reviewers(vec![review_target(&kind, &id)])
        .await
        .ok()
        .and_then(|mut lines| lines.pop())
        .unwrap_or_default();
    let (name, members_get) = match kind {
        PermissionRequestTargetKind::Group => {
            let name = get_group(id.clone())
                .await?
                .map(|g| g.name)
                .unwrap_or_default();
            let gives = get_group_access(id)
                .await
                .map(|perms| perms.into_iter().map(|p| p.display_name).collect())
                .unwrap_or_default();
            (name, gives)
        }
        PermissionRequestTargetKind::Permission => {
            let name = get_permission(id)
                .await?
                .map(|p| {
                    if p.display_name.trim().is_empty() {
                        p.name
                    } else {
                        p.display_name
                    }
                })
                .unwrap_or_default();
            (name.clone(), vec![name])
        }
    };
    Ok(TargetFacts {
        name,
        members_get,
        reviewers,
    })
}

/// The dialog's title: "Ask to join Release Captains" or "Ask for Approve releases".
#[must_use]
pub fn ask_title(kind: &PermissionRequestTargetKind, name: &str) -> String {
    match (kind, name.is_empty()) {
        (_, true) => "Request access".to_string(),
        (PermissionRequestTargetKind::Group, false) => format!("Ask to join {name}"),
        (PermissionRequestTargetKind::Permission, false) => format!("Ask for {name}"),
    }
}

/// Request Access dialog: what members get and who reviews, a "Request for" choice (yourself
/// or one of your active service users), a reason, and Send. After Send it says who the
/// request went to; asking again for yourself shows the open request instead. Calls
/// `on_submitted` when the dialog closes after a request was sent.
#[component]
pub fn RequestAccessDialog(
    /// Whether the dialog is showing.
    open: RwSignal<bool>,
    /// Whether the target is a permission or a group.
    target_kind: PermissionRequestTargetKind,
    /// Id of the permission or group being requested.
    #[prop(into)]
    target_id: Signal<String>,
    /// Service user handle to preselect (from `?for=`).
    #[prop(into)]
    preselect_for: Signal<Option<String>>,
    /// Runs after a request is created and the dialog closes.
    on_submitted: Callback<()>,
) -> impl IntoView {
    let reason = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let request_for = RwSignal::new(String::new());
    let not_yours = RwSignal::new(None::<String>);
    // The request this dialog just sent.
    let sent = RwSignal::new(None::<PermissionRequestRowDto>);
    let navigate = use_navigate();

    // Only fetched once the dialog opens, so plain page views don't list service users.
    let service_users = Resource::new(
        move || open.get(),
        |is_open| async move {
            if is_open {
                list_my_active_service_users().await
            } else {
                Ok(Vec::new())
            }
        },
    );
    let kind_for_facts = target_kind.clone();
    let facts = crate::pages::shared::local_resource(
        move || open.get().then(|| target_id.get()),
        move |id| {
            let kind = kind_for_facts.clone();
            async move {
                match id {
                    Some(id) => target_facts(kind, id).await.ok(),
                    None => None,
                }
            }
        },
    );
    // The caller's own open request for this target, if they already asked for themselves.
    let kind_for_open = target_kind.clone();
    let already = crate::pages::shared::local_resource(
        move || open.get().then(|| target_id.get()),
        move |id| {
            let kind = kind_for_open.clone();
            async move {
                let id = id?;
                list_my_permission_requests()
                    .await
                    .ok()?
                    .into_iter()
                    .find(|r| {
                        r.target_kind == kind
                            && r.target_id == id
                            && r.on_behalf_of.is_none()
                            && r.status == PermissionRequestStatusDto::Pending
                    })
            }
        },
    );
    let reviewers = Signal::derive(move || {
        facts
            .get()
            .flatten()
            .map(|f| f.reviewers)
            .filter(|r| !r.is_empty())
            .unwrap_or_else(|| "its owners".to_string())
    });

    Effect::new(move |_| {
        if let Some(Ok(options)) = service_users.get() {
            let (choice, warning) =
                initial_choice(&options, preselect_for.get_untracked().as_deref());
            request_for.set(choice);
            not_yours.set(warning);
        }
    });

    let title_kind = StoredValue::new(target_kind.clone());
    let close = Callback::new(move |()| {
        open.set(false);
        error.set(None);
        if sent.get_untracked().is_some() {
            sent.set(None);
            on_submitted.run(());
        }
    });

    let submit = move |_| {
        let reason_text = reason.get();
        if reason_text.trim().is_empty() {
            error.set(Some("validation: Say why you need it.".to_string()));
            return;
        }
        let handle = request_for.get();
        let input = PermissionRequestCreateInput {
            target_kind: target_kind.clone(),
            target_id: target_id.get(),
            reason: reason_text,
            on_behalf_of: (!handle.is_empty()).then_some(handle),
        };
        spawn_local_scoped(async move {
            match create_permission_request(input).await {
                Ok(row) => {
                    error.set(None);
                    reason.set(String::new());
                    sent.set(Some(row));
                }
                Err(err) => error.set(Some(err.to_string())),
            }
        });
    };
    let open_request = {
        let navigate = navigate.clone();
        Callback::new(move |id: String| {
            open.set(false);
            sent.set(None);
            navigate(
                &format!("/permission/requests/{id}"),
                NavigateOptions::default(),
            );
        })
    };

    view! {
        <Dialog open=open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>
                        {move || ask_title(&title_kind.get_value(), &facts.get().flatten().map(|f| f.name).unwrap_or_default())}
                    </DialogTitle>
                    <DialogContent>
                        {move || {
                            if let Some(row) = sent.get() {
                                let target = facts.get().flatten().map(|f| f.name).unwrap_or_default();
                                return view! {
                                    <div data-testid="gauge-request-sent">
                                        <MessageBar intent=MessageBarIntent::Success layout=MessageBarLayout::Multiline>
                                            <MessageBarBody>
                                                <MessageBarTitle>{format!("Request sent to {}", reviewers.get())}</MessageBarTitle>
                                                {format!("They review requests for {target}. You'll get a notification when it's answered, and you can follow it under Requests.")}
                                            </MessageBarBody>
                                        </MessageBar>
                                        <span hidden=true data-testid="gauge-request-sent-id">{row.id}</span>
                                    </div>
                                }.into_any();
                            }
                            if let Some(row) = already.get().flatten() {
                                return view! {
                                    <div data-testid="gauge-request-already">
                                        <MessageBar intent=MessageBarIntent::Info layout=MessageBarLayout::Multiline>
                                            <MessageBarBody>
                                                <MessageBarTitle>"You already asked"</MessageBarTitle>
                                                "Your request from " {when_view(row.created_at.clone())} {format!(" is waiting for {}.", reviewers.get())}
                                            </MessageBarBody>
                                        </MessageBar>
                                    </div>
                                }.into_any();
                            }
                            view! {
                                <Flex vertical=true gap=FlexGap::Medium>
                                    <div data-testid="gauge-request-facts">
                                        <Flex gap=FlexGap::Large wrap=FlexWrap::Wrap>
                                            <Flex vertical=true gap=FlexGap::Size(0)>
                                                <Caption1Strong>"Members get"</Caption1Strong>
                                                <Body1>{move || match facts.get() {
                                                    Some(Some(f)) if !f.members_get.is_empty() => f.members_get.join(", "),
                                                    Some(Some(_)) => "No permissions yet".to_string(),
                                                    _ => String::new(),
                                                }}</Body1>
                                            </Flex>
                                            <Flex vertical=true gap=FlexGap::Size(0)>
                                                <Caption1Strong>"Reviewed by"</Caption1Strong>
                                                <Body1>{move || reviewers.get()}</Body1>
                                            </Flex>
                                        </Flex>
                                    </div>
                                    <Transition fallback=|| ()>
                                        {move || service_users.get().map(|result| {
                                            let options = result.unwrap_or_default();
                                            view! {
                                                <div id="gauge-request-for">
                                                <Field label="Request for">
                                                    <Select bind=request_for>
                                                        <option value="">"Yourself"</option>
                                                        {options.into_iter().map(|su| {
                                                            let label = format!("@{} ({})", su.handle, su.display_name);
                                                            view! { <option value=su.handle>{label}</option> }
                                                        }).collect_view()}
                                                    </Select>
                                                </Field>
                                                </div>
                                            }
                                        })}
                                    </Transition>
                                    <Show when=move || not_yours.get().is_some()>
                                        <MessageBar intent=MessageBarIntent::Warning>
                                            {move || format!(
                                                "@{} isn't one of your active service users.",
                                                not_yours.get().unwrap_or_default()
                                            )}
                                        </MessageBar>
                                    </Show>
                                    <Field label="Why do you need it?">
                                        <Textarea bind=reason />
                                    </Field>
                                    <Caption1>{move || format!("{} sees your name and this reason.", reviewers.get())}</Caption1>
                                    <MaybeErrorBar error=error what="your request" />
                                </Flex>
                            }.into_any()
                        }}
                    </DialogContent>
                    <DialogActions>
                        {move || {
                            let pending_id = sent.get().map(|r| r.id).or_else(|| already.get().flatten().map(|r| r.id));
                            match pending_id {
                                Some(id) => view! {
                                    <Button appearance=ButtonAppearance::Secondary on_click=Callback::new(move |_| open_request.run(id.clone()))>
                                        "Open request"
                                    </Button>
                                    <Button appearance=ButtonAppearance::Primary on_click=Callback::new(move |_| close.run(()))>
                                        {move || if sent.get().is_some() { "Done" } else { "Close" }}
                                    </Button>
                                }.into_any(),
                                None => view! {
                                    <Button appearance=ButtonAppearance::Secondary on_click=Callback::new(move |_| close.run(()))>
                                        "Cancel"
                                    </Button>
                                    <Button appearance=ButtonAppearance::Primary on_click=Callback::new(submit.clone())>
                                        "Send request"
                                    </Button>
                                }.into_any(),
                            }
                        }}
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
        </Dialog>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn option(handle: &str) -> ServiceUserOption {
        ServiceUserOption {
            handle: handle.to_string(),
            display_name: handle.to_string(),
        }
    }

    #[test]
    fn titles_name_the_target_happy() {
        assert_eq!(
            ask_title(&PermissionRequestTargetKind::Group, "Release Captains"),
            "Ask to join Release Captains"
        );
        assert_eq!(
            ask_title(&PermissionRequestTargetKind::Permission, "Approve releases"),
            "Ask for Approve releases"
        );
        assert_eq!(
            ask_title(&PermissionRequestTargetKind::Group, ""),
            "Request access"
        );
    }

    #[test]
    fn query_helpers_read_request_and_for() {
        let mut query = ParamsMap::new();
        assert!(!wants_request_dialog(&query));
        assert_eq!(requested_for_handle(&query), None);

        query.insert("request", "1".to_string());
        query.insert("for", "@spectra-agent".to_string());
        assert!(wants_request_dialog(&query));
        assert_eq!(
            requested_for_handle(&query),
            Some("spectra-agent".to_string())
        );
    }

    #[test]
    fn initial_choice_prefers_owned_handle_and_flags_others() {
        let mine = [option("spectra-agent")];
        assert_eq!(initial_choice(&mine, None), (String::new(), None));
        assert_eq!(
            initial_choice(&mine, Some("Spectra-Agent")),
            ("spectra-agent".to_string(), None)
        );
        assert_eq!(
            initial_choice(&mine, Some("someone-else")),
            (String::new(), Some("someone-else".to_string()))
        );
    }
}
