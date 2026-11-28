//! A server error in words: a title, one sentence, and what to do next.
//!
//! Server functions fail with prefixed strings (`permission_denied::…`, `STEP_UP:…`,
//! `not_authorized: …`, `validation: …`, or a plain `Failed to …` for a Valence failure).
//! [`describe_error`](crate::components::error_bar::describe_error) sorts them into an
//! [`ErrorCopy`](crate::components::error_bar::ErrorCopy);
//! [`ErrorBar`](crate::components::error_bar::ErrorBar) shows that copy so no
//! page puts a raw `ServerFnError` string in front of people.

use leptos::prelude::*;
use uf_product::primitives::{
    Button, ButtonAppearance, MessageBar, MessageBarActions, MessageBarBody, MessageBarIntent,
    MessageBarLayout, MessageBarTitle,
};

/// What a failure means for the person who hit it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// They don't have the access the action needs.
    Denied,
    /// The server wants an authenticator code first.
    StepUp,
    /// The authenticator code was wrong or already used.
    WrongCode,
    /// Too many wrong codes in a row.
    RateLimited,
    /// They have no authenticator app set up.
    NoAuthenticator,
    /// The session ended.
    SignedOut,
    /// The record is gone.
    NotFound,
    /// Gauge refused the input; the message says why.
    Validation,
    /// Gauge or its store didn't answer.
    Unavailable,
}

/// The words for one failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorCopy {
    /// Which kind of failure it is.
    pub kind: ErrorKind,
    /// Short title.
    pub title: String,
    /// One sentence saying what happened and what to do.
    pub sentence: String,
}

/// Sentence for a wrong authenticator code, shared with the step-up dialog.
pub const WRONG_CODE_SENTENCE: &str =
    "Codes change every 30 seconds and each works once. Type the newest one from your app.";

/// Sentence for a caller with no authenticator app.
pub const NO_AUTHENTICATOR_SENTENCE: &str = "This change needs a code from an authenticator app. Set one up under Account settings, then come back and try again.";

/// Sort a server error string into words. `what` names what was loading or changing
/// ("this permission"), for the Try again case.
#[must_use]
pub fn describe_error(raw: &str, what: &str) -> ErrorCopy {
    let copy = |kind, title: &str, sentence: String| ErrorCopy {
        kind,
        title: title.to_string(),
        sentence,
    };
    if raw.contains("STEP_UP:step_up_invalid") {
        return copy(
            ErrorKind::WrongCode,
            "That code didn't work",
            WRONG_CODE_SENTENCE.to_string(),
        );
    }
    if raw.contains("STEP_UP:step_up_rate_limited") {
        return copy(
            ErrorKind::RateLimited,
            "Too many tries",
            "Wait a few minutes, then type the newest code from your authenticator app."
                .to_string(),
        );
    }
    if raw.contains("STEP_UP:totp_enrollment_required") || raw.contains("STEP_UP:totp_unavailable")
    {
        return copy(
            ErrorKind::NoAuthenticator,
            "Set up an authenticator first",
            NO_AUTHENTICATOR_SENTENCE.to_string(),
        );
    }
    if raw.contains("STEP_UP:auth_required") || raw.contains("Authentication required") {
        return copy(
            ErrorKind::SignedOut,
            "You're signed out",
            "Sign in again, then try once more.".to_string(),
        );
    }
    if raw.contains("STEP_UP:") {
        return copy(
            ErrorKind::StepUp,
            "Confirm it's you",
            "This change needs a code from your authenticator app. Try again and type the code when asked.".to_string(),
        );
    }
    if raw.contains("permission_denied::")
        || raw.contains("not_authorized:")
        || raw.contains("Not authorized")
    {
        return copy(
            ErrorKind::Denied,
            "You don't have access",
            format!("You can't open or change {what}. Ask its owners for access."),
        );
    }
    if raw.contains("not_found:") {
        return copy(
            ErrorKind::NotFound,
            "Not found",
            format!("Gauge couldn't find {what}. It may have been deleted."),
        );
    }
    if let Some(message) = raw.split("validation: ").nth(1) {
        let message = message.trim();
        let sentence = if message.ends_with('.') {
            message.to_string()
        } else {
            format!("{message}.")
        };
        return copy(ErrorKind::Validation, "Check what you entered", sentence);
    }
    copy(
        ErrorKind::Unavailable,
        &format!("Couldn't load {what}"),
        "Gauge didn't answer, so nothing changed. Try again in a moment.".to_string(),
    )
}

/// An error message bar for a failed load or change, worded by [`describe_error`].
///
/// Shows Try again when `on_retry` is set and the failure might pass on a retry, and
/// Ask for access when `on_ask` is set and the caller lacks access.
#[component]
pub fn ErrorBar(
    /// The server error text.
    #[prop(into)]
    error: Signal<String>,
    /// What was loading or changing, for the sentence ("this permission").
    #[prop(into)]
    what: String,
    /// Runs on Try again.
    #[prop(optional)]
    on_retry: Option<Callback<()>>,
    /// Runs on Ask for access.
    #[prop(optional)]
    on_ask: Option<Callback<()>>,
) -> impl IntoView {
    let copy = Memo::new(move |_| describe_error(&error.get(), &what));
    view! {
        <div data-testid="gauge-error-bar">
            <MessageBar intent=MessageBarIntent::Error layout=MessageBarLayout::Multiline>
                <MessageBarBody>
                    <MessageBarTitle>{move || copy.get().title}</MessageBarTitle>
                    {move || copy.get().sentence}
                </MessageBarBody>
                <MessageBarActions>
                    {move || {
                        let kind = copy.get().kind;
                        let retry = on_retry.filter(|_| {
                            matches!(kind, ErrorKind::Unavailable | ErrorKind::StepUp | ErrorKind::WrongCode | ErrorKind::RateLimited)
                        });
                        let ask = on_ask.filter(|_| kind == ErrorKind::Denied);
                        view! {
                            {retry.map(|cb| view! {
                                <Button appearance=ButtonAppearance::Secondary on_click=Callback::new(move |_| cb.run(()))>
                                    "Try again"
                                </Button>
                            })}
                            {ask.map(|cb| view! {
                                <Button appearance=ButtonAppearance::Primary on_click=Callback::new(move |_| cb.run(()))>
                                    "Ask for access"
                                </Button>
                            })}
                        }
                    }}
                </MessageBarActions>
            </MessageBar>
        </div>
    }
}

/// [`ErrorBar`] for an optional error signal: nothing while it's `None`.
#[component]
pub fn MaybeErrorBar(
    /// The latest error, if any.
    error: RwSignal<Option<String>>,
    /// What was changing, for the sentence.
    #[prop(into)]
    what: String,
) -> impl IntoView {
    let what = StoredValue::new(what);
    view! {
        <Show when=move || error.get().is_some()>
            <ErrorBar
                error=Signal::derive(move || error.get().unwrap_or_default())
                what=what.get_value()
            />
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_up_codes_read_as_words_happy() {
        let wrong = describe_error(
            "error running server function: STEP_UP:step_up_invalid: invalid or reused totp code",
            "x",
        );
        assert_eq!(wrong.kind, ErrorKind::WrongCode);
        assert_eq!(wrong.title, "That code didn't work");
        assert_eq!(wrong.sentence, WRONG_CODE_SENTENCE);
        assert_eq!(
            describe_error("STEP_UP:totp_enrollment_required: enroll", "x").kind,
            ErrorKind::NoAuthenticator
        );
        assert_eq!(
            describe_error("STEP_UP:step_up_required: recent", "x").kind,
            ErrorKind::StepUp
        );
    }

    #[test]
    fn denied_and_not_found_name_the_record_happy() {
        let denied = describe_error("permission_denied::GaugeAdmin", "this permission");
        assert_eq!(denied.kind, ErrorKind::Denied);
        assert!(denied.sentence.contains("this permission"));
        let gone = describe_error("not_found: Permission not found: p1", "this permission");
        assert_eq!(gone.kind, ErrorKind::NotFound);
        assert!(!gone.sentence.contains("p1"));
    }

    #[test]
    fn validation_keeps_gauge_message_happy() {
        let v = describe_error(
            "error: validation: Cannot remove the last owner from a group",
            "x",
        );
        assert_eq!(v.kind, ErrorKind::Validation);
        assert_eq!(v.sentence, "Cannot remove the last owner from a group.");
    }

    #[test]
    fn valence_failure_never_shows_raw_text_sad() {
        let raw = "Failed to load permission detail: surreal: connection reset at 10.0.0.4";
        let c = describe_error(raw, "this permission");
        assert_eq!(c.kind, ErrorKind::Unavailable);
        assert_eq!(c.title, "Couldn't load this permission");
        assert!(!c.sentence.contains("surreal"));
    }
}
