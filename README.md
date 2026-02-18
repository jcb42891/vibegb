# VibeGB

Cross-platform Game Boy / Game Boy Color emulator workspace.

## Workspace Layout

- `crates/core`: emulator core building blocks and ROM/header parsing.
- `crates/runner`: headless CLI runner for ROM validation flows.
- `apps/desktop`: Tauri desktop shell scaffolding.

## Baseline Commands

- Run all tests: `cargo test --workspace --all-targets`
- Lint with clippy: `cargo clippy --workspace --all-targets -- -D warnings`
- Check formatting: `cargo fmt --all -- --check`
- Load and print Pokemon Red header:
  - `cargo run -p vibegb-runner -- --rom "Pokemon - Red Version (USA, Europe) (SGB Enhanced).gb"`
- Run a conformance ROM in execution mode with serial expectation (M1 harness):
  - `cargo run -p vibegb-runner -- --rom "<path-to-test-rom.gb>" --mode exec --max-steps 2000000 --expect-serial "Passed"`
- Run a mooneye-style pass-signature check for a single ROM:
  - `cargo run -p vibegb-runner -- --rom "<path-to-mooneye-rom.gb>" --mode exec --max-steps 2000000 --expect-mooneye-pass`
- Run the M1 subset suite via manifest:
  - `cargo run -p vibegb-runner -- --suite "tools/rom-suites/m1-subset.template.txt" --rom-root "<path-to-local-rom-suite-root>" --max-steps 2000000`

## Docs

- Spec: `SPEC.md`
- Roadmap: `ROADMAP.md`
- Test plan: `TEST_PLAN.md`
- Task ledger: `TASKS.md`
