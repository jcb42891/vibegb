# VibeGB Test Plan

Last Updated: 2026-02-18

## 1. Goals

- Define exact validation layers for emulator development.
- Make unit testing mandatory for all subsystem changes.
- Keep milestone gates objective and repeatable.
- Include Pokemon Red smoke checks without distributing ROM data.

## 2. Test Layers

## 2.1 Unit Tests (Required Every PR)

- Run command: `cargo test`
- Scope:
- CPU opcode semantics and cycle checks
- MMU routing and memory behaviors
- Timer and interrupt edge cases
- PPU mode transitions and register behavior
- MBC bank switching and save RAM logic
- APU frame-sequencer and register behaviors

Policy:
- New behavior requires new unit tests.
- Bug fixes require a regression test.

## 2.2 ROM Conformance Tests

- Run through `crates/runner` in headless mode.
- Suites:
- blargg
- mooneye (selected subsets)
- `dmg-acid2`
- `cgb-acid2`
- mealybug-tearoom-tests (selected subsets)
- SameSuite (selected subsets)

## 2.3 Commercial Smoke Tests (Pokemon Red)

- ROM source: user-provided local file path.
- Not committed to repository.
- Used for milestone smoke criteria:
- M0: header load and metadata parse
- M2: reaches title screen and accepts input
- M4: save/load flow works end-to-end

## 3. CI Requirements

- CI matrix runs on Windows/Linux/macOS.
- CI stages:
- `cargo fmt --check`
- `cargo clippy --workspace -- -D warnings`
- `cargo test`
- milestone-appropriate ROM test subset

- No regressions in milestone `must-pass` tests.

## 4. Local Validation Loop

Run this before opening or updating a PR:

1. `cargo test`
2. targeted ROM tests related to changed subsystem
3. full milestone gate subset
4. Pokemon Red smoke check relevant to active milestone

## 5. Failure Handling

- Any unit-test failure blocks merge.
- Any `must-pass` ROM regression blocks milestone completion.
- `should-pass` failures require an issue with owner and next action.
- `known-fail` tests must be listed with rationale.

## 6. Reporting Format

For milestone reviews, record:

- Date
- Commit/branch
- Unit test status
- ROM gate status
- Pokemon Red smoke status
- New regressions
- Linked issues

