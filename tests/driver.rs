//! Integration tests: the public types working together as a small compiler
//! driver, exercised through the crate's public API only.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use driver_lang::{Diagnostic, DriverError, Pipeline, Session, Severity, Stage};

/// Configuration a language might carry: here, whether unknown tokens are fatal.
struct Config {
    strict: bool,
}

/// text -> integers, warning (or erroring, under `strict`) on unreadable tokens.
struct Lex;
impl Stage<Config> for Lex {
    type Input = &'static str;
    type Output = Vec<i64>;
    fn name(&self) -> &'static str {
        "lex"
    }
    fn run(
        &mut self,
        input: &'static str,
        session: &mut Session<Config>,
    ) -> Result<Vec<i64>, DriverError> {
        let strict = session.config().strict;
        let mut out = Vec::new();
        for token in input.split_whitespace() {
            match token.parse::<i64>() {
                Ok(value) => out.push(value),
                Err(_) if strict => {
                    session.error("unexpected token");
                }
                Err(_) => {
                    session.warn("skipping token");
                }
            }
        }
        session.abort_if_errors()?;
        Ok(out)
    }
}

/// integers -> their sum.
struct Sum;
impl Stage<Config> for Sum {
    type Input = Vec<i64>;
    type Output = i64;
    fn name(&self) -> &'static str {
        "sum"
    }
    fn run(&mut self, input: Vec<i64>, _s: &mut Session<Config>) -> Result<i64, DriverError> {
        Ok(input.iter().sum())
    }
}

#[test]
fn test_full_pipeline_happy_path() {
    let mut driver = Pipeline::new(Lex).then(Sum);
    let mut session = Session::new(Config { strict: false });

    let total = driver.run("2 4 6 8", &mut session).unwrap();

    assert_eq!(total, 20);
    assert!(!session.has_errors());
    assert!(session.diagnostics().is_empty());
}

#[test]
fn test_lenient_run_warns_but_succeeds() {
    let mut driver = Pipeline::new(Lex).then(Sum);
    let mut session = Session::new(Config { strict: false });

    let total = driver.run("1 x 2 y 3", &mut session).unwrap();

    assert_eq!(total, 6);
    assert_eq!(session.error_count(), 0);
    assert_eq!(session.diagnostics().len(), 2); // two skipped-token warnings
    assert_eq!(session.diagnostics()[0].severity(), Severity::Warning);
}

#[test]
fn test_strict_run_aborts_at_lex_and_attributes_stage() {
    let mut driver = Pipeline::new(Lex).then(Sum);
    let mut session = Session::new(Config { strict: true });

    let error = driver.run("1 x 2 y 3", &mut session).unwrap_err();

    // Two bad tokens were reported before the checkpoint aborted.
    assert_eq!(error.stage(), "lex");
    assert_eq!(error.message(), "aborting due to 2 previous errors");
    assert_eq!(session.error_count(), 2);
}

#[test]
fn test_diagnostics_can_be_drained_and_rendered() {
    let mut driver = Pipeline::new(Lex).then(Sum);
    let mut session = Session::new(Config { strict: false });
    let _ = driver.run("7 nope 8", &mut session).unwrap();

    let rendered: Vec<String> = session
        .take_diagnostics()
        .iter()
        .map(Diagnostic::to_string)
        .collect();

    assert_eq!(rendered, ["warning: skipping token"]);
    assert!(session.diagnostics().is_empty());
}

#[test]
fn test_config_can_be_read_across_stages() {
    // A stage that reads config, and a second that reads it again: both see the
    // same value threaded through the one session.
    struct EchoStrict;
    impl Stage<Config> for EchoStrict {
        type Input = i64;
        type Output = i64;
        fn name(&self) -> &'static str {
            "echo-strict"
        }
        fn run(&mut self, input: i64, session: &mut Session<Config>) -> Result<i64, DriverError> {
            if session.config().strict {
                session.note("strict mode is on");
            }
            Ok(input)
        }
    }

    let mut driver = Pipeline::new(EchoStrict).then(EchoStrict);
    let mut session = Session::new(Config { strict: true });
    let out = driver.run(99, &mut session).unwrap();

    assert_eq!(out, 99);
    assert_eq!(session.diagnostics().len(), 2);
    assert!(
        session
            .diagnostics()
            .iter()
            .all(|d| d.severity() == Severity::Note)
    );
}
