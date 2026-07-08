//! Typed composition of stages into an end-to-end driver.

use core::marker::PhantomData;

use crate::error::DriverError;
use crate::session::Session;
use crate::stage::Stage;

/// Two stages run back to back — the output of [`Then`] is the composition of its
/// parts.
///
/// `Then<A, B>` is itself a [`Stage`]: it feeds an input through `A`, then feeds
/// `A`'s output through `B`, and its own `Input`/`Output` are `A::Input` and
/// `B::Output`. It is the node [`Pipeline::then`] builds, and you rarely name it
/// directly — a three-stage pipeline has type `Pipeline<Then<Then<A, B>, C>>`,
/// assembled for you by chaining `.then(...)`. It is `pub` only because it appears
/// in those inferred types.
///
/// Composition is monomorphized: dispatch to each stage is a direct, inlinable
/// call with no boxing and no virtual dispatch, so a pipeline of any length is as
/// cheap as calling the stages by hand.
#[derive(Debug, Clone)]
pub struct Then<A, B> {
    first: A,
    second: B,
}

impl<C, A, B> Stage<C> for Then<A, B>
where
    A: Stage<C>,
    B: Stage<C, Input = A::Output>,
{
    type Input = A::Input;
    type Output = B::Output;

    /// The name of the final stage in the composition — the stage whose output the
    /// pipeline ultimately produces.
    fn name(&self) -> &'static str {
        self.second.name()
    }

    fn run(
        &mut self,
        input: Self::Input,
        session: &mut Session<C>,
    ) -> Result<Self::Output, DriverError> {
        let mid = self
            .first
            .run(input, session)
            .map_err(|e| e.in_stage(self.first.name()))?;
        self.second
            .run(mid, session)
            .map_err(|e| e.in_stage(self.second.name()))
    }
}

/// An end-to-end driver: a chain of [`Stage`]s whose types line up, run against a
/// [`Session`].
///
/// A `Pipeline` is how stages become a compiler. You start from one stage with
/// [`Pipeline::new`] and extend it with [`then`](Pipeline::then), which appends a
/// stage whose [`Input`](Stage::Input) matches the current
/// [`Output`](Stage::Output). That match is checked *at compile time*: a pipeline
/// that tries to feed tokens into a stage expecting an AST does not build. The
/// finished pipeline is driven with [`run`](Pipeline::run), which threads an input
/// through every stage in order and returns the final artifact.
///
/// On the first stage that returns a [`DriverError`], the pipeline stops — later
/// stages do not run — and the error names the stage that failed. Diagnostics a
/// stage emitted before failing remain in the session.
///
/// The whole pipeline is a single monomorphized type, so composing stages adds no
/// runtime indirection over calling them directly.
///
/// # Examples
///
/// A two-stage calculator driver — tokenize, then sum:
///
/// ```
/// use driver_lang::{DriverError, Pipeline, Session, Stage};
///
/// struct Lex;
/// impl Stage<()> for Lex {
///     type Input = &'static str;
///     type Output = Vec<i64>;
///     fn name(&self) -> &'static str { "lex" }
///     fn run(&mut self, input: &'static str, _s: &mut Session<()>)
///         -> Result<Vec<i64>, DriverError>
///     {
///         input.split_whitespace()
///             .map(|w| w.parse::<i64>().map_err(|_| DriverError::new("not an integer")))
///             .collect()
///     }
/// }
///
/// struct Sum;
/// impl Stage<()> for Sum {
///     type Input = Vec<i64>;
///     type Output = i64;
///     fn name(&self) -> &'static str { "sum" }
///     fn run(&mut self, input: Vec<i64>, _s: &mut Session<()>)
///         -> Result<i64, DriverError>
///     {
///         Ok(input.iter().sum())
///     }
/// }
///
/// let mut driver = Pipeline::new(Lex).then(Sum);
/// let mut session = Session::new(());
///
/// assert_eq!(driver.run("1 2 3 4", &mut session).unwrap(), 10);
/// ```
///
/// The configuration type `C` is a phantom parameter: a pipeline threads a
/// `&mut Session<C>` through its stages but stores no `C` of its own. It is
/// inferred from the stages, so you write `Pipeline::new(stage)` and never name
/// `C` explicitly.
pub struct Pipeline<C, S> {
    stage: S,
    _config: PhantomData<fn() -> C>,
}

impl<C, S> Pipeline<C, S> {
    /// Start a pipeline from a single stage.
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::{DriverError, Pipeline, Session, Stage};
    ///
    /// struct Identity;
    /// impl Stage<()> for Identity {
    ///     type Input = i64;
    ///     type Output = i64;
    ///     fn name(&self) -> &'static str { "identity" }
    ///     fn run(&mut self, input: i64, _s: &mut Session<()>) -> Result<i64, DriverError> {
    ///         Ok(input)
    ///     }
    /// }
    ///
    /// let driver = Pipeline::new(Identity);
    /// assert_eq!(driver.name(), "identity");
    /// ```
    #[must_use]
    pub fn new(stage: S) -> Self {
        Self {
            stage,
            _config: PhantomData,
        }
    }
}

impl<C, S: Stage<C>> Pipeline<C, S> {
    /// Append a stage whose [`Input`](Stage::Input) is this pipeline's current
    /// [`Output`](Stage::Output).
    ///
    /// The type bound `N: Stage<C, Input = S::Output>` is the compile-time check
    /// that the phases connect: the appended stage must accept exactly what the
    /// pipeline currently produces. The result is a longer pipeline that produces
    /// `N`'s output.
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::{DriverError, Pipeline, Session, Stage};
    ///
    /// struct Parse;
    /// impl Stage<()> for Parse {
    ///     type Input = &'static str;
    ///     type Output = usize;
    ///     fn name(&self) -> &'static str { "parse" }
    ///     fn run(&mut self, input: &'static str, _s: &mut Session<()>)
    ///         -> Result<usize, DriverError>
    ///     { Ok(input.len()) }
    /// }
    ///
    /// struct Halve;
    /// impl Stage<()> for Halve {
    ///     type Input = usize;
    ///     type Output = usize;
    ///     fn name(&self) -> &'static str { "halve" }
    ///     fn run(&mut self, input: usize, _s: &mut Session<()>)
    ///         -> Result<usize, DriverError>
    ///     { Ok(input / 2) }
    /// }
    ///
    /// let mut driver = Pipeline::new(Parse).then(Halve);
    /// let mut session = Session::new(());
    /// assert_eq!(driver.run("abcdef", &mut session).unwrap(), 3);
    /// ```
    #[must_use]
    pub fn then<N>(self, next: N) -> Pipeline<C, Then<S, N>>
    where
        N: Stage<C, Input = S::Output>,
    {
        Pipeline {
            stage: Then {
                first: self.stage,
                second: next,
            },
            _config: PhantomData,
        }
    }

    /// Run the pipeline: thread `input` through every stage in order and return the
    /// final output.
    ///
    /// Stages run left to right, each receiving the previous stage's output and the
    /// shared [`Session`]. The run stops at the first stage that returns a
    /// [`DriverError`]; that error names the failing stage.
    ///
    /// # Errors
    ///
    /// Returns the [`DriverError`] of the first stage that fails, with the stage's
    /// name stamped in.
    ///
    /// # Examples
    ///
    /// ```
    /// use driver_lang::{DriverError, Pipeline, Session, Stage};
    ///
    /// struct Fail;
    /// impl Stage<()> for Fail {
    ///     type Input = ();
    ///     type Output = ();
    ///     fn name(&self) -> &'static str { "fail" }
    ///     fn run(&mut self, _input: (), _s: &mut Session<()>) -> Result<(), DriverError> {
    ///         Err(DriverError::new("nope"))
    ///     }
    /// }
    ///
    /// let mut driver = Pipeline::new(Fail);
    /// let mut session = Session::new(());
    /// let err = driver.run((), &mut session).unwrap_err();
    /// assert_eq!(err.stage(), "fail");
    /// assert_eq!(err.message(), "nope");
    /// ```
    pub fn run(
        &mut self,
        input: S::Input,
        session: &mut Session<C>,
    ) -> Result<S::Output, DriverError> {
        let name = self.stage.name();
        self.stage.run(input, session).map_err(|e| e.in_stage(name))
    }

    /// The name of the final stage — the stage whose output this pipeline produces.
    #[must_use]
    #[inline]
    pub fn name(&self) -> &'static str {
        self.stage.name()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    struct Lex;
    impl Stage<()> for Lex {
        type Input = &'static str;
        type Output = Vec<i64>;
        fn name(&self) -> &'static str {
            "lex"
        }
        fn run(
            &mut self,
            input: &'static str,
            _s: &mut Session<()>,
        ) -> Result<Vec<i64>, DriverError> {
            input
                .split_whitespace()
                .map(|w| {
                    w.parse::<i64>()
                        .map_err(|_| DriverError::new("not an integer"))
                })
                .collect()
        }
    }

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

    struct Negate;
    impl Stage<()> for Negate {
        type Input = i64;
        type Output = i64;
        fn name(&self) -> &'static str {
            "negate"
        }
        fn run(&mut self, input: i64, _s: &mut Session<()>) -> Result<i64, DriverError> {
            Ok(-input)
        }
    }

    /// A stage that emits an error then aborts, to prove a mid-pipeline abort is
    /// attributed to the stage that made it.
    struct EmitThenAbort;
    impl Stage<()> for EmitThenAbort {
        type Input = i64;
        type Output = i64;
        fn name(&self) -> &'static str {
            "emit-then-abort"
        }
        fn run(&mut self, _input: i64, session: &mut Session<()>) -> Result<i64, DriverError> {
            session.error("boom");
            session.abort_if_errors()?;
            Ok(0)
        }
    }

    #[test]
    fn test_single_stage_pipeline_runs() {
        let mut driver = Pipeline::new(Sum);
        let mut s = Session::new(());
        assert_eq!(driver.run(vec![1, 2, 3], &mut s).unwrap(), 6);
    }

    #[test]
    fn test_two_stage_pipeline_threads_output() {
        let mut driver = Pipeline::new(Lex).then(Sum);
        let mut s = Session::new(());
        assert_eq!(driver.run("1 2 3 4", &mut s).unwrap(), 10);
    }

    #[test]
    fn test_three_stage_pipeline_threads_output() {
        let mut driver = Pipeline::new(Lex).then(Sum).then(Negate);
        let mut s = Session::new(());
        assert_eq!(driver.run("1 2 3", &mut s).unwrap(), -6);
    }

    #[test]
    fn test_pipeline_name_is_final_stage() {
        let driver = Pipeline::new(Lex).then(Sum).then(Negate);
        assert_eq!(driver.name(), "negate");
    }

    #[test]
    fn test_pipeline_stops_at_first_failing_stage() {
        let mut driver = Pipeline::new(Lex).then(Sum);
        let mut s = Session::new(());
        let err = driver.run("1 nope 3", &mut s).unwrap_err();
        assert_eq!(err.stage(), "lex");
        assert_eq!(err.message(), "not an integer");
    }

    #[test]
    fn test_failing_stage_in_the_middle_is_attributed() {
        // Lex succeeds, Sum succeeds, EmitThenAbort aborts — the error names the
        // stage that actually failed, not an outer wrapper.
        let mut driver = Pipeline::new(Lex).then(Sum).then(EmitThenAbort);
        let mut s = Session::new(());
        let err = driver.run("1 2 3", &mut s).unwrap_err();
        assert_eq!(err.stage(), "emit-then-abort");
        assert_eq!(err.message(), "aborting due to 1 previous error");
        // The diagnostic the failing stage emitted survives in the session.
        assert_eq!(s.diagnostics().len(), 1);
        assert_eq!(s.diagnostics()[0].message(), "boom");
    }

    /// A stage generic over a shared string config, to prove every stage in a
    /// pipeline reads and writes the same `Session<C>`.
    struct TagFromConfig;
    impl Stage<&'static str> for TagFromConfig {
        type Input = i64;
        type Output = i64;
        fn name(&self) -> &'static str {
            "tag-from-config"
        }
        fn run(
            &mut self,
            input: i64,
            session: &mut Session<&'static str>,
        ) -> Result<i64, DriverError> {
            session.note(*session.config());
            Ok(input)
        }
    }

    #[test]
    fn test_stages_share_the_session_config() {
        let mut driver = Pipeline::new(TagFromConfig).then(TagFromConfig);
        let mut s = Session::new("shared");
        let out = driver.run(7, &mut s).unwrap();
        assert_eq!(out, 7);
        // Both stages saw the same config and emitted from it.
        let notes: Vec<_> = s.diagnostics().iter().map(|d| d.message()).collect();
        assert_eq!(notes, ["shared", "shared"]);
    }

    #[test]
    fn test_diagnostics_from_earlier_stages_survive_a_later_failure() {
        struct WarnThenPass;
        impl Stage<()> for WarnThenPass {
            type Input = &'static str;
            type Output = &'static str;
            fn name(&self) -> &'static str {
                "warn-then-pass"
            }
            fn run(
                &mut self,
                input: &'static str,
                session: &mut Session<()>,
            ) -> Result<&'static str, DriverError> {
                session.warn("heads up");
                Ok(input)
            }
        }
        struct AlwaysFail;
        impl Stage<()> for AlwaysFail {
            type Input = &'static str;
            type Output = ();
            fn name(&self) -> &'static str {
                "always-fail"
            }
            fn run(
                &mut self,
                _input: &'static str,
                _s: &mut Session<()>,
            ) -> Result<(), DriverError> {
                Err(DriverError::new("stop"))
            }
        }

        let mut driver = Pipeline::new(WarnThenPass).then(AlwaysFail);
        let mut s = Session::new(());
        let err = driver.run("x", &mut s).unwrap_err();
        assert_eq!(err.stage(), "always-fail");
        assert_eq!(s.diagnostics().len(), 1);
        assert_eq!(s.diagnostics()[0].message(), "heads up");
    }
}
