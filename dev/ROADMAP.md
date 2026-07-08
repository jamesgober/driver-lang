# driver-lang - Roadmap

> Path from scaffold to a stable 1.0. Hard parts are front-loaded; each phase has hard exit criteria.
> Master plan: ../../_strategy/LANG_COLLECTION.md
>
> **Anti-deferral rule:** no listed hard task moves to a later phase unless this file records the move and the reason.

## v0.1.0 - Scaffold (DONE)
Compiles, CI green, structure correct, no domain logic.
- [x] Manifest, README, CHANGELOG, REPS, dual license, CI, deny, clippy, rustfmt.

## v0.2.0 - Core (THE HARD PART, NOT DEFERRED) (DONE)
Compiler driver/session orchestration tying the pipeline together.
Shipped as a generic, dependency-light driver: `Session`, `Stage`, `Pipeline`,
`Diagnostic`, `Severity`, `DriverError`. No first-party dependency was wired — the
driver is generic over the artifacts and configuration it threads, so a language
plugs its lexer/parser/backend in as stages (the same reasoning that kept
`pass-lang` dependency-free). Optional `serde` only.
Exit criteria:
- [x] Every public item has rustdoc + a runnable example.
- [x] Core invariants property-tested (full API authored at this stage).

## v1.0.0 - API freeze (DONE)
Public surface stable and frozen until 2.0.
- [x] docs/API.md marked stable; SemVer promise recorded.
- [x] Full test + benchmark suite green (Windows + WSL2 locally; macOS via CI matrix).
