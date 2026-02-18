# VibeGB Implementation Spec

Status: Draft  
Last Updated: 2026-02-18  
Audience: Emulator core and tooling contributors

## 1. Purpose

Define the scope, architecture, and quality bar for a cross-platform Game Boy Color emulator built in Rust.

This is a living spec. If implementation and spec diverge, update this file in the same change set.

## 2. Product Scope

## 2.1 Goals

- Run commercial Game Boy and Game Boy Color games on PC with high compatibility.
- Ship cross-platform desktop support from the start: Windows, Linux, macOS.
- Keep emulation deterministic for repeatable automated validation.
- Make unit testing and ROM-based regression testing part of daily development.
- Support both windowed gameplay and headless test execution.

## 2.2 Non-Goals (Initial Releases)

- Netplay
- Rewind/rollback
- RetroAchievements integration
- Mobile builds
- Shader-heavy post-processing
- Cycle-perfect modeling of every silicon revision

## 2.3 Compatibility Target

`v1.0` target:
- Pass selected CPU/timing/PPU/CGB test suites (see Section 9).
- Boot and be playable for a curated compatibility list of commercial titles.
- Include Pokemon Red as a mandatory smoke ROM in the validation loop.
- No known hard crashes on supported ROM types and MBCs in scope.

## 3. Technology and Platform Decisions

## 3.1 Language and Toolchain

- Rust stable toolchain.
- Cargo workspace with isolated crates for core, frontend shell, and headless runner.

## 3.2 Frontend Stack

- Tauri desktop shell for cross-platform packaging and host integration.
- Emulation core remains frontend-agnostic and reusable outside Tauri.

## 3.3 Platform Support

- Tier 1 from project start:
- Windows
- Linux
- macOS

- CI must build and test on all Tier 1 platforms.

## 4. Architecture

## 4.1 Workspace Layout

- `crates/core/`
- CPU, MMU, PPU, APU, timers, interrupt controller, DMA, cartridge/MBC
- `crates/runner/`
- Headless ROM runner and test harness integration
- `apps/desktop/`
- Tauri application for window/input/audio/video
- `tools/`
- Trace utilities, test orchestration helpers

## 4.2 Core Design Rules

- Single source of truth for emulated memory reads/writes (MMU).
- Components advance using a shared timing contract (T-cycles baseline).
- No direct cross-module register mutation outside typed interfaces.
- Deterministic stepping APIs:
- step instruction
- step cycles
- step frame

## 4.3 Data Flow

- Frontend loads ROM and optional boot ROM and constructs emulator instance.
- Main loop:
- poll input
- execute emulation until frame boundary
- submit framebuffer
- stream audio samples
- persist save RAM when dirty

## 5. Subsystem Specifications

## 5.1 CPU (LR35902)

- Full base opcode table and CB-prefixed table.
- Correct flag behavior for arithmetic/logic/bit ops.
- Correct instruction length and cycle behavior, including conditional branches.
- Correct `HALT`, `STOP`, interrupt enable/disable timing, and `IME` transitions.

Acceptance:
- Pass blargg CPU instruction tests and interrupt-focused suites in scope.

## 5.2 Memory and Boot Process

- Full memory map routing for ROM/RAM/VRAM/OAM/HRAM/IO.
- Boot ROM mapping and unmapping behavior.
- Open bus and unmapped access behavior documented where relevant.

Acceptance:
- Boot sequence behavior aligns with selected boot mode (boot ROM or fast boot).

## 5.3 Timer and Interrupts

- Divider (`DIV`) and timer (`TIMA/TMA/TAC`) behavior including edge cases.
- Interrupt request/acknowledge flow with priority and vectoring.
- Correct interaction with halted CPU state.

Acceptance:
- Pass timer/interrupt tests from mooneye subset in roadmap gates.

## 5.4 PPU

- DMG-compatible rendering first (modes, LY/LYC, STAT, VBlank).
- Sprite evaluation and priority behavior per test requirements.
- CGB extensions: VRAM banks, tile attributes, palette RAM, priority rules.

Acceptance:
- Pass `dmg-acid2`, then `cgb-acid2`, then selected mealybug tests.

## 5.5 DMA

- OAM DMA timing/behavior.
- CGB HDMA and GDMA with correct register semantics.

Acceptance:
- Pass CGB DMA-related tests in the selected suite.

## 5.6 Cartridge and MBC

Initial scope:
- ROM-only
- MBC1
- MBC3 (RTC baseline)
- MBC5

Requirements:
- Header parsing and validation.
- Banking rules and RAM enable behavior per mapper.
- Battery-backed RAM persistence.

Acceptance:
- Mapper-specific tests pass and commercial ROM samples boot.

## 5.7 APU

- Four channels, frame sequencer, register behavior, and mixing path.
- Audio sync strategy that avoids drift and stutter.

Acceptance:
- Pass selected APU checks; known audible differences documented.

## 5.8 Input, Serial, and Misc

- Joypad register behavior and host input mapping.
- Serial may start as stub unless required by test coverage.

## 6. User-Facing Features (Initial)

- Load ROM from CLI or UI.
- Reset, pause, frame-step (debug mode), speed toggle.
- Save RAM persistence.
- Optional boot ROM usage toggle.

## 7. Debuggability and Observability

- Instruction trace logger (toggleable).
- Breakpoints/watchpoints in debug builds.
- Deterministic trace capture for failing ROM tests.
- Optional on-screen debug overlay (FPS, frame time, mode).

## 8. Unit Testing Policy

- Every subsystem change requires unit tests in the same PR.
- New opcode implementations require opcode-specific tests for:
- flags
- cycles
- register and memory side effects

- Bug fixes require a regression test.
- Unit tests run locally before ROM suites and run in CI for every PR.

Minimum unit-test targets:
- CPU decode and execution semantics
- Timer edge cases
- Interrupt priority and servicing
- MMU region routing and mirror behavior
- PPU mode transitions
- MBC bank switching and RAM enable logic

## 9. Verification Strategy

## 9.1 Test Types

- Unit tests (`cargo test`)
- ROM conformance suites
- Commercial smoke tests (Pokemon Red)
- Cross-platform build and test matrix in CI

## 9.2 ROM Suites

- blargg test ROMs
- mooneye-test-suite (selected gates)
- `dmg-acid2`
- `cgb-acid2`
- mealybug-tearoom-tests (selected gates)
- SameSuite (selected gates)

## 9.3 Commercial Smoke ROM Policy

- Pokemon Red is a required smoke ROM for iterative validation.
- We do not commit proprietary ROM data; tests reference user-provided ROM paths.
- Smoke checks are milestone-scoped:
- boot progression checks early
- title-screen/input checks after PPU baseline
- save/load checks once MBC/save flow is in scope

## 9.4 Validation Loop (Developer Workflow)

Each implementation PR follows this loop:

1. Add or update unit tests for changed subsystem/opcodes.
2. Run fast local loop:
- `cargo test -p core`
- targeted runner invocation for relevant ROM tests
3. Run milestone gate subset locally via headless runner.
4. Run Pokemon Red smoke scenario relevant to current milestone.
5. Push only when local loop passes; CI re-validates on all platforms.

## 9.5 Done Criteria by Test Quality

- `must-pass`: blocks milestone completion.
- `should-pass`: does not block but requires tracked issue.
- `known-fail`: documented with reason and owner.

## 9.6 CI Policy

- Every PR runs:
- unit tests
- milestone-appropriate ROM regression subset
- platform matrix build checks (Windows/Linux/macOS)

- No new `must-pass` regressions allowed.
- CPU/timer failures include trace artifacts where possible.

## 10. Milestone Exit Criteria Contract

A milestone is complete only when:

- Deliverables in `ROADMAP.md` are implemented.
- All milestone `must-pass` tests are green in CI.
- Pokemon Red smoke check for that milestone passes.
- Compatibility matrix updated for newly in-scope features.
- Major deviations from spec are logged in Decision Log.

## 11. Risk Register

- Timing edge cases (`HALT` bug, timer glitches) causing broad compatibility failures.
- PPU sprite/priority corner cases causing visual regressions.
- APU complexity delaying release; mitigated by phased gates.
- Cross-platform frontend/audio differences introducing non-deterministic behavior.
- Scope creep before baseline compatibility stabilizes.

## 12. Decision Log Template

- Date:
- Decision:
- Context:
- Options Considered:
- Chosen Option:
- Consequences:

## 13. Progress Tracking Conventions

- Each task maps to:
- subsystem tag (`cpu`, `ppu`, `apu`, `mbc`, `infra`, `frontend`)
- milestone id (`M0`, `M1`, ...)
- verification artifact (unit test, ROM test, smoke check)

- Suggested task states:
- `todo`
- `doing`
- `blocked`
- `done`

## 14. Definition of Done (PR-Level)

- Code merged with passing CI.
- Unit tests added or updated for all changed behavior.
- Relevant ROM or smoke checks updated when needed.
- Spec and roadmap updated when behavior or scope changes.
- No unexplained regressions in compatibility matrix.

## 15. Legal and ROM Handling

- Emulator code and tests must not include copyrighted commercial ROM binaries.
- Commercial smoke tests use local user-supplied ROMs configured by path.
- Open test ROM suites can be vendorized or fetched by scripts as needed.

## 16. References

- Pan Docs
- RGBDS `gbz80(7)`
- Gekkio Game Boy: Complete Technical Reference
- Test suite repositories listed in project docs

