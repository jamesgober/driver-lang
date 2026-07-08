//! The error that halts a compilation run.

use alloc::borrow::Cow;
use core::fmt;

/// The error that stops a compilation run.
///
/// A [`Stage`](crate::Stage) returns a `DriverError` from
/// [`Stage::run`](crate::Stage::run) when it cannot produce its output: a phase
/// that hit an unrecoverable condition, or a call to
/// [`Session::abort_if_errors`](crate::Session::abort_if_errors) that found the
/// session already holds errors. It is the *control-flow* failure that ends the
/// pipeline, as distinct from a [`Diagnostic`](crate::Diagnostic), which is a
/// *record* a stage emits and may keep going after.
///
/// The error carries the [`message`](Self::message) describing what went wrong and
/// the [`stage`](Self::stage) it came from. A stage does not name itself: it
/// returns `DriverError::new(reason)` with an empty stage, and the
/// [`Pipeline`](crate::Pipeline) stamps in the name of the stage that failed as it
/// propagates the error. Stamping only fills an *empty* name, so in a chain the
/// innermost stage — the one that actually failed — keeps the attribution.
///
/// The reason is stored as a [`Cow<'static, str>`](alloc::borrow::Cow), so a
/// static message costs no allocation and a computed one is owned only when built.
///
/// # Examples
///
/// A stage failing with a static reason:
///
/// ```
/// use driver_lang::DriverError;
///
/// let err = DriverError::new("unresolved import");
/// assert_eq!(err.message(), "unresolved import");
/// assert_eq!(err.stage(), ""); // not stamped until it flows through a Pipeline
/// ```
///
/// A computed reason:
///
/// ```
/// use driver_lang::DriverError;
///
/// let path = "src/main";
/// let err = DriverError::new(format!("cannot read `{path}.rs`"));
/// assert_eq!(err.message(), "cannot read `src/main.rs`");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverError {
    stage: &'static str,
    message: Cow<'static, str>,
}

impl DriverError {
    /// Create an error describing why a stage could not complete.
    ///
    /// Call this from inside [`Stage::run`](crate::Stage::run). The `message`
    /// accepts a string literal (no allocation) or an owned
    /// [`String`](alloc::string::String) (a computed reason). The failing stage's
    /// name is filled in by the [`Pipeline`](crate::Pipeline) — you do not repeat
    /// it here.
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::DriverError;
    ///
    /// let from_literal = DriverError::new("boom");
    /// let from_owned = DriverError::new(String::from("boom"));
    /// assert_eq!(from_literal, from_owned);
    /// ```
    #[must_use]
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            stage: "",
            message: message.into(),
        }
    }

    /// The name of the stage that failed, or `""` if the error has not yet
    /// propagated through a [`Pipeline`](crate::Pipeline).
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::DriverError;
    ///
    /// assert_eq!(DriverError::new("boom").stage(), "");
    /// ```
    #[must_use]
    #[inline]
    pub fn stage(&self) -> &str {
        self.stage
    }

    /// The reason the stage could not complete.
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::DriverError;
    ///
    /// assert_eq!(DriverError::new("type error").message(), "type error");
    /// ```
    #[must_use]
    #[inline]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Stamp the failing stage's name into the error, but only if it is not
    /// already set. Called by the [`Pipeline`](crate::Pipeline) as it propagates a
    /// failure; the first (innermost) stage to stamp wins, so the error names the
    /// stage that actually failed rather than an outer wrapper.
    #[must_use]
    pub(crate) fn in_stage(mut self, name: &'static str) -> Self {
        if self.stage.is_empty() {
            self.stage = name;
        }
        self
    }
}

impl fmt::Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.stage.is_empty() {
            write!(f, "driver error: {}", self.message)
        } else {
            write!(f, "stage `{}` failed: {}", self.stage, self.message)
        }
    }
}

impl core::error::Error for DriverError {}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use alloc::string::{String, ToString};

    #[test]
    fn test_new_accepts_literal_and_owned() {
        assert_eq!(DriverError::new("x"), DriverError::new(String::from("x")));
    }

    #[test]
    fn test_message_is_preserved() {
        assert_eq!(DriverError::new("type error").message(), "type error");
    }

    #[test]
    fn test_stage_is_empty_before_stamping() {
        assert_eq!(DriverError::new("boom").stage(), "");
    }

    #[test]
    fn test_in_stage_stamps_empty_name() {
        let err = DriverError::new("boom").in_stage("parse");
        assert_eq!(err.stage(), "parse");
        assert_eq!(err.message(), "boom");
    }

    #[test]
    fn test_in_stage_does_not_overwrite_existing_name() {
        let err = DriverError::new("boom").in_stage("lex").in_stage("parse");
        assert_eq!(err.stage(), "lex");
    }

    #[test]
    fn test_display_without_stage() {
        assert_eq!(DriverError::new("boom").to_string(), "driver error: boom");
    }

    #[test]
    fn test_display_with_stage() {
        let err = DriverError::new("boom").in_stage("codegen");
        assert_eq!(err.to_string(), "stage `codegen` failed: boom");
    }

    #[test]
    fn test_error_trait_is_implemented() {
        fn assert_error<E: core::error::Error>(_: &E) {}
        assert_error(&DriverError::new("boom"));
    }
}
