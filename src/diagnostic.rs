//! A single message a stage reports about the program it is compiling.

use alloc::borrow::Cow;
use core::fmt;

use crate::severity::Severity;

/// One message a stage reports about the program under compilation.
///
/// A diagnostic pairs a [`Severity`] with a human-readable message. Stages create
/// diagnostics and hand them to the [`Session`](crate::Session), which stores them
/// in emission order and keeps a running count of how many were errors. The driver
/// owns *policy* — whether to keep going after an error, when to stop — while the
/// diagnostic is just the *record* of what happened.
///
/// The message is a [`Cow<'static, str>`](alloc::borrow::Cow): a fixed message
/// (`"unexpected end of input"`) is stored as a borrowed `&'static str` with no
/// allocation, while a computed one (`format!("unknown name `{name}`")`) is owned
/// only when it is actually built. This keeps the common "constant message" path
/// allocation-free.
///
/// This type is deliberately small — a severity and a message, nothing else.
/// driver-lang does not model spans, error codes, or suggestions: those belong to
/// a dedicated diagnostics crate that a language can plug in as its own concern.
/// The driver only needs enough to route messages and decide when to abort.
///
/// # Examples
///
/// ```
/// use driver_lang::{Diagnostic, Severity};
///
/// let d = Diagnostic::error("unexpected `}`");
/// assert_eq!(d.severity(), Severity::Error);
/// assert_eq!(d.message(), "unexpected `}`");
/// assert!(d.is_error());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Diagnostic {
    severity: Severity,
    message: Cow<'static, str>,
}

impl Diagnostic {
    /// Create a diagnostic with an explicit [`Severity`].
    ///
    /// The `message` accepts a string literal (stored without allocation) or an
    /// owned [`String`](alloc::string::String) (a computed message). Prefer the
    /// [`error`](Self::error), [`warning`](Self::warning), and [`note`](Self::note)
    /// shorthands when the severity is known at the call site.
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::{Diagnostic, Severity};
    ///
    /// let from_literal = Diagnostic::new(Severity::Note, "defined here");
    /// let from_owned = Diagnostic::new(Severity::Note, String::from("defined here"));
    /// assert_eq!(from_literal, from_owned);
    /// ```
    #[must_use]
    pub fn new(severity: Severity, message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            severity,
            message: message.into(),
        }
    }

    /// Create an [`Error`](Severity::Error) diagnostic — a hard failure that
    /// counts toward the session's error total.
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::Diagnostic;
    ///
    /// let d = Diagnostic::error("type mismatch");
    /// assert!(d.is_error());
    /// ```
    #[must_use]
    pub fn error(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(Severity::Error, message)
    }

    /// Create a [`Warning`](Severity::Warning) diagnostic — recorded and shown,
    /// but it does not by itself stop the build.
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::Diagnostic;
    ///
    /// let d = Diagnostic::warning("unused variable `x`");
    /// assert!(!d.is_error());
    /// ```
    #[must_use]
    pub fn warning(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(Severity::Warning, message)
    }

    /// Create a [`Note`](Severity::Note) diagnostic — neutral context attached to
    /// the output.
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::Diagnostic;
    ///
    /// let d = Diagnostic::note("`main` is defined here");
    /// assert!(!d.is_error());
    /// ```
    #[must_use]
    pub fn note(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(Severity::Note, message)
    }

    /// The [`Severity`] of this diagnostic.
    #[must_use]
    #[inline]
    pub fn severity(&self) -> Severity {
        self.severity
    }

    /// The message text.
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::Diagnostic;
    ///
    /// assert_eq!(Diagnostic::error("boom").message(), "boom");
    /// ```
    #[must_use]
    #[inline]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Whether this diagnostic's severity is [`Error`](Severity::Error).
    ///
    /// A shorthand for `self.severity().is_error()`, the same test the
    /// [`Session`](crate::Session) uses to decide whether a diagnostic counts
    /// toward its error total.
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::Diagnostic;
    ///
    /// assert!(Diagnostic::error("boom").is_error());
    /// assert!(!Diagnostic::warning("hmm").is_error());
    /// ```
    #[must_use]
    #[inline]
    pub fn is_error(&self) -> bool {
        self.severity.is_error()
    }
}

impl fmt::Display for Diagnostic {
    /// Renders as `severity: message`, e.g. `error: unexpected `}``.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.severity, self.message)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use alloc::string::{String, ToString};

    #[test]
    fn test_new_accepts_literal_and_owned() {
        let a = Diagnostic::new(Severity::Error, "x");
        let b = Diagnostic::new(Severity::Error, String::from("x"));
        assert_eq!(a, b);
    }

    #[test]
    fn test_error_constructor_sets_error_severity() {
        let d = Diagnostic::error("boom");
        assert_eq!(d.severity(), Severity::Error);
        assert!(d.is_error());
    }

    #[test]
    fn test_warning_constructor_is_not_error() {
        let d = Diagnostic::warning("hmm");
        assert_eq!(d.severity(), Severity::Warning);
        assert!(!d.is_error());
    }

    #[test]
    fn test_note_constructor_is_not_error() {
        let d = Diagnostic::note("fyi");
        assert_eq!(d.severity(), Severity::Note);
        assert!(!d.is_error());
    }

    #[test]
    fn test_message_is_preserved() {
        assert_eq!(
            Diagnostic::error("unexpected token").message(),
            "unexpected token"
        );
    }

    #[test]
    fn test_display_renders_severity_and_message() {
        assert_eq!(Diagnostic::error("boom").to_string(), "error: boom");
        assert_eq!(Diagnostic::note("here").to_string(), "note: here");
    }
}
