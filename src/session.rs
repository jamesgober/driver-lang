//! The ambient state of one compilation run.

use alloc::borrow::Cow;
use alloc::vec::Vec;

use crate::diagnostic::Diagnostic;
use crate::error::DriverError;

/// The ambient state threaded through one compilation run.
///
/// Every [`Stage`](crate::Stage) in a pipeline receives `&mut Session<C>`. The
/// session is where the shared, cross-stage state lives: the compilation
/// *configuration* (`C` — target, options, input paths, whatever a language
/// needs), and the *diagnostics* every stage emits as it works. It is the one
/// piece of state a driver carries from the first phase to the last, so a later
/// stage can read what an earlier one recorded.
///
/// The session is generic over the configuration type `C` and never inspects it —
/// it hands out `&C` / `&mut C` and otherwise leaves it alone. This keeps the
/// crate free of any opinion about *what* a language's configuration looks like.
///
/// Diagnostics are stored in emission order, and the count of
/// [`error`](crate::Severity::Error)-severity diagnostics is maintained
/// incrementally, so [`error_count`](Self::error_count) and
/// [`has_errors`](Self::has_errors) are O(1) — a driver can check them between
/// every phase without re-scanning the list.
///
/// # Examples
///
/// ```
/// use driver_lang::{Diagnostic, Session};
///
/// // The configuration is whatever a language needs; here, a target triple.
/// let mut session = Session::new("x86_64-unknown-linux-gnu");
///
/// session.warn("unused import `std::mem`");
/// session.error("cannot find type `Foo`");
///
/// assert_eq!(session.config(), &"x86_64-unknown-linux-gnu");
/// assert_eq!(session.error_count(), 1);   // the warning does not count
/// assert_eq!(session.diagnostics().len(), 2);
/// assert!(session.has_errors());
/// ```
#[derive(Debug, Clone)]
pub struct Session<C> {
    config: C,
    diagnostics: Vec<Diagnostic>,
    error_count: usize,
}

impl<C> Session<C> {
    /// Create a session over a configuration, with no diagnostics recorded yet.
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::Session;
    ///
    /// let session = Session::new(());
    /// assert!(!session.has_errors());
    /// assert!(session.diagnostics().is_empty());
    /// ```
    #[must_use]
    pub fn new(config: C) -> Self {
        Self {
            config,
            diagnostics: Vec::new(),
            error_count: 0,
        }
    }

    /// Borrow the compilation configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::Session;
    ///
    /// let session = Session::new(42u32);
    /// assert_eq!(*session.config(), 42);
    /// ```
    #[must_use]
    #[inline]
    pub fn config(&self) -> &C {
        &self.config
    }

    /// Mutably borrow the compilation configuration, so a stage can update options
    /// that later stages read.
    #[must_use]
    #[inline]
    pub fn config_mut(&mut self) -> &mut C {
        &mut self.config
    }

    /// Consume the session and return the configuration, discarding the recorded
    /// diagnostics. Use [`take_diagnostics`](Self::take_diagnostics) first if you
    /// need to keep them.
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::Session;
    ///
    /// let session = Session::new(String::from("opts"));
    /// let config = session.into_config();
    /// assert_eq!(config, "opts");
    /// ```
    #[must_use]
    pub fn into_config(self) -> C {
        self.config
    }

    /// Record a [`Diagnostic`], updating the error count if it is an error.
    ///
    /// Returns `&mut Self` so emissions can be chained. This is the one path by
    /// which diagnostics enter the session; the [`error`](Self::error),
    /// [`warn`](Self::warn), and [`note`](Self::note) shorthands all route through
    /// it.
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::{Diagnostic, Session};
    ///
    /// let mut session = Session::new(());
    /// session
    ///     .emit(Diagnostic::error("first"))
    ///     .emit(Diagnostic::warning("second"));
    ///
    /// assert_eq!(session.diagnostics().len(), 2);
    /// assert_eq!(session.error_count(), 1);
    /// ```
    pub fn emit(&mut self, diagnostic: Diagnostic) -> &mut Self {
        if diagnostic.is_error() {
            // Saturating: a run that somehow emits usize::MAX errors should report
            // a huge count, not wrap to zero and look successful.
            self.error_count = self.error_count.saturating_add(1);
        }
        self.diagnostics.push(diagnostic);
        self
    }

    /// Emit an [`error`](crate::Severity::Error)-severity diagnostic.
    ///
    /// Shorthand for `self.emit(Diagnostic::error(message))`.
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::Session;
    ///
    /// let mut session = Session::new(());
    /// session.error("cannot find value `x`");
    /// assert!(session.has_errors());
    /// ```
    pub fn error(&mut self, message: impl Into<Cow<'static, str>>) -> &mut Self {
        self.emit(Diagnostic::error(message))
    }

    /// Emit a [`warning`](crate::Severity::Warning)-severity diagnostic.
    ///
    /// Shorthand for `self.emit(Diagnostic::warning(message))`.
    pub fn warn(&mut self, message: impl Into<Cow<'static, str>>) -> &mut Self {
        self.emit(Diagnostic::warning(message))
    }

    /// Emit a [`note`](crate::Severity::Note)-severity diagnostic.
    ///
    /// Shorthand for `self.emit(Diagnostic::note(message))`.
    pub fn note(&mut self, message: impl Into<Cow<'static, str>>) -> &mut Self {
        self.emit(Diagnostic::note(message))
    }

    /// All diagnostics recorded so far, in emission order.
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::Session;
    ///
    /// let mut session = Session::new(());
    /// session.note("a").warn("b");
    /// let messages: Vec<_> = session.diagnostics().iter().map(|d| d.message()).collect();
    /// assert_eq!(messages, ["a", "b"]);
    /// ```
    #[must_use]
    #[inline]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// How many [`error`](crate::Severity::Error)-severity diagnostics have been
    /// emitted. Maintained incrementally, so this is O(1).
    #[must_use]
    #[inline]
    pub fn error_count(&self) -> usize {
        self.error_count
    }

    /// Whether any error has been emitted. O(1).
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::Session;
    ///
    /// let mut session = Session::new(());
    /// assert!(!session.has_errors());
    /// session.warn("just a warning");
    /// assert!(!session.has_errors());
    /// session.error("a real error");
    /// assert!(session.has_errors());
    /// ```
    #[must_use]
    #[inline]
    pub fn has_errors(&self) -> bool {
        self.error_count != 0
    }

    /// Stop the run if any error has been emitted.
    ///
    /// This is the driver's checkpoint primitive: emit diagnostics through a phase,
    /// then call `abort_if_errors` to turn accumulated errors into the
    /// [`DriverError`] that ends the pipeline — the "aborting due to N previous
    /// errors" step at the end of a compiler phase. Because a stage can emit
    /// several errors before this is checked, one checkpoint can report many
    /// diagnostics at once instead of stopping at the first.
    ///
    /// Returns `Ok(())` when the session is clean, so `session.abort_if_errors()?`
    /// inside [`Stage::run`](crate::Stage::run) is a no-op on a healthy run and a
    /// clean stop otherwise. The returned error has no stage name yet; the
    /// [`Pipeline`](crate::Pipeline) stamps in the stage that made the call.
    ///
    /// # Errors
    ///
    /// Returns a [`DriverError`] reporting the error count when
    /// [`has_errors`](Self::has_errors) is `true`.
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::Session;
    ///
    /// let mut session = Session::new(());
    /// assert!(session.abort_if_errors().is_ok());
    ///
    /// session.error("first");
    /// session.error("second");
    /// let err = session.abort_if_errors().unwrap_err();
    /// assert_eq!(err.message(), "aborting due to 2 previous errors");
    /// ```
    pub fn abort_if_errors(&self) -> Result<(), DriverError> {
        if self.error_count == 0 {
            return Ok(());
        }
        let noun = if self.error_count == 1 {
            "error"
        } else {
            "errors"
        };
        Err(DriverError::new(alloc::format!(
            "aborting due to {} previous {noun}",
            self.error_count
        )))
    }

    /// Remove and return all recorded diagnostics, resetting the error count to
    /// zero. The configuration is left untouched.
    ///
    /// Use this to drain diagnostics for rendering between runs, or to hand them
    /// off before [`into_config`](Self::into_config) discards the session.
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::Session;
    ///
    /// let mut session = Session::new(());
    /// session.error("boom");
    /// let drained = session.take_diagnostics();
    ///
    /// assert_eq!(drained.len(), 1);
    /// assert!(!session.has_errors());          // count reset
    /// assert!(session.diagnostics().is_empty());
    /// ```
    #[must_use]
    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        self.error_count = 0;
        core::mem::take(&mut self.diagnostics)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::severity::Severity;
    use alloc::vec::Vec;

    #[test]
    fn test_new_session_is_clean() {
        let session = Session::new(());
        assert!(!session.has_errors());
        assert_eq!(session.error_count(), 0);
        assert!(session.diagnostics().is_empty());
    }

    #[test]
    fn test_config_accessors() {
        let mut session = Session::new(1u32);
        assert_eq!(*session.config(), 1);
        *session.config_mut() = 2;
        assert_eq!(*session.config(), 2);
        assert_eq!(session.into_config(), 2);
    }

    #[test]
    fn test_emit_counts_only_errors() {
        let mut session = Session::new(());
        session
            .emit(Diagnostic::error("e"))
            .emit(Diagnostic::warning("w"))
            .emit(Diagnostic::note("n"));
        assert_eq!(session.diagnostics().len(), 3);
        assert_eq!(session.error_count(), 1);
    }

    #[test]
    fn test_shorthands_set_expected_severity() {
        let mut session = Session::new(());
        session.error("e").warn("w").note("n");
        let severities: Vec<_> = session.diagnostics().iter().map(|d| d.severity()).collect();
        assert_eq!(
            severities,
            [Severity::Error, Severity::Warning, Severity::Note]
        );
    }

    #[test]
    fn test_diagnostics_preserve_emission_order() {
        let mut session = Session::new(());
        session.note("a").error("b").warn("c");
        let messages: Vec<_> = session.diagnostics().iter().map(|d| d.message()).collect();
        assert_eq!(messages, ["a", "b", "c"]);
    }

    #[test]
    fn test_abort_if_errors_ok_when_clean() {
        let mut session = Session::new(());
        session.warn("only a warning");
        assert!(session.abort_if_errors().is_ok());
    }

    #[test]
    fn test_abort_if_errors_singular_message() {
        let mut session = Session::new(());
        session.error("one");
        let err = session.abort_if_errors().unwrap_err();
        assert_eq!(err.message(), "aborting due to 1 previous error");
    }

    #[test]
    fn test_abort_if_errors_plural_message() {
        let mut session = Session::new(());
        session.error("one").error("two").error("three");
        let err = session.abort_if_errors().unwrap_err();
        assert_eq!(err.message(), "aborting due to 3 previous errors");
    }

    #[test]
    fn test_take_diagnostics_drains_and_resets() {
        let mut session = Session::new(());
        session.error("a").warn("b");
        let drained = session.take_diagnostics();
        assert_eq!(drained.len(), 2);
        assert!(!session.has_errors());
        assert_eq!(session.error_count(), 0);
        assert!(session.diagnostics().is_empty());
    }
}
