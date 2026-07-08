//! A tiny calculator built as a driver: three stages, a shared session, and
//! diagnostics rendered at the end.
//!
//! The pipeline lexes a line of whitespace-separated integers, checks the result
//! against a configured cap, and sums it. It shows the pieces a real compiler
//! driver uses: a configuration carried in the [`Session`], diagnostics emitted by
//! stages, [`Session::abort_if_errors`] as a checkpoint between phases, and a
//! [`Pipeline`] whose stage types are checked at compile time.
//!
//! Run it with:
//!
//! ```text
//! cargo run --example calculator
//! ```

use driver_lang::{DriverError, Pipeline, Session, Stage};

/// Compilation configuration for our calculator: reject any single value larger
/// than this cap.
struct Config {
    max_value: i64,
}

/// Phase 1 — text to integers. Skips (with a warning) any token that is not an
/// integer, and fails outright if nothing at all parsed.
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
        let mut values = Vec::new();
        for token in input.split_whitespace() {
            match token.parse::<i64>() {
                Ok(value) => values.push(value),
                Err(_) => {
                    session.warn(format!("ignoring non-integer token `{token}`"));
                }
            }
        }
        if values.is_empty() {
            return Err(DriverError::new("no integers to sum"));
        }
        Ok(values)
    }
}

/// Phase 2 — validate each value against the configured cap. Emits an error per
/// offending value, then aborts if any were emitted, so one checkpoint can report
/// every problem at once.
struct Check;

impl Stage<Config> for Check {
    type Input = Vec<i64>;
    type Output = Vec<i64>;

    fn name(&self) -> &'static str {
        "check"
    }

    fn run(
        &mut self,
        input: Vec<i64>,
        session: &mut Session<Config>,
    ) -> Result<Vec<i64>, DriverError> {
        let cap = session.config().max_value;
        for &value in &input {
            if value > cap {
                session.error(format!("value {value} exceeds cap {cap}"));
            }
        }
        session.abort_if_errors()?;
        Ok(input)
    }
}

/// Phase 3 — sum the validated values.
struct Sum;

impl Stage<Config> for Sum {
    type Input = Vec<i64>;
    type Output = i64;

    fn name(&self) -> &'static str {
        "sum"
    }

    fn run(&mut self, input: Vec<i64>, _session: &mut Session<Config>) -> Result<i64, DriverError> {
        Ok(input.iter().sum())
    }
}

/// Build the driver fresh for each run so no stage carries state between lines.
fn run_line(line: &'static str, max_value: i64) {
    let mut driver = Pipeline::new(Lex).then(Check).then(Sum);
    let mut session = Session::new(Config { max_value });

    println!("input: {line:?}  (cap {max_value})");
    match driver.run(line, &mut session) {
        Ok(total) => println!("  => {total}"),
        Err(error) => println!("  => failed: {error}"),
    }
    for diagnostic in session.take_diagnostics() {
        println!("     {diagnostic}");
    }
    println!();
}

fn main() {
    // A clean run: everything under the cap, one warning for the stray token.
    run_line("10 20 oops 30", 100);

    // A capped run: two values exceed the cap, so `check` aborts and reports both.
    run_line("5 250 400", 100);

    // An empty run: `lex` fails before any later stage runs.
    run_line("nothing numeric here", 100);
}
