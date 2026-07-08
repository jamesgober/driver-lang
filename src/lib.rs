//! # driver_lang
//!
//! The driver that ties a compiler's phases into one run.
//!
//! A compiler is a sequence of phases — lex the source, parse the tokens, resolve
//! names, check types, lower to an intermediate representation, emit code. Each
//! phase takes the previous phase's artifact and produces the next. The machinery
//! that holds those phases together — carrying the shared configuration and
//! diagnostics from the first phase to the last, threading each artifact into the
//! next phase, and stopping cleanly when a phase fails — is the same regardless of
//! *what* language is being compiled. driver-lang is that machinery, and nothing
//! more.
//!
//! It contributes three pieces:
//!
//! - A [`Session`] — the ambient state of one run. It holds the compilation
//!   configuration (`C`, whatever a language needs) and the [`Diagnostic`]s every
//!   phase emits, and it tracks how many were errors so a driver can decide when to
//!   stop with [`Session::abort_if_errors`].
//! - A [`Stage`] — one phase, as a transform from an input artifact to an output
//!   artifact run against the session. Distinct from a *pass* (which rewrites one
//!   type in place), a stage *changes the type* as the program moves down the
//!   pipeline.
//! - A [`Pipeline`] — stages composed end to end, with each stage's output checked
//!   at compile time against the next stage's input. Running it threads an input
//!   through every phase and returns the final artifact, stopping at the first
//!   failure.
//!
//! The crate owns no compiler phases and wires no first-party dependency. It is
//! generic over the artifacts that flow through it and the configuration the
//! session carries, so a language plugs its own lexer, parser, and backend in as
//! stages. The design mirrors how a real compiler's driver is a thin orchestrator
//! over phases it does not itself implement.
//!
//! ## Example
//!
//! A three-phase calculator driver: tokenize a string into integers, sum them,
//! then negate — collecting a diagnostic along the way.
//!
//! ```
//! use driver_lang::{DriverError, Pipeline, Session, Stage};
//!
//! // Phase 1: text -> integers. Warns on tokens it skips; fails on an empty result.
//! struct Lex;
//! impl Stage<()> for Lex {
//!     type Input = &'static str;
//!     type Output = Vec<i64>;
//!     fn name(&self) -> &'static str { "lex" }
//!     fn run(&mut self, input: &'static str, session: &mut Session<()>)
//!         -> Result<Vec<i64>, DriverError>
//!     {
//!         let mut out = Vec::new();
//!         for word in input.split_whitespace() {
//!             match word.parse::<i64>() {
//!                 Ok(n) => out.push(n),
//!                 Err(_) => { session.warn("skipping non-integer token"); }
//!             }
//!         }
//!         if out.is_empty() {
//!             return Err(DriverError::new("nothing to compute"));
//!         }
//!         Ok(out)
//!     }
//! }
//!
//! // Phase 2: integers -> their sum.
//! struct Sum;
//! impl Stage<()> for Sum {
//!     type Input = Vec<i64>;
//!     type Output = i64;
//!     fn name(&self) -> &'static str { "sum" }
//!     fn run(&mut self, input: Vec<i64>, _session: &mut Session<()>)
//!         -> Result<i64, DriverError>
//!     { Ok(input.iter().sum()) }
//! }
//!
//! // Phase 3: negate.
//! struct Negate;
//! impl Stage<()> for Negate {
//!     type Input = i64;
//!     type Output = i64;
//!     fn name(&self) -> &'static str { "negate" }
//!     fn run(&mut self, input: i64, _session: &mut Session<()>)
//!         -> Result<i64, DriverError>
//!     { Ok(-input) }
//! }
//!
//! let mut driver = Pipeline::new(Lex).then(Sum).then(Negate);
//! let mut session = Session::new(());
//!
//! let result = driver.run("1 two 3", &mut session).unwrap();
//! assert_eq!(result, -4);                    // -(1 + 3)
//! assert_eq!(session.diagnostics().len(), 1); // the "two" warning
//! ```
//!
//! ## Features
//!
//! - `std` (default) — link the standard library. Without it the crate is
//!   `#![no_std]` and needs only `alloc`; every type here works in both modes.
//! - `serde` — derive `serde::Serialize` for [`Severity`] and [`Diagnostic`], so
//!   a run's diagnostics can be serialized for tooling.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

extern crate alloc;

mod diagnostic;
mod error;
mod pipeline;
mod session;
mod severity;
mod stage;

pub use diagnostic::Diagnostic;
pub use error::DriverError;
pub use pipeline::{Pipeline, Then};
pub use session::Session;
pub use severity::Severity;
pub use stage::Stage;
