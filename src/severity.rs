//! How serious a [`Diagnostic`](crate::Diagnostic) is.

use core::fmt;

/// The seriousness of a [`Diagnostic`](crate::Diagnostic).
///
/// A driver runs a program through a chain of stages, and each stage reports
/// what it found by emitting diagnostics into the [`Session`](crate::Session).
/// The severity is what tells the driver — and the person reading the output —
/// whether a diagnostic is a hard failure that should stop compilation, a
/// heads-up that something looks wrong, or a neutral remark that adds context.
///
/// Only [`Error`](Severity::Error) counts toward
/// [`Session::error_count`](crate::Session::error_count); a warning or a note is
/// recorded and rendered but never trips
/// [`Session::abort_if_errors`](crate::Session::abort_if_errors). Ordering the
/// variants by seriousness — error first — mirrors that: `Error < Warning < Note`
/// is *not* the intent, so the type intentionally does not derive `Ord`. Compare
/// with [`is_error`](Severity::is_error) instead of relying on a numeric rank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Severity {
    /// A hard failure. Counts toward the session's error total and makes
    /// [`Session::abort_if_errors`](crate::Session::abort_if_errors) stop the
    /// build.
    Error,
    /// Something suspicious that does not by itself stop compilation.
    Warning,
    /// Neutral context attached to the output — a hint, a location, a reminder.
    Note,
}

impl Severity {
    /// Whether this is [`Severity::Error`] — the only severity that counts as a
    /// build failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::Severity;
    ///
    /// assert!(Severity::Error.is_error());
    /// assert!(!Severity::Warning.is_error());
    /// assert!(!Severity::Note.is_error());
    /// ```
    #[must_use]
    #[inline]
    pub fn is_error(self) -> bool {
        matches!(self, Severity::Error)
    }

    /// The lowercase label used when a diagnostic is rendered
    /// (`"error"`, `"warning"`, `"note"`).
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::Severity;
    ///
    /// assert_eq!(Severity::Error.as_str(), "error");
    /// assert_eq!(Severity::Warning.as_str(), "warning");
    /// assert_eq!(Severity::Note.as_str(), "note");
    /// ```
    #[must_use]
    #[inline]
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn test_is_error_only_true_for_error() {
        assert!(Severity::Error.is_error());
        assert!(!Severity::Warning.is_error());
        assert!(!Severity::Note.is_error());
    }

    #[test]
    fn test_as_str_matches_variant() {
        assert_eq!(Severity::Error.as_str(), "error");
        assert_eq!(Severity::Warning.as_str(), "warning");
        assert_eq!(Severity::Note.as_str(), "note");
    }

    #[test]
    fn test_display_matches_as_str() {
        assert_eq!(Severity::Warning.to_string(), "warning");
    }
}
