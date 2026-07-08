<h1 align="center">
    <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br><b>driver-lang</b><br>
    <sub><sup>API REFERENCE</sup></sub>
</h1>
<div align="center">
    <sup>
        <a href="../README.md" title="Project Home"><b>HOME</b></a>
        <span>&nbsp;│&nbsp;</span>
        <span>API</span>
        <span>&nbsp;│&nbsp;</span>
        <a href="../CHANGELOG.md" title="Changelog"><b>CHANGELOG</b></a>
    </sup>
</div>
<br>

The driver that ties a compiler's phases into one run. The public surface is five
types: a [`Session`](#session) carrying configuration and diagnostics, a
[`Stage`](#stage) for one phase, a [`Pipeline`](#pipeline) composing stages, and
the [`Diagnostic`](#diagnostic) / [`Severity`](#severity) / [`DriverError`](#drivererror)
values they produce.

- **Version:** 1.0.0
- **MSRV:** Rust 1.85 (2024 edition)
- **`no_std`:** yes (needs `alloc`; the default `std` feature only toggles the `no_std` attribute)
- **Unsafe:** none — `#![forbid(unsafe_code)]`
- **Stability:** stable — the surface below is frozen (see [Stability](#stability)).

## Table of Contents

- **[Stability](#stability)**
- **[Installation](#installation)**
- **[Quick Start](#quick-start)**
- **[Mental model](#mental-model)**
- **[Public API](#public-api)**
  - [`Session<C>`](#session)
    - [`Session::new`](#session-new)
    - [`Session::config`](#config) · [`config_mut`](#config_mut) · [`into_config`](#into_config)
    - [`Session::emit`](#emit)
    - [`Session::error`](#session-error) · [`warn`](#warn) · [`note`](#note)
    - [`Session::diagnostics`](#diagnostics) · [`take_diagnostics`](#take_diagnostics)
    - [`Session::error_count`](#error_count) · [`has_errors`](#has_errors)
    - [`Session::abort_if_errors`](#abort_if_errors)
  - [`Stage<C>`](#stage)
    - [`Stage::name`](#stage-name)
    - [`Stage::run`](#stage-run)
  - [`Pipeline<C, S>`](#pipeline)
    - [`Pipeline::new`](#pipeline-new)
    - [`Pipeline::then`](#then)
    - [`Pipeline::run`](#pipeline-run)
    - [`Pipeline::name`](#pipeline-name)
    - [`Then<A, B>`](#then-type)
  - [`Diagnostic`](#diagnostic)
  - [`Severity`](#severity)
  - [`DriverError`](#drivererror)
- **[Feature flags](#feature-flags)**
- **[Design notes](#design-notes)**

<br>

## Stability

As of **1.0.0** the public API documented here is **stable and frozen**. The crate
follows [Semantic Versioning](https://semver.org):

- Nothing in the frozen surface — [`Session`](#session), [`Stage`](#stage),
  [`Pipeline`](#pipeline) and its [`Then`](#then-type) node,
  [`Diagnostic`](#diagnostic), [`Severity`](#severity), [`DriverError`](#drivererror),
  every method listed above, and the `std` / `serde` feature flags — will be removed
  or changed in a breaking way within the `1.x` series. A breaking change means a new
  major version.
- `1.x` releases may **add** to the surface (new methods, new trait impls, new
  feature-gated helpers) without breaking existing code.
- Two behaviors are part of the contract, not just the current implementation: a
  pipeline runs its stages left to right and stops at the first
  [`DriverError`](#drivererror); and only [`Severity::Error`](#severity) counts toward
  [`error_count`](#error_count) and [`abort_if_errors`](#abort_if_errors). A `1.x`
  release will not change either except to fix a documented bug.
- The private fields behind `Session`, `Pipeline`, `Then`, `Diagnostic`, and
  `DriverError` are not part of the surface; they may change freely.
- MSRV (Rust 1.85) is a compatibility surface: a raise is a minor, documented change,
  never a patch.

<br>

## Installation

```toml
[dependencies]
driver-lang = "1"

# no_std:
driver-lang = { version = "1", default-features = false }

# with serde derives on Severity and Diagnostic:
driver-lang = { version = "1", features = ["serde"] }
```

<br>

## Quick Start

```rust
use driver_lang::{DriverError, Pipeline, Session, Stage};

struct Lex;
impl Stage<()> for Lex {
    type Input = &'static str;
    type Output = Vec<i64>;
    fn name(&self) -> &'static str { "lex" }
    fn run(&mut self, input: &'static str, _s: &mut Session<()>)
        -> Result<Vec<i64>, DriverError>
    {
        input.split_whitespace()
            .map(|w| w.parse().map_err(|_| DriverError::new("not an integer")))
            .collect()
    }
}

struct Sum;
impl Stage<()> for Sum {
    type Input = Vec<i64>;
    type Output = i64;
    fn name(&self) -> &'static str { "sum" }
    fn run(&mut self, input: Vec<i64>, _s: &mut Session<()>)
        -> Result<i64, DriverError>
    { Ok(input.iter().sum()) }
}

let mut driver = Pipeline::new(Lex).then(Sum);
let mut session = Session::new(());
assert_eq!(driver.run("1 2 3 4", &mut session).unwrap(), 10);
```

<br>

## Mental model

A compiler is a chain of phases. Each phase takes the previous one's artifact and
produces the next: source → tokens → AST → typed AST → IR → object code. driver-lang
gives you three pieces to express that chain and run it:

- A **[`Stage<C>`](#stage)** is one phase — an `Input` type, an `Output` type, and a
  `run` that turns the first into the second while reading and writing a shared
  session. Unlike a *pass* (which rewrites one type in place), a stage **changes the
  type** as the program moves down the pipeline.
- A **[`Session<C>`](#session)** is the ambient state every stage shares: the
  compilation configuration `C`, and the [`Diagnostic`](#diagnostic)s the phases
  emit. It tracks how many diagnostics were errors, so the driver can stop at the
  right point.
- A **[`Pipeline<C, S>`](#pipeline)** composes stages end to end. [`then`](#then)
  only accepts a stage whose input matches the current output — the phases are
  checked to connect at compile time — and [`run`](#pipeline-run) threads an input
  through the whole chain.

There is no dynamic dispatch: a pipeline is one monomorphized type, so composing
stages costs nothing over calling them by hand.

<br>

## Public API

<a id="session"></a>
### `Session<C>`

```rust
pub struct Session<C> { /* private */ }
```

The ambient state threaded through one compilation run. It owns the compilation
*configuration* (`C` — target, options, input paths, whatever a language needs) and
the *diagnostics* every stage emits, and it keeps a running count of how many were
errors. Every [`Stage`](#stage) receives `&mut Session<C>`, so the session is how an
earlier phase's findings reach a later one.

`C` is whatever a language defines. The session never inspects it — it hands out
`&C` / `&mut C` and otherwise leaves it alone — so the crate holds no opinion about
what a configuration looks like. Derives `Debug` and `Clone` when `C` does.

---

<a id="session-new"></a>
#### `Session::new`

```rust
pub fn new(config: C) -> Session<C>
```

Create a session over a configuration, with no diagnostics recorded yet.

**Parameters**

- `config: C` — the compilation configuration this run carries. Borrow it later with
  [`config`](#config), mutate it with [`config_mut`](#config_mut), or reclaim it with
  [`into_config`](#into_config).

**Returns** a fresh `Session<C>` with an empty diagnostic list and a zero error count.

**Examples**

```rust
use driver_lang::Session;

// A unit config when a run needs no options.
let session = Session::new(());
assert!(!session.has_errors());

// A real config: here, a target triple.
let session = Session::new("x86_64-unknown-linux-gnu");
assert_eq!(*session.config(), "x86_64-unknown-linux-gnu");
```

---

<a id="config"></a>
#### `Session::config` / <a id="config_mut"></a>`config_mut` / <a id="into_config"></a>`into_config`

```rust
pub fn config(&self) -> &C
pub fn config_mut(&mut self) -> &mut C
pub fn into_config(self) -> C
```

Access the configuration. [`config`](#config) borrows it (any stage can read the
options a run was started with), [`config_mut`](#config_mut) borrows it mutably (a
stage can update an option later stages read), and [`into_config`](#into_config)
consumes the session and returns the config, discarding the diagnostics — call
[`take_diagnostics`](#take_diagnostics) first if you need to keep them.

**Returns** `&C`, `&mut C`, and `C` respectively.

**Examples**

```rust
use driver_lang::Session;

struct Options { optimize: bool }

let mut session = Session::new(Options { optimize: false });
assert!(!session.config().optimize);

// A stage flips an option the next stage will read.
session.config_mut().optimize = true;
assert!(session.config().optimize);

// Reclaim the config when the run is done.
let options = session.into_config();
assert!(options.optimize);
```

---

<a id="emit"></a>
#### `Session::emit`

```rust
pub fn emit(&mut self, diagnostic: Diagnostic) -> &mut Self
```

Record a [`Diagnostic`](#diagnostic), updating the error count if it is an error.
This is the one path by which diagnostics enter the session; the
[`error`](#session-error) / [`warn`](#warn) / [`note`](#note) shorthands all route
through it. Returns `&mut Self` so emissions can be chained.

**Parameters**

- `diagnostic: Diagnostic` — the message to record. If its severity is
  [`Error`](#severity), [`error_count`](#error_count) increments.

**Returns** `&mut Self` for chaining.

**Examples**

```rust
use driver_lang::{Diagnostic, Session};

let mut session = Session::new(());
session
    .emit(Diagnostic::error("cannot find type `Foo`"))
    .emit(Diagnostic::note("defined in a later module"));

assert_eq!(session.diagnostics().len(), 2);
assert_eq!(session.error_count(), 1); // the note does not count
```

Use `emit` directly when a diagnostic is built elsewhere (for example, one carried
across a boundary):

```rust
use driver_lang::{Diagnostic, Severity, Session};

fn from_level(level: u8, msg: &'static str) -> Diagnostic {
    let severity = if level == 0 { Severity::Error } else { Severity::Warning };
    Diagnostic::new(severity, msg)
}

let mut session = Session::new(());
session.emit(from_level(0, "hard failure"));
assert!(session.has_errors());
```

---

<a id="session-error"></a>
#### `Session::error` / <a id="warn"></a>`warn` / <a id="note"></a>`note`

```rust
pub fn error(&mut self, message: impl Into<Cow<'static, str>>) -> &mut Self
pub fn warn(&mut self, message: impl Into<Cow<'static, str>>) -> &mut Self
pub fn note(&mut self, message: impl Into<Cow<'static, str>>) -> &mut Self
```

Emit a diagnostic of the named severity — shorthands for
`emit(Diagnostic::error(message))` and friends. These are the common way a stage
reports what it found.

**Parameters**

- `message: impl Into<Cow<'static, str>>` — the message. A string literal is stored
  without allocation; an owned `String` (e.g. from `format!`) is taken by value.

**Returns** `&mut Self` for chaining.

**Examples**

```rust
use driver_lang::Session;

let mut session = Session::new(());
session
    .error("expected `;`")
    .warn("unused variable `x`")
    .note("`x` is introduced here");

assert_eq!(session.error_count(), 1);
assert_eq!(session.diagnostics().len(), 3);
```

A computed message with `format!`:

```rust
use driver_lang::Session;

let mut session = Session::new(());
let name = "Frobnicate";
session.error(format!("unknown function `{name}`"));
assert_eq!(session.diagnostics()[0].message(), "unknown function `Frobnicate`");
```

---

<a id="diagnostics"></a>
#### `Session::diagnostics`

```rust
pub fn diagnostics(&self) -> &[Diagnostic]
```

All diagnostics recorded so far, in emission order. Read them to render output, or
inspect them in a test.

**Returns** a slice `&[Diagnostic]` over every emission.

**Examples**

```rust
use driver_lang::Session;

let mut session = Session::new(());
session.note("a").warn("b").error("c");

let rendered: Vec<String> = session.diagnostics().iter().map(|d| d.to_string()).collect();
assert_eq!(rendered, ["note: a", "warning: b", "error: c"]);
```

---

<a id="take_diagnostics"></a>
#### `Session::take_diagnostics`

```rust
pub fn take_diagnostics(&mut self) -> Vec<Diagnostic>
```

Remove and return all recorded diagnostics, resetting the error count to zero. The
configuration is left untouched. Use it to drain diagnostics for rendering between
runs, or to hand them off before [`into_config`](#into_config) discards the session.

**Returns** an owned `Vec<Diagnostic>` of every diagnostic emitted since the last
drain.

**Examples**

```rust
use driver_lang::Session;

let mut session = Session::new(());
session.error("boom").warn("hmm");

let drained = session.take_diagnostics();
assert_eq!(drained.len(), 2);

// The session is clean again.
assert!(!session.has_errors());
assert!(session.diagnostics().is_empty());
```

---

<a id="error_count"></a>
#### `Session::error_count` / <a id="has_errors"></a>`has_errors`

```rust
pub fn error_count(&self) -> usize
pub fn has_errors(&self) -> bool
```

How many [`Error`](#severity)-severity diagnostics have been emitted, and whether
that count is non-zero. Both are O(1) — the count is maintained incrementally as
diagnostics are emitted — so a driver can check them between every phase without
re-scanning the list.

**Returns** the error count as `usize`, and `error_count() != 0` as `bool`.

**Examples**

```rust
use driver_lang::Session;

let mut session = Session::new(());
assert_eq!(session.error_count(), 0);
assert!(!session.has_errors());

session.warn("just a warning");
assert!(!session.has_errors()); // warnings do not count

session.error("a real error");
assert_eq!(session.error_count(), 1);
assert!(session.has_errors());
```

---

<a id="abort_if_errors"></a>
#### `Session::abort_if_errors`

```rust
pub fn abort_if_errors(&self) -> Result<(), DriverError>
```

Stop the run if any error has been emitted. This is the driver's checkpoint: emit
diagnostics through a phase, then call `abort_if_errors` to turn accumulated errors
into the [`DriverError`](#drivererror) that ends the pipeline — the "aborting due to
N previous errors" step at the end of a compiler phase. Because a stage can emit
several errors before this is checked, one checkpoint reports many problems at once
instead of stopping at the first.

**Returns**

- `Ok(())` when the session holds no errors — so `session.abort_if_errors()?` inside
  [`run`](#stage-run) is a no-op on a healthy run.
- `Err(DriverError)` reporting the error count otherwise. The error has no stage name
  yet; the [`Pipeline`](#pipeline) stamps in the stage that made the call.

**Examples**

Clean session — no-op:

```rust
use driver_lang::Session;

let mut session = Session::new(());
session.warn("only a warning");
assert!(session.abort_if_errors().is_ok());
```

As a phase checkpoint that reports every error at once:

```rust
use driver_lang::{DriverError, Session, Stage, Pipeline};

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
        session.abort_if_errors()?; // reports both bad tokens together
        Ok(out)
    }
}

let mut driver = Pipeline::new(Lex);
let mut session = Session::new(());
let err = driver.run("1 x 2 y", &mut session).unwrap_err();
assert_eq!(err.message(), "aborting due to 2 previous errors");
```

<br>

<a id="stage"></a>
### `Stage<C>`

```rust
pub trait Stage<C> {
    type Input;
    type Output;
    fn name(&self) -> &'static str;
    fn run(&mut self, input: Self::Input, session: &mut Session<C>)
        -> Result<Self::Output, DriverError>;
}
```

One phase of compilation: a transform from an input artifact to an output artifact,
run against a [`Session`](#session). Implement it for each phase of your compiler —
a lexer is `Stage<Input = Source, Output = Tokens>`, a parser is `Stage<Input =
Tokens, Output = Ast>`, and so on. A stage is generic over the session's
configuration type `C`, so every stage in a pipeline shares one configuration and
one diagnostic buffer.

Compose stages with [`Pipeline`](#pipeline), which enforces that each stage's
`Output` is the next stage's `Input`.

**Contract**

- [`name`](#stage-name) is a stable, static identifier used to attribute errors. It
  must not change between runs.
- [`run`](#stage-run) consumes `input`, may read and write the session, and returns
  the output — or a [`DriverError`](#drivererror) if it cannot proceed. It must not
  panic on a recoverable condition.

---

<a id="stage-name"></a>
#### `Stage::name`

```rust
fn name(&self) -> &'static str
```

A stable, static name for this stage. It appears in a [`DriverError`](#drivererror)
when the stage fails, so a caller can tell which phase halted the run.

**Returns** the stage's identifier.

**Examples**

```rust
use driver_lang::{DriverError, Session, Stage};

struct TypeCheck;
impl Stage<()> for TypeCheck {
    type Input = ();
    type Output = ();
    fn name(&self) -> &'static str { "type-check" }
    fn run(&mut self, _input: (), _s: &mut Session<()>) -> Result<(), DriverError> {
        Ok(())
    }
}

assert_eq!(TypeCheck.name(), "type-check");
```

---

<a id="stage-run"></a>
#### `Stage::run`

```rust
fn run(&mut self, input: Self::Input, session: &mut Session<C>)
    -> Result<Self::Output, DriverError>
```

Run the stage: consume `input`, use the [`Session`](#session) as needed, and produce
the output.

**Parameters**

- `input: Self::Input` — the previous phase's artifact, taken by value.
- `session: &mut Session<C>` — the shared session. Read configuration with
  [`config`](#config), record findings with [`error`](#session-error) /
  [`warn`](#warn) / [`note`](#note).

**Returns**

- `Ok(Self::Output)` — the artifact for the next phase.
- `Err(DriverError)` — the phase cannot proceed; the pipeline stops here and stamps
  this stage's name into the error. Return `Err` (or call
  [`abort_if_errors`](#abort_if_errors)) rather than panicking.

**Examples**

A stage that emits a diagnostic and still succeeds — emitting an error *diagnostic*
records a problem but does not by itself stop the pipeline:

```rust
use driver_lang::{DriverError, Session, Stage};

struct Lex;
impl Stage<()> for Lex {
    type Input = &'static str;
    type Output = Vec<i64>;
    fn name(&self) -> &'static str { "lex" }
    fn run(&mut self, input: &'static str, session: &mut Session<()>)
        -> Result<Vec<i64>, DriverError>
    {
        let mut out = Vec::new();
        for word in input.split_whitespace() {
            match word.parse::<i64>() {
                Ok(n) => out.push(n),
                Err(_) => { session.warn("skipping non-integer token"); }
            }
        }
        Ok(out)
    }
}

let mut session = Session::new(());
let tokens = Lex.run("1 two 3", &mut session).unwrap();
assert_eq!(tokens, vec![1, 3]);
assert_eq!(session.diagnostics().len(), 1);
```

A stage that fails outright:

```rust
use driver_lang::{DriverError, Session, Stage};

struct RequireNonEmpty;
impl Stage<()> for RequireNonEmpty {
    type Input = Vec<i64>;
    type Output = Vec<i64>;
    fn name(&self) -> &'static str { "require-non-empty" }
    fn run(&mut self, input: Vec<i64>, _s: &mut Session<()>)
        -> Result<Vec<i64>, DriverError>
    {
        if input.is_empty() {
            return Err(DriverError::new("no input"));
        }
        Ok(input)
    }
}

let mut session = Session::new(());
let err = RequireNonEmpty.run(vec![], &mut session).unwrap_err();
assert_eq!(err.message(), "no input");
```

<br>

<a id="pipeline"></a>
### `Pipeline<C, S>`

```rust
pub struct Pipeline<C, S> { /* private */ }
```

An end-to-end driver: a chain of [`Stage`](#stage)s whose types line up, run against
a [`Session`](#session). Start from one stage with [`new`](#pipeline-new), extend it
with [`then`](#then), and drive it with [`run`](#pipeline-run).

`C` is a phantom parameter — a pipeline threads a `&mut Session<C>` through its
stages but stores no `C` of its own — so it is inferred from the stages and you never
name it. On the first stage that returns a [`DriverError`](#drivererror), the
pipeline stops and the error names the failing stage.

---

<a id="pipeline-new"></a>
#### `Pipeline::new`

```rust
pub fn new(stage: S) -> Pipeline<C, S>
```

Start a pipeline from a single stage.

**Parameters**

- `stage: S` — the first (and, until you call [`then`](#then), only) stage.

**Returns** a `Pipeline<C, S>` wrapping the stage.

**Examples**

```rust
use driver_lang::{DriverError, Pipeline, Session, Stage};

struct Identity;
impl Stage<()> for Identity {
    type Input = i64;
    type Output = i64;
    fn name(&self) -> &'static str { "identity" }
    fn run(&mut self, input: i64, _s: &mut Session<()>) -> Result<i64, DriverError> {
        Ok(input)
    }
}

let mut driver = Pipeline::new(Identity);
let mut session = Session::new(());
assert_eq!(driver.run(42, &mut session).unwrap(), 42);
```

---

<a id="then"></a>
#### `Pipeline::then`

```rust
pub fn then<N>(self, next: N) -> Pipeline<C, Then<S, N>>
where
    N: Stage<C, Input = S::Output>,
```

Append a stage whose [`Input`](#stage) is this pipeline's current
[`Output`](#stage). The bound `N: Stage<C, Input = S::Output>` is the compile-time
check that the phases connect — a stage that does not accept what the pipeline
currently produces is a type error, not a runtime surprise.

**Parameters**

- `next: N` — the stage to run after the current chain. Its `Input` must equal the
  current `Output`, and it must share the configuration type `C`.

**Returns** a longer `Pipeline<C, Then<S, N>>` that produces `N`'s output.

**Examples**

```rust
use driver_lang::{DriverError, Pipeline, Session, Stage};

struct Lex;
impl Stage<()> for Lex {
    type Input = &'static str;
    type Output = Vec<i64>;
    fn name(&self) -> &'static str { "lex" }
    fn run(&mut self, i: &'static str, _s: &mut Session<()>) -> Result<Vec<i64>, DriverError> {
        i.split_whitespace().map(|w| w.parse().map_err(|_| DriverError::new("bad"))).collect()
    }
}
struct Sum;
impl Stage<()> for Sum {
    type Input = Vec<i64>;
    type Output = i64;
    fn name(&self) -> &'static str { "sum" }
    fn run(&mut self, i: Vec<i64>, _s: &mut Session<()>) -> Result<i64, DriverError> {
        Ok(i.iter().sum())
    }
}
struct Negate;
impl Stage<()> for Negate {
    type Input = i64;
    type Output = i64;
    fn name(&self) -> &'static str { "negate" }
    fn run(&mut self, i: i64, _s: &mut Session<()>) -> Result<i64, DriverError> { Ok(-i) }
}

// Chain three phases; each `then` checks the types connect.
let mut driver = Pipeline::new(Lex).then(Sum).then(Negate);
let mut session = Session::new(());
assert_eq!(driver.run("1 2 3", &mut session).unwrap(), -6);
```

---

<a id="pipeline-run"></a>
#### `Pipeline::run`

```rust
pub fn run(&mut self, input: S::Input, session: &mut Session<C>)
    -> Result<S::Output, DriverError>
```

Run the pipeline: thread `input` through every stage in order and return the final
output. Stages run left to right, each receiving the previous stage's output and the
shared session. The run stops at the first stage that returns a
[`DriverError`](#drivererror).

**Parameters**

- `input: S::Input` — the input to the first stage.
- `session: &mut Session<C>` — the session shared by every stage.

**Returns**

- `Ok(S::Output)` — the final artifact.
- `Err(DriverError)` — the first stage that failed, with its name stamped in. Later
  stages do not run; diagnostics emitted before the failure remain in the session.

**Examples**

Success, and a failure attributed to the right stage:

```rust
use driver_lang::{DriverError, Pipeline, Session, Stage};

struct Lex;
impl Stage<()> for Lex {
    type Input = &'static str;
    type Output = Vec<i64>;
    fn name(&self) -> &'static str { "lex" }
    fn run(&mut self, i: &'static str, _s: &mut Session<()>) -> Result<Vec<i64>, DriverError> {
        i.split_whitespace()
            .map(|w| w.parse().map_err(|_| DriverError::new("not an integer")))
            .collect()
    }
}
struct Sum;
impl Stage<()> for Sum {
    type Input = Vec<i64>;
    type Output = i64;
    fn name(&self) -> &'static str { "sum" }
    fn run(&mut self, i: Vec<i64>, _s: &mut Session<()>) -> Result<i64, DriverError> {
        Ok(i.iter().sum())
    }
}

let mut driver = Pipeline::new(Lex).then(Sum);
let mut session = Session::new(());

assert_eq!(driver.run("4 5 6", &mut session).unwrap(), 15);

let err = driver.run("4 oops 6", &mut session).unwrap_err();
assert_eq!(err.stage(), "lex"); // the parse failed in `lex`, before `sum`
```

---

<a id="pipeline-name"></a>
#### `Pipeline::name`

```rust
pub fn name(&self) -> &'static str
```

The name of the final stage — the stage whose output this pipeline produces.

**Returns** the last stage's [`name`](#stage-name).

**Examples**

```rust
use driver_lang::{DriverError, Pipeline, Session, Stage};

struct A;
impl Stage<()> for A {
    type Input = ();
    type Output = ();
    fn name(&self) -> &'static str { "a" }
    fn run(&mut self, _i: (), _s: &mut Session<()>) -> Result<(), DriverError> { Ok(()) }
}
struct B;
impl Stage<()> for B {
    type Input = ();
    type Output = ();
    fn name(&self) -> &'static str { "b" }
    fn run(&mut self, _i: (), _s: &mut Session<()>) -> Result<(), DriverError> { Ok(()) }
}

let driver = Pipeline::new(A).then(B);
assert_eq!(driver.name(), "b");
```

---

<a id="then-type"></a>
#### `Then<A, B>`

```rust
pub struct Then<A, B> { /* private */ }
```

Two stages run back to back — the node [`Pipeline::then`](#then) builds. `Then<A, B>`
is itself a [`Stage`](#stage): it feeds an input through `A`, then feeds `A`'s output
through `B`, and its `Input` / `Output` are `A::Input` / `B::Output`. You rarely name
it directly — a three-stage pipeline has type `Pipeline<C, Then<Then<A, B>, C2>>`,
assembled for you by chaining `.then(...)`. It is public only because it appears in
those inferred types. Composition is monomorphized: each stage is a direct,
inlinable call with no boxing.

<br>

<a id="diagnostic"></a>
### `Diagnostic`

```rust
pub struct Diagnostic { /* private */ }

impl Diagnostic {
    pub fn new(severity: Severity, message: impl Into<Cow<'static, str>>) -> Diagnostic
    pub fn error(message: impl Into<Cow<'static, str>>) -> Diagnostic
    pub fn warning(message: impl Into<Cow<'static, str>>) -> Diagnostic
    pub fn note(message: impl Into<Cow<'static, str>>) -> Diagnostic
    pub fn severity(&self) -> Severity
    pub fn message(&self) -> &str
    pub fn is_error(&self) -> bool
}
```

One message a stage reports about the program under compilation: a
[`Severity`](#severity) paired with text. Stages create diagnostics and hand them to
the [`Session`](#session) with [`emit`](#emit) (or the [`error`](#session-error) /
[`warn`](#warn) / [`note`](#note) shorthands).

The message is a `Cow<'static, str>`: a constant message costs no allocation, a
computed one is owned only when built. The type is deliberately small — a severity
and a message. It does not model spans or error codes; a language plugs a richer
diagnostics representation in as its own concern.

- `new` builds a diagnostic with an explicit severity; `error` / `warning` / `note`
  are shorthands.
- `severity` and `message` read the parts; `is_error` is `severity().is_error()`.
- `Display` renders as `severity: message` (e.g. `error: unexpected token`).
- Derives `Debug`, `Clone`, `PartialEq`, `Eq`, and — behind the `serde` feature —
  `Serialize`.

**Parameters** (constructors)

- `severity: Severity` (`new` only) — the seriousness.
- `message: impl Into<Cow<'static, str>>` — a string literal (no allocation) or an
  owned `String`.

**Examples**

```rust
use driver_lang::{Diagnostic, Severity};

let d = Diagnostic::error("unexpected `}`");
assert_eq!(d.severity(), Severity::Error);
assert_eq!(d.message(), "unexpected `}`");
assert!(d.is_error());
assert_eq!(d.to_string(), "error: unexpected `}`");

// `new` with an explicit, possibly computed, severity.
let level = 1;
let severity = if level == 0 { Severity::Error } else { Severity::Warning };
let d = Diagnostic::new(severity, format!("issue at level {level}"));
assert!(!d.is_error());
```

<br>

<a id="severity"></a>
### `Severity`

```rust
pub enum Severity { Error, Warning, Note }

impl Severity {
    pub fn is_error(self) -> bool
    pub fn as_str(self) -> &'static str
}
```

The seriousness of a [`Diagnostic`](#diagnostic). Only `Error` counts toward
[`Session::error_count`](#error_count); a `Warning` or `Note` is recorded and
rendered but never trips [`abort_if_errors`](#abort_if_errors).

- `is_error` is `true` only for `Error`.
- `as_str` is the lowercase label (`"error"`, `"warning"`, `"note"`); `Display` uses
  it.
- Derives `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, and — behind the
  `serde` feature — `Serialize` (rendered lowercase). It does **not** derive `Ord`:
  compare with `is_error`, not a numeric rank.

**Examples**

```rust
use driver_lang::Severity;

assert!(Severity::Error.is_error());
assert!(!Severity::Warning.is_error());
assert_eq!(Severity::Note.as_str(), "note");
assert_eq!(Severity::Warning.to_string(), "warning");
```

<br>

<a id="drivererror"></a>
### `DriverError`

```rust
pub struct DriverError { /* private */ }

impl DriverError {
    pub fn new(message: impl Into<Cow<'static, str>>) -> DriverError
    pub fn stage(&self) -> &str
    pub fn message(&self) -> &str
}
```

The error that stops a compilation run. A [`Stage`](#stage) returns one from
[`run`](#stage-run) when it cannot produce its output; it is the *control-flow*
failure that ends the pipeline, distinct from a [`Diagnostic`](#diagnostic) (a record
a stage emits and may keep going after).

A stage does not name itself: it returns `DriverError::new(reason)` with an empty
stage, and the [`Pipeline`](#pipeline) stamps in the failing stage's name as it
propagates. Stamping fills only an empty name, so in a chain the innermost stage —
the one that actually failed — keeps the attribution.

- `new` builds an error with an empty stage; `stage` and `message` read the parts.
- `Display` renders as `stage \`name\` failed: message`, or `driver error: message`
  before a stage name is stamped in.
- Implements `std::error::Error`; derives `Debug`, `Clone`, `PartialEq`, `Eq`.

**Parameters**

- `message: impl Into<Cow<'static, str>>` — a string literal (no allocation) or an
  owned `String`.

**Examples**

```rust
use driver_lang::DriverError;

// A static reason.
let err = DriverError::new("unresolved import");
assert_eq!(err.message(), "unresolved import");
assert_eq!(err.stage(), ""); // empty until it flows through a Pipeline

// A computed reason.
let path = "src/main";
let err = DriverError::new(format!("cannot read `{path}.rs`"));
assert_eq!(err.message(), "cannot read `src/main.rs`");
assert_eq!(err.to_string(), "driver error: cannot read `src/main.rs`");
```

<br>

## Feature flags

| Feature | Default | Description                                                                 |
|---------|:-------:|-----------------------------------------------------------------------------|
| `std`   | ✅      | Links the standard library. With it off the crate is `no_std` + `alloc`; every type works in both modes. |
| `serde` | ⬜      | Derives `serde::Serialize` for [`Severity`](#severity) and [`Diagnostic`](#diagnostic). |

Feature flags are additive: enabling `serde` never changes existing behavior, and
disabling `std` removes no type or method — it only switches the crate to `no_std`.

<br>

## Design notes

**A driver, not a pass manager.** A pass rewrites one type in place; a stage changes
the type as the artifact moves down the pipeline. driver-lang manages that
type-threading, the shared session, and the clean stop — the concerns unique to
driving a whole compile. IR-level, same-type transforms belong in a pass manager
([`pass-lang`](https://crates.io/crates/pass-lang)) that a stage can run internally.

**No first-party dependencies.** The crate is generic over the artifacts that flow
through it and the configuration the session carries. It does not wire a lexer, a
parser, or a specific diagnostics crate — a language plugs those in as stages and as
the session's `C`. This keeps the driver a thin orchestrator with nothing to force on
a consumer, and keeps its dependency surface empty but for optional `serde`.

**Single-threaded by design.** A compile pipeline is an inherently ordered sequence
of phases, each consuming the last one's output, so the driver carries no atomic
overhead. Parallelism, where a language wants it, lives inside a stage (parsing many
files at once, say), not in the driver that sequences the phases.

**Diagnostics are policy-free.** The session records diagnostics and counts errors;
it never decides on its own to stop. A stage decides, by returning a
[`DriverError`](#drivererror) or calling [`abort_if_errors`](#abort_if_errors). This
keeps mechanism (recording) separate from policy (when a run is doomed), so a
language can collect as many errors as it wants before halting.

<br>

---

<div align="center">
    <sup>
        <a href="../README.md" title="Project Home"><b>HOME</b></a>
        <span>&nbsp;│&nbsp;</span>
        <span>API</span>
        <span>&nbsp;│&nbsp;</span>
        <a href="../CHANGELOG.md" title="Changelog"><b>CHANGELOG</b></a>
    </sup>
    <br><br>
    <sub>Copyright &copy; 2026 <strong>James Gober</strong>.</sub>
</div>
