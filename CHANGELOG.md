<h1 align="center">
    <img width="90px" height="auto" src="https://raw.githubusercontent.com/jamesgober/jamesgober/main/media/icons/hexagon-3.svg" alt="Triple Hexagon">
    <br><b>CHANGELOG</b>
</h1>
<p>
  All notable changes to <code>driver-lang</code> will be documented in this file. The format is based on <a href="https://keepachangelog.com/en/1.1.0/">Keep a Changelog</a>,
  and this project adheres to <a href="https://semver.org/spec/v2.0.0.html/">Semantic Versioning</a>.
</p>

---

## [Unreleased]

### Added

### Changed

### Fixed

### Security

---

## [1.0.0] - 2026-07-07

API freeze. The surface introduced in `0.2.0` is now stable under Semantic Versioning and will not change in a breaking way within the `1.x` series. No functional changes from `0.2.0`; this release records the promise.

### Changed

- `docs/API.md` marks the public API stable and documents the SemVer promise (see its [Stability](./docs/API.md#stability) section): the run-order and error-counting behaviors are part of the contract, and `1.x` releases are additive only.
- `README.md` status is now **stable**; installation examples pin `driver-lang = "1"`.

---

## [0.2.0] - 2026-07-01

The core lands: a compiler driver built from a session, stages, and a type-checked pipeline. This is the first release with domain logic — the public surface is designed here and will be frozen at `1.0.0`.

### Added

- `Session<C>` — the ambient state of one run: the compilation configuration `C` and the diagnostics every stage emits. Emit with `emit` / `error` / `warn` / `note`; read with `diagnostics`, `error_count`, `has_errors`; drain with `take_diagnostics`; check a phase boundary with `abort_if_errors`; reach the config with `config` / `config_mut` / `into_config`. Error counting is incremental (O(1) checkpoints).
- `Stage<C>` — one compilation phase, as a transform from an `Input` artifact to an `Output` artifact run against the session, plus a stable `name` for error attribution.
- `Pipeline<C, S>` — stages composed end to end. `then` appends a stage whose input matches the current output (checked at compile time), `run` threads an input through the whole chain, and `name` reports the final stage. Fully monomorphized — no dynamic dispatch on the run path. `Then<A, B>` is the composition node.
- `Diagnostic` and `Severity` (`Error` / `Warning` / `Note`) — one message and its seriousness, with `Cow<'static, str>` messages and a `severity: message` `Display`.
- `DriverError` — the failure that halts a run, carrying a message and the name of the stage the pipeline stamps in.
- `serde` feature — derives `serde::Serialize` for `Severity` and `Diagnostic`.
- `examples/calculator.rs` — a three-stage driver (lex, cap-check, sum) that renders its diagnostics.
- Criterion benchmarks (`benches/bench.rs`), property tests (`tests/proptests.rs`), and integration tests (`tests/driver.rs`).
- `docs/API.md` — full reference for every public item, with parameters and examples.

### Fixed

- `Cargo.toml` `keywords` and `categories` were unquoted, so the manifest failed to parse; both are now quoted string arrays.
- `clippy.toml` `msrv` was `1.87`, ahead of the crate's declared MSRV; aligned to `1.85`.

---

## [0.1.0] - 2026-06-18

Initial scaffold and repository bootstrap. No domain logic yet &mdash; this release establishes the structure, tooling, and quality gates the implementation will be built on.

### Added

- `Cargo.toml` with crate metadata, Rust 2024 edition, MSRV 1.85.
- Dual `Apache-2.0 OR MIT` license files.
- `README.md`, `CHANGELOG.md`, and a documentation skeleton.
- `REPS.md` compliance baseline.
- `.github/workflows/ci.yml` CI matrix; `deny.toml`, `clippy.toml`, `rustfmt.toml`.
- `dev/DIRECTIVES.md` and `dev/ROADMAP.md` (committed engineering standards + plan).

[Unreleased]: https://github.com/jamesgober/driver-lang/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/jamesgober/driver-lang/compare/v0.2.0...v1.0.0
[0.2.0]: https://github.com/jamesgober/driver-lang/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/jamesgober/driver-lang/releases/tag/v0.1.0
