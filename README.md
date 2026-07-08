<h1 align="center">
    <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br>
    <b>driver-lang</b>
    <br>
    <sub><sup>COMPILER DRIVER</sup></sub>
</h1>

<div align="center">
    <a href="https://crates.io/crates/driver-lang"><img alt="Crates.io" src="https://img.shields.io/crates/v/driver-lang"></a>
    <a href="https://crates.io/crates/driver-lang"><img alt="Downloads" src="https://img.shields.io/crates/d/driver-lang?color=%230099ff"></a>
    <a href="https://docs.rs/driver-lang"><img alt="docs.rs" src="https://img.shields.io/docsrs/driver-lang"></a>
    <a href="https://github.com/jamesgober/driver-lang/actions"><img alt="CI" src="https://github.com/jamesgober/driver-lang/actions/workflows/ci.yml/badge.svg"></a>
    <a href="https://github.com/rust-lang/rfcs/blob/master/text/2495-min-rust-version.md"><img alt="MSRV" src="https://img.shields.io/badge/MSRV-1.85%2B-blue"></a>
</div>

<br>

<div align="left">
    <p>
        driver-lang is the TOOL-tier crate: the driver that ties a compiler's phases into one run — session state, diagnostics, and a type-checked stage pipeline. Part of the -lang language-construction family; see _strategy/LANG_COLLECTION.md for the master plan.
    </p>
    <br>
    <hr>
    <p>
        <strong>MSRV is 1.85+</strong> (Rust 2024 edition).
    </p>
    <blockquote>
        <strong>Status: stable.</strong> As of <code>1.0.0</code> the public API is frozen under Semantic Versioning; see <a href="./docs/API.md#stability"><code>docs/API.md</code></a> for the promise and <a href="./CHANGELOG.md"><code>CHANGELOG.md</code></a> for the history.
    </blockquote>
</div>

<hr>
<br>

<div align="left">
    <p>
        <strong>driver-lang</strong> is the orchestration layer a compiler front-end hangs off of. A compiler is a sequence of phases — lex, parse, resolve names, check types, lower, emit — and each phase takes the previous phase's artifact and produces the next. The machinery that holds those phases together is the same for every language, and that machinery is all this crate is.
    </p>
    <p>
        It contributes three pieces. A <a href="./docs/API.md#session"><code>Session</code></a> carries the compilation configuration and the diagnostics every phase emits, and tracks how many were errors so a driver can stop at the right moment. A <a href="./docs/API.md#stage"><code>Stage</code></a> is one phase, expressed as a transform from an input artifact to an output artifact run against the session. A <a href="./docs/API.md#pipeline"><code>Pipeline</code></a> composes stages end to end, checking at <em>compile time</em> that each stage's output type is the next stage's input type.
    </p>
    <p>
        The crate owns no compiler phases and wires no first-party dependency. It is generic over the artifacts that flow through it and the configuration the session carries, so a language plugs its own lexer, parser, and backend in as stages — the same way a production compiler's driver is a thin orchestrator over phases it does not itself implement. The crate is <code>no_std</code> (needs only <code>alloc</code>) and contains no <code>unsafe</code> (<code>#![forbid(unsafe_code)]</code>).
    </p>
</div>

<hr>
<br>

## Performance First

A pipeline is a single monomorphized type: composing stages with [`then`](./docs/API.md#then) builds a nested [`Then`](./docs/API.md#then-type) whose `run` inlines into straight-line calls, so driving a pipeline of any length costs no more than calling the stages by hand — no boxing, no `dyn` dispatch, no per-phase indirection. The session keeps its error tally incrementally, so [`error_count`](./docs/API.md#error_count) and [`has_errors`](./docs/API.md#has_errors) are O(1) checkpoints a driver can run between every phase. Diagnostic messages are [`Cow<'static, str>`](https://doc.rust-lang.org/std/borrow/enum.Cow.html), so a constant message never allocates and a computed one is owned only when it is built.

Latest local Criterion means (`cargo bench --bench bench`, Linux x86_64 / WSL2, Rust stable, release build):

| Workload | Time |
|----------|-----:|
| `Double → Increment → Sum` pipeline, 16 elements | ~15 ns |
| `Double → Increment → Sum` pipeline, 256 elements | ~100 ns |
| `Double → Increment → Sum` pipeline, 4096 elements | ~1.16 µs |
| Emit 128 diagnostics into a session | ~260 ns |

Numbers vary by CPU and environment; run the suite on your target to establish a baseline. The pipeline benchmark clones its input each iteration, so it includes the allocation the two mapping stages consume.

<br>
<hr>

## Features

- **Type-checked phase chain** — [`Pipeline::then`](./docs/API.md#then) only accepts a stage whose input is the current output. A pipeline that feeds tokens into a stage expecting an AST does not compile.
- **One shared session** — every stage reads and writes the same [`Session`](./docs/API.md#session): the language's configuration, and the diagnostics accumulated so far.
- **Diagnostics with severities** — [`error`](./docs/API.md#severity), `warning`, and `note`, recorded in order; only errors count toward the abort checkpoint.
- **Clean stops** — a stage returns a [`DriverError`](./docs/API.md#drivererror) to halt the run, and the pipeline stamps in the failing stage's name. [`Session::abort_if_errors`](./docs/API.md#abort_if_errors) turns accumulated errors into that stop.
- **Zero-cost composition** — fully monomorphized; no dynamic dispatch on the run path.
- **`no_std`** — needs only `alloc`; the default `std` feature only toggles the `no_std` attribute.
- **Fully safe** — no `unsafe`, `#![forbid(unsafe_code)]`.
- **Property-tested** — diagnostic accounting and pipeline value-threading checked across randomized inputs with `proptest`.

<br>
<hr>

## Installation

```toml
[dependencies]
driver-lang = "1"

# no_std:
driver-lang = { version = "1", default-features = false }

# with serde derives on Severity and Diagnostic:
driver-lang = { version = "1", features = ["serde"] }
```

**MSRV is 1.85+** (Rust 2024 edition).

<hr>
<br>

## Quick Start

Build a driver from stages and run an input through it. Each stage is a phase; the pipeline threads the artifact from one to the next.

```rust
use driver_lang::{DriverError, Pipeline, Session, Stage};

// Phase 1: text -> integers.
struct Lex;
impl Stage<()> for Lex {
    type Input = &'static str;
    type Output = Vec<i64>;
    fn name(&self) -> &'static str { "lex" }
    fn run(&mut self, input: &'static str, _session: &mut Session<()>)
        -> Result<Vec<i64>, DriverError>
    {
        input.split_whitespace()
            .map(|w| w.parse::<i64>().map_err(|_| DriverError::new("not an integer")))
            .collect()
    }
}

// Phase 2: integers -> their sum.
struct Sum;
impl Stage<()> for Sum {
    type Input = Vec<i64>;
    type Output = i64;
    fn name(&self) -> &'static str { "sum" }
    fn run(&mut self, input: Vec<i64>, _session: &mut Session<()>)
        -> Result<i64, DriverError>
    { Ok(input.iter().sum()) }
}

let mut driver = Pipeline::new(Lex).then(Sum);
let mut session = Session::new(());

assert_eq!(driver.run("1 2 3 4", &mut session).unwrap(), 10);
```

### Diagnostics and a clean stop

A stage records problems as diagnostics and decides when they are fatal. `abort_if_errors` is the checkpoint: it reports every error emitted so far at once, instead of stopping at the first.

```rust
use driver_lang::{DriverError, Pipeline, Session, Stage};

struct Lex;
impl Stage<()> for Lex {
    type Input = &'static str;
    type Output = Vec<i64>;
    fn name(&self) -> &'static str { "lex" }
    fn run(&mut self, input: &'static str, session: &mut Session<()>)
        -> Result<Vec<i64>, DriverError>
    {
        let mut out = Vec::new();
        for token in input.split_whitespace() {
            match token.parse::<i64>() {
                Ok(n) => out.push(n),
                Err(_) => { session.error("not an integer"); }
            }
        }
        session.abort_if_errors()?; // stop here if any token failed
        Ok(out)
    }
}

let mut driver = Pipeline::new(Lex);
let mut session = Session::new(());

let err = driver.run("1 x 2 y", &mut session).unwrap_err();
assert_eq!(err.stage(), "lex");
assert_eq!(err.message(), "aborting due to 2 previous errors");
assert_eq!(session.error_count(), 2); // both bad tokens were reported
```

See [`examples/calculator.rs`](./examples/calculator.rs) for a three-stage driver — lex, cap-check, sum — that renders its diagnostics at the end.

<hr>
<br>

## How the pipeline fits together

A [`Stage<C>`](./docs/API.md#stage) has an `Input` type, an `Output` type, a `name`, and a `run` that consumes the input, uses the shared [`Session<C>`](./docs/API.md#session), and produces the output. [`Pipeline::then`](./docs/API.md#then) appends a stage with the bound `N: Stage<C, Input = S::Output>` — the compile-time check that the phases connect. The result is a longer pipeline whose type encodes the whole chain, and whose `run` threads an input through every stage left to right.

The distinction from a *pass manager* is the type change. A pass rewrites one type in place (an IR stays an IR); a stage transforms one artifact into a different one (source becomes tokens becomes an AST). Managing that type-threading — while carrying the session and stopping cleanly on failure — is exactly what a driver exists to do, and it is why this is a separate crate from [`pass-lang`](https://crates.io/crates/pass-lang).

When a stage returns a [`DriverError`](./docs/API.md#drivererror), the run stops there; later stages do not execute, and the error names the stage that failed. Diagnostics emitted before the failure remain in the session, so a driver can render them alongside the error.

<hr>
<br>

## API Overview

For the complete reference with examples, see [`docs/API.md`](./docs/API.md).

- [`Session<C>`](./docs/API.md#session) — the ambient state of one run: configuration and diagnostics.
  - **Config:** [`new`](./docs/API.md#session-new), [`config`](./docs/API.md#config), [`config_mut`](./docs/API.md#config_mut), [`into_config`](./docs/API.md#into_config).
  - **Diagnostics:** [`emit`](./docs/API.md#emit), [`error`](./docs/API.md#session-error), [`warn`](./docs/API.md#warn), [`note`](./docs/API.md#note), [`diagnostics`](./docs/API.md#diagnostics), [`take_diagnostics`](./docs/API.md#take_diagnostics).
  - **Checkpoints:** [`error_count`](./docs/API.md#error_count), [`has_errors`](./docs/API.md#has_errors), [`abort_if_errors`](./docs/API.md#abort_if_errors).
- [`Stage<C>`](./docs/API.md#stage) — one phase: `Input`, `Output`, [`name`](./docs/API.md#stage-name), [`run`](./docs/API.md#stage-run).
- [`Pipeline<C, S>`](./docs/API.md#pipeline) — stages composed: [`new`](./docs/API.md#pipeline-new), [`then`](./docs/API.md#then), [`run`](./docs/API.md#pipeline-run), [`name`](./docs/API.md#pipeline-name).
- [`Diagnostic`](./docs/API.md#diagnostic) / [`Severity`](./docs/API.md#severity) — one message and its seriousness.
- [`DriverError`](./docs/API.md#drivererror) — the failure that halts a run.

<br>

### Feature Flags

| Feature | Default | Description                                                                 |
|---------|:-------:|-----------------------------------------------------------------------------|
| `std`   | ✅      | Links the standard library. With it off the crate is `no_std` + `alloc`; every type works in both modes. |
| `serde` | ⬜      | Derives `serde::Serialize` for [`Severity`](./docs/API.md#severity) and [`Diagnostic`](./docs/API.md#diagnostic), so a run's diagnostics can be serialized for tooling. |

<hr>
<br>

## Testing

```bash
cargo test                    # unit + doctests
cargo test --all-features     # adds the serde-gated paths
cargo test --test proptests   # property-based invariants
cargo test --test driver      # end-to-end integration tests
cargo bench --bench bench     # Criterion pipeline + session benchmarks
cargo run --example calculator
```

The property suite in [`tests/proptests.rs`](./tests/proptests.rs) checks the core invariants — the session's error count matches the errors emitted, `take_diagnostics` drains and resets, and a pipeline computes the same result as doing its stages by hand — across randomized inputs.

<hr>
<br>

## Cross-Platform Support

The driver is pure computation with no platform-specific code, so it behaves identically everywhere Rust runs. CI covers **Linux**, **macOS**, and **Windows** on both stable and the 1.85 MSRV.

<hr>
<br>

## Contributing

See <a href="./REPS.md"><code>REPS.md</code></a> for the engineering standards and the definition of done. Before a PR: `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features` must be clean.

<br>

<div id="license">
    <h2>License</h2>
    <p>Licensed under either of</p>
    <ul>
        <li><b>Apache License, Version 2.0</b> &mdash; <a href="./LICENSE-APACHE">LICENSE-APACHE</a></li>
        <li><b>MIT License</b> &mdash; <a href="./LICENSE-MIT">LICENSE-MIT</a></li>
    </ul>
    <p>at your option.</p>
</div>

<div align="center">
  <h2></h2>
  <sup>COPYRIGHT <small>&copy;</small> 2026 <strong>James Gober <me@jamesgober.com>.</strong></sup>
</div>
