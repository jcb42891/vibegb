# VibeGB Roadmap

Last Updated: 2026-02-18  
Status Legend: `todo`, `doing`, `blocked`, `done`

## 1. Planning Principles

- Build in thin vertical slices with measurable gates.
- Ship cross-platform support from day one (Windows, Linux, macOS).
- Keep emulator core independent from desktop shell implementation details.
- Treat unit tests as mandatory, not optional.
- Do not advance milestones with unresolved `must-pass` regressions.

## 2. Core Stack Commitments

- Language: Rust (stable)
- Frontend: Tauri desktop shell
- Workspace shape:
- `crates/core`
- `crates/runner`
- `apps/desktop`

## 3. Milestone Overview

## M0 - Workspace, Tooling, and Validation Harness

Objective:
- Establish Rust workspace, Tauri shell scaffolding, and validation loop plumbing.

Deliverables:
- Cargo workspace and crate layout (`core`, `runner`, desktop app)
- Tauri app shell builds on Windows/Linux/macOS
- Basic ROM loader and header parser
- Headless runner CLI with configurable ROM path
- CI matrix for Windows/Linux/macOS
- Unit test harness wired into CI

Exit Gates (`must-pass`):
- `cargo test` passes for current unit tests.
- CI matrix builds all three platforms.
- Runner loads Pokemon Red header from local ROM path without crash.

Status: `todo`

## M1 - CPU, Interrupts, Timers (Deterministic Core)

Objective:
- Build deterministic instruction execution foundation.

Deliverables:
- Full opcode decode/execute (`base` + `cb`)
- Accurate flags and cycle counts
- Interrupt controller and IME behavior
- Timer and DIV/TIMA/TMA/TAC behavior
- HALT/STOP baseline behavior
- Unit tests for opcode groups and timing-critical logic

Exit Gates (`must-pass`):
- CPU/timer unit-test suites pass.
- blargg CPU instruction target subset passes.
- mooneye timer/interrupt subset passes.

Status: `todo`

## M2 - DMG PPU Baseline + First Playable Smoke

Objective:
- Render correct DMG frames and achieve first real-game playability checks.

Deliverables:
- PPU mode timing (`OAM`, `transfer`, `HBlank`, `VBlank`)
- Background/window rendering
- Sprite rendering with priority rules
- STAT/LY/LYC behavior
- PPU unit tests for mode transitions and key register behavior

Exit Gates (`must-pass`):
- PPU unit-test suites pass.
- `dmg-acid2` passes.
- selected mealybug DMG tests pass.
- Pokemon Red reaches title screen and accepts start/input flow.

Status: `todo`

## M3 - CGB Features

Objective:
- Add Game Boy Color hardware behavior on top of stable baseline.

Deliverables:
- CGB VRAM banking and tile attributes
- CGB palettes and color fetch path
- CGB double-speed mode
- HDMA/GDMA behavior
- CGB-specific register handling
- Unit tests for CGB register and banking behavior

Exit Gates (`must-pass`):
- CGB unit-test suites pass.
- `cgb-acid2` passes.
- selected CGB mooneye tests pass.

Status: `todo`

## M4 - Cartridge Expansion, Saves, and Pokemon Red Progression

Objective:
- Support common production cartridges and persistent saves.

Deliverables:
- MBC1, MBC3, MBC5 production-ready
- RAM enable and banking correctness
- Battery-backed save RAM persistence
- MBC3 RTC baseline support
- Unit tests for mapper bank switching and save behavior

Exit Gates (`must-pass`):
- Mapper unit tests pass.
- mapper-specific ROM suite subset passes.
- Pokemon Red save file create/load flow works end-to-end.

Status: `todo`

## M5 - APU and AV Sync

Objective:
- Reach practical playable quality with stable audio.

Deliverables:
- Channel emulation and frame sequencer
- Mixer and output resampling strategy
- Audio/video synchronization under normal load
- Unit tests for frame sequencer timing and key register behavior

Exit Gates (`must-pass`):
- APU unit tests pass.
- selected APU behavior tests pass.
- No persistent crackle/drift in manual acceptance sessions.

Status: `todo`

## M6 - Compatibility, Performance, and Release Hardening

Objective:
- Improve stability and compatibility for real game workloads.

Deliverables:
- Compatibility matrix and regression workflow
- Performance profiling and targeted optimizations
- Debug tooling improvements (trace, breakpoints, diagnostics)
- Packaging and release-candidate workflow for all desktop platforms

Exit Gates (`must-pass`):
- Agreed compatibility threshold met for curated game list.
- Cross-platform release artifacts produced.
- No open P0/P1 regressions in in-scope features.

Status: `todo`

## 4. Validation Loop (Always-On)

Each feature branch must run this loop:

1. Add unit tests first for new behavior.
2. Implement feature increment.
3. Run local fast checks:
- `cargo test`
- targeted ROM tests via `crates/runner`
4. Run milestone-specific gate ROMs.
5. Run Pokemon Red smoke check for current milestone.
6. Push only when local loop passes; CI must confirm on all platforms.

## 5. Task Ledger Format

Track tasks in this file (or `TASKS.md`) with one-line entries:

- `[status] [milestone] [subsystem] short task description | verification`

Examples:
- `[todo] [M1] [cpu] Implement DAA behavior | cpu unit test + blargg cpu_instrs 01`
- `[doing] [M2] [ppu] Sprite priority in mode 3 | ppu unit test + mealybug case`
- `[done] [M0] [infra] Add CI matrix build | GitHub Actions win/linux/macos`

## 6. Test Gate Matrix (Initial Draft)

- M0:
- workspace unit tests
- CI matrix build
- Pokemon Red header load smoke

- M1:
- CPU/timer/interrupt unit tests
- blargg CPU subset
- mooneye timer/interrupt subset

- M2:
- PPU unit tests
- `dmg-acid2`
- selected mealybug DMG tests
- Pokemon Red title/input smoke

- M3:
- CGB unit tests
- `cgb-acid2`
- selected mooneye CGB tests

- M4:
- mapper unit tests
- mapper ROM suite subset
- Pokemon Red save/load smoke

- M5:
- APU unit tests
- selected APU checks
- manual AV sync validation

- M6:
- compatibility threshold
- performance budget
- release artifact validation

## 7. Execution Cadence

- Update task status continuously during implementation.
- At milestone close:
- record pass/fail evidence for each gate
- link failures to tracked issues
- do not move forward until all `must-pass` gates are green

## 8. Immediate Next Planning Actions

- Draft `TEST_PLAN.md` with exact ROM list and run commands.
- Create initial M0 backlog items with owners and estimates.
- Set up CI matrix and baseline unit-test command wiring.

