//! Retry Tier A Gauge mutations after a TOTP sudo window (or collect a fresh code).
//!
//! The code dialog says what it confirms ("…to confirm: Add Noor Haddad to Release
//! Captains."), and a wrong code reads "That code didn't work" with how to get a fresh one.

use lepton_auth::actions::step_up::verify_step_up_totp;
use lepton_auth_ui::{
    use_step_up_controller, StepUpController, StepUpFactors, StepUpPolicy, StepUpRequest,
};
use leptos::prelude::*;
use leptos::task::spawn_local_scoped;

use crate::components::error_bar::{describe_error, ErrorKind};

/// True when the server asked for (or expired) a TOTP sudo window.
#[must_use]
pub fn is_step_up_challenge(err: &impl std::fmt::Display) -> bool {
    let s = err.to_string();
    s.contains("STEP_UP:step_up_required") || s.contains("STEP_UP:step_up_expired")
}

/// The code dialog's sentence for `action` ("Add Noor Haddad to Release Captains").
#[must_use]
pub fn confirm_sentence(action: &str) -> String {
    let action = action.trim().trim_end_matches('.');
    if action.is_empty() {
        "Type the code from your authenticator app to continue.".to_string()
    } else {
        format!("Type the code from your authenticator app to confirm: {action}.")
    }
}

/// What the dialog shows for a failed code or retry: words, never the raw error.
#[must_use]
pub fn dialog_error(raw: &str) -> String {
    let copy = describe_error(raw, "this change");
    match copy.kind {
        ErrorKind::WrongCode | ErrorKind::RateLimited | ErrorKind::NoAuthenticator => {
            format!("{}. {}", copy.title, copy.sentence)
        }
        _ => copy.sentence,
    }
}

fn window_request(action: &str) -> StepUpRequest {
    StepUpRequest {
        title: "Confirm it's you".to_string(),
        description: Some(confirm_sentence(action)),
        policy: StepUpPolicy::Totp,
    }
}

fn fresh_request(action: &str) -> StepUpRequest {
    StepUpRequest {
        title: "Confirm it's you".to_string(),
        description: Some(format!(
            "{} Super User changes need a fresh code, even if you typed one a minute ago.",
            confirm_sentence(action)
        )),
        policy: StepUpPolicy::Totp,
    }
}

/// Run `action`; on a window challenge, open step-up, verify, then retry once.
pub fn spawn_with_step_up<T, F, Fut>(error: RwSignal<Option<String>>, on_ok: Callback<T>, action: F)
where
    T: 'static,
    F: Fn() -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
{
    spawn_with_step_up_to(String::new(), error, on_ok, action);
}

/// [`spawn_with_step_up`] whose code dialog names what it confirms (`confirm`, such as
/// "Remove Ravi Menon from Release Captains").
pub fn spawn_with_step_up_to<T, F, Fut>(
    confirm: impl Into<String>,
    error: RwSignal<Option<String>>,
    on_ok: Callback<T>,
    action: F,
) where
    T: 'static,
    F: Fn() -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
{
    let confirm = confirm.into();
    let step_up = use_step_up_controller();
    spawn_local_scoped(async move {
        match action().await {
            Ok(value) => on_ok.run(value),
            Err(err) if is_step_up_challenge(&err) => {
                let Some(ctrl) = step_up else {
                    error.set(Some(err.to_string()));
                    return;
                };
                request_window_then_retry(ctrl, &confirm, error, on_ok, action);
            }
            Err(err) => error.set(Some(err.to_string())),
        }
    });
}

fn request_window_then_retry<T, F, Fut>(
    ctrl: StepUpController,
    confirm: &str,
    error: RwSignal<Option<String>>,
    on_ok: Callback<T>,
    action: F,
) where
    T: 'static,
    F: Fn() -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
{
    ctrl.request(
        window_request(confirm),
        Callback::new(move |factors: StepUpFactors| {
            let action = action.clone();
            spawn_local_scoped(async move {
                if let Err(e) = verify_step_up_totp(factors.totp_code).await {
                    ctrl.report_error(dialog_error(&e.to_string()));
                    return;
                }
                match action().await {
                    Ok(value) => {
                        ctrl.complete_success();
                        on_ok.run(value);
                    }
                    Err(e) => {
                        if is_step_up_challenge(&e) {
                            ctrl.report_error(dialog_error(&e.to_string()));
                        } else {
                            ctrl.complete_success();
                            error.set(Some(e.to_string()));
                        }
                    }
                }
            });
        }),
    );
}

/// Collect a fresh TOTP code, then run `action(code)` (Super User changes).
pub fn spawn_with_fresh_totp<T, F, Fut>(
    error: RwSignal<Option<String>>,
    on_ok: Callback<T>,
    action: F,
) where
    T: 'static,
    F: Fn(String) -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
{
    spawn_with_fresh_totp_to(String::new(), error, on_ok, action);
}

/// [`spawn_with_fresh_totp`] whose code dialog names what it confirms.
pub fn spawn_with_fresh_totp_to<T, F, Fut>(
    confirm: impl Into<String>,
    error: RwSignal<Option<String>>,
    on_ok: Callback<T>,
    action: F,
) where
    T: 'static,
    F: Fn(String) -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
{
    let Some(ctrl) = use_step_up_controller() else {
        error.set(Some(
            "STEP_UP:step_up_required: step-up UI is not mounted".to_string(),
        ));
        return;
    };
    let confirm = confirm.into();
    ctrl.request(
        fresh_request(&confirm),
        Callback::new(move |factors: StepUpFactors| {
            let action = action.clone();
            spawn_local_scoped(async move {
                match action(factors.totp_code).await {
                    Ok(value) => {
                        ctrl.complete_success();
                        on_ok.run(value);
                    }
                    Err(e) => ctrl.report_error(dialog_error(&e.to_string())),
                }
            });
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_names_the_action_happy() {
        assert_eq!(
            confirm_sentence("Add Noor Haddad to Release Captains"),
            "Type the code from your authenticator app to confirm: Add Noor Haddad to Release Captains."
        );
        assert_eq!(
            confirm_sentence(""),
            "Type the code from your authenticator app to continue."
        );
    }

    #[test]
    fn wrong_code_reads_as_a_sentence_sad() {
        let text = dialog_error("STEP_UP:step_up_invalid: invalid or reused totp code");
        assert!(text.starts_with("That code didn't work."));
        assert!(text.contains("Codes change every 30 seconds"));
        assert!(!text.contains("STEP_UP"));
    }
}
