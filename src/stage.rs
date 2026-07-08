//! The compilation-phase trait — the seam a driver is built from.

use crate::error::DriverError;
use crate::session::Session;

/// One phase of compilation: a transform from an input artifact to an output
/// artifact, run against a [`Session`].
///
/// A stage is the unit a driver is assembled from. Each phase of a compiler — lex,
/// parse, resolve names, type-check, lower, emit — is a `Stage` that takes the
/// previous phase's artifact as [`Input`](Self::Input) and produces the next one
/// as [`Output`](Self::Output): a lexer is `Stage<Input = Source, Output =
/// Tokens>`, a parser is `Stage<Input = Tokens, Output = Ast>`, and so on. Unlike
/// a `pass` (which rewrites one type in place), a stage *changes the type* as the
/// program moves down the pipeline — that type-threading is exactly what a driver
/// exists to manage.
///
/// A stage is generic over the session's configuration type `C`, so every stage in
/// a pipeline reads and writes the same shared configuration and diagnostics. Wire
/// stages together with [`Pipeline`](crate::Pipeline), which enforces at compile
/// time that each stage's `Output` is the next stage's `Input`.
///
/// # Contract
///
/// - [`name`](Self::name) returns a stable, static identifier, used to attribute a
///   [`DriverError`] to the stage that produced it. It must not change between
///   runs.
/// - [`run`](Self::run) consumes `input`, may read and write the [`Session`]
///   (emitting diagnostics, reading configuration), and returns the output. A
///   phase that cannot produce its output returns a [`DriverError`] — never a
///   panic. Emitting an error *diagnostic* records a problem but does not by itself
///   stop the pipeline; return `Err`, or call
///   [`Session::abort_if_errors`](crate::Session::abort_if_errors), to stop.
///
/// # Examples
///
/// A stage that parses whitespace-separated integers, warning on each token it
/// cannot read and failing only if nothing parsed:
///
/// ```
/// use driver_lang::{DriverError, Session, Stage};
///
/// struct Lex;
///
/// impl Stage<()> for Lex {
///     type Input = &'static str;
///     type Output = Vec<i64>;
///
///     fn name(&self) -> &'static str {
///         "lex"
///     }
///
///     fn run(&mut self, input: &'static str, session: &mut Session<()>)
///         -> Result<Vec<i64>, DriverError>
///     {
///         let mut out = Vec::new();
///         for word in input.split_whitespace() {
///             match word.parse::<i64>() {
///                 Ok(n) => out.push(n),
///                 Err(_) => { session.warn("skipping non-integer token"); }
///             }
///         }
///         if out.is_empty() {
///             return Err(DriverError::new("no integers in input"));
///         }
///         Ok(out)
///     }
/// }
///
/// let mut session = Session::new(());
/// let tokens = Lex.run("1 two 3", &mut session).unwrap();
/// assert_eq!(tokens, vec![1, 3]);
/// assert_eq!(session.diagnostics().len(), 1); // one "skipping" warning
/// ```
pub trait Stage<C> {
    /// The artifact this stage consumes.
    type Input;

    /// The artifact this stage produces.
    type Output;

    /// A stable, static name for this stage, used to attribute errors.
    fn name(&self) -> &'static str;

    /// Run the stage: consume `input`, use the [`Session`] as needed, and produce
    /// the output.
    ///
    /// Return a [`DriverError`] — never a panic — if the phase cannot produce its
    /// output; the [`Pipeline`](crate::Pipeline) stops there and stamps this
    /// stage's name into the error.
    ///
    /// # Errors
    ///
    /// Returns whatever [`DriverError`] the phase produces when it cannot continue.
    fn run(
        &mut self,
        input: Self::Input,
        session: &mut Session<C>,
    ) -> Result<Self::Output, DriverError>;
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    /// Doubles every element; changes the length-independent shape but not the type.
    struct Double;
    impl Stage<()> for Double {
        type Input = Vec<i64>;
        type Output = Vec<i64>;
        fn name(&self) -> &'static str {
            "double"
        }
        fn run(&mut self, input: Vec<i64>, _s: &mut Session<()>) -> Result<Vec<i64>, DriverError> {
            Ok(input.into_iter().map(|x| x * 2).collect())
        }
    }

    /// A type-changing stage: sums a list into a scalar.
    struct Sum;
    impl Stage<()> for Sum {
        type Input = Vec<i64>;
        type Output = i64;
        fn name(&self) -> &'static str {
            "sum"
        }
        fn run(&mut self, input: Vec<i64>, _s: &mut Session<()>) -> Result<i64, DriverError> {
            Ok(input.iter().sum())
        }
    }

    #[test]
    fn test_stage_transforms_input() {
        let mut s = Session::new(());
        let out = Double.run(vec![1, 2, 3], &mut s).unwrap();
        assert_eq!(out, vec![2, 4, 6]);
    }

    #[test]
    fn test_stage_can_change_output_type() {
        let mut s = Session::new(());
        let out = Sum.run(vec![1, 2, 3], &mut s).unwrap();
        assert_eq!(out, 6);
    }

    #[test]
    fn test_stage_name_is_stable() {
        assert_eq!(Double.name(), "double");
        assert_eq!(Sum.name(), "sum");
    }
}
