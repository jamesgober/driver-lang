//! Property-based tests for the invariants that must hold across arbitrary input.
//!
//! These exercise the session's diagnostic accounting and the pipeline's
//! value-threading over a wide, randomized input space, catching corner cases that
//! hand-written cases miss.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use driver_lang::{DriverError, Pipeline, Session, Severity, Stage};
use proptest::prelude::*;

/// A severity chosen by proptest, as a small tag we can map back to `Severity`.
fn severity_strategy() -> impl Strategy<Value = Severity> {
    prop_oneof![
        Just(Severity::Error),
        Just(Severity::Warning),
        Just(Severity::Note),
    ]
}

proptest! {
    /// The session's error count is exactly the number of error-severity
    /// diagnostics emitted, and the diagnostic list holds every emission in order.
    #[test]
    fn prop_error_count_tracks_error_emissions(severities in prop::collection::vec(severity_strategy(), 0..64)) {
        let expected_errors = severities.iter().filter(|s| s.is_error()).count();

        let mut session = Session::new(());
        for &severity in &severities {
            match severity {
                Severity::Error => { session.error("e"); }
                Severity::Warning => { session.warn("w"); }
                Severity::Note => { session.note("n"); }
            }
        }

        prop_assert_eq!(session.error_count(), expected_errors);
        prop_assert_eq!(session.diagnostics().len(), severities.len());
        prop_assert_eq!(session.has_errors(), expected_errors > 0);
        prop_assert_eq!(session.abort_if_errors().is_err(), expected_errors > 0);
    }

    /// `take_diagnostics` returns every recorded diagnostic and leaves the session
    /// clean.
    #[test]
    fn prop_take_diagnostics_drains_all_and_resets(errors in 0usize..32, warnings in 0usize..32) {
        let mut session = Session::new(());
        for _ in 0..errors { session.error("e"); }
        for _ in 0..warnings { session.warn("w"); }

        let drained = session.take_diagnostics();

        prop_assert_eq!(drained.len(), errors + warnings);
        prop_assert_eq!(session.diagnostics().len(), 0);
        prop_assert_eq!(session.error_count(), 0);
        prop_assert!(!session.has_errors());
    }
}

/// Doubles every element — a same-type transform stage.
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

/// Sums a list into a scalar — a type-changing stage.
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

proptest! {
    /// A `Double -> Sum` pipeline computes the same result as doing the two steps
    /// by hand, for any input. Values are drawn as `i32` and widened, so neither
    /// the doubling nor the sum can overflow `i64`.
    #[test]
    fn prop_pipeline_matches_manual_composition(values in prop::collection::vec(any::<i32>(), 0..256)) {
        let widened: Vec<i64> = values.iter().map(|&v| v as i64).collect();
        let expected: i64 = widened.iter().map(|&v| v * 2).sum();

        let mut driver = Pipeline::new(Double).then(Sum);
        let mut session = Session::new(());
        let got = driver.run(widened, &mut session).unwrap();

        prop_assert_eq!(got, expected);
        prop_assert!(session.diagnostics().is_empty());
    }
}
