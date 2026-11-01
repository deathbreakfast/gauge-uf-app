//! Request Access dialog shared by the group and permission detail pages, and the readers for
//! the query keys that let other apps link straight to it.
//!
//! A detail page opened with `?request=1` shows the dialog on arrival; `&for=<handle>`
//! preselects one of the caller's active service users in the "Request for" choice.

use gauge::types::{PermissionRequestCreateInput, PermissionRequestTargetKind};
use leptos::prelude::*;
use leptos::task::spawn_local_scoped;
use leptos_router::params::ParamsMap;
use uf_product::primitives::{
    Button, ButtonAppearance, Dialog, DialogActions, DialogBody, DialogContent, DialogSurface,
    DialogTitle, Field, Flex, FlexGap, MessageBar, MessageBarIntent, Select, Textarea,
};

use crate::server::{create_permission_request, list_my_active_service_users, ServiceUserOption};

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

/// Request Access dialog: a "Request for" choice (yourself or one of your active service
/// users), a reason, and submit. Calls `on_submitted` after the request is created.
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
    /// Runs after a request is created.
    on_submitted: Callback<()>,
) -> impl IntoView {
    let reason = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let request_for = RwSignal::new(String::new());
    let not_yours = RwSignal::new(None::<String>);

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

    Effect::new(move |_| {
        if let Some(Ok(options)) = service_users.get() {
            let (choice, warning) =
                initial_choice(&options, preselect_for.get_untracked().as_deref());
            request_for.set(choice);
            not_yours.set(warning);
        }
    });

    let submit = move |_| {
        let reason_text = reason.get();
        if reason_text.trim().is_empty() {
            error.set(Some("Reason is required.".to_string()));
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
                Ok(_) => {
                    open.set(false);
                    error.set(None);
                    reason.set(String::new());
                    on_submitted.run(());
                }
                Err(err) => error.set(Some(err.to_string())),
            }
        });
    };

    view! {
        <Dialog open=open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>"Request Access"</DialogTitle>
                    <DialogContent>
                        <Flex vertical=true gap=FlexGap::Small>
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
                            <Field label="Reason">
                                <Textarea bind=reason />
                            </Field>
                            <Show when=move || error.get().is_some()>
                                <MessageBar intent=MessageBarIntent::Error>
                                    {move || error.get().unwrap_or_default()}
                                </MessageBar>
                            </Show>
                        </Flex>
                    </DialogContent>
                    <DialogActions>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            on_click=Callback::new(move |_| {
                                open.set(false);
                                error.set(None);
                            })
                        >
                            "Cancel"
                        </Button>
                        <Button appearance=ButtonAppearance::Primary on_click=Callback::new(submit)>
                            "Submit Request"
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

    fn option(handle: &str) -> ServiceUserOption {
        ServiceUserOption {
            handle: handle.to_string(),
            display_name: handle.to_string(),
        }
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
