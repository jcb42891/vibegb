# VibeGB Task Ledger

Last Updated: 2026-02-18

Status Legend: `todo`, `doing`, `blocked`, `done`

## Milestone Ledger

- `[done] [M0] [infra] Workspace, tooling, and validation harness | fmt + clippy + tests + Pokemon Red header smoke + desktop shell build/launch smoke (local, 2026-02-18)`
- `[doing] [M1] [core] CPU, interrupts, and timers milestone implementation | CPU/timer/interrupt unit tests + runner exec/serial harness + local blargg/mooneye subset run wired (2026-02-18); subset status: 18/18 pass`
- `[todo] [M2] [ppu] DMG PPU baseline and first playable smoke | PPU tests + dmg-acid2 + selected mealybug + Pokemon Red title/input smoke`
- `[todo] [M3] [cgb] CGB hardware feature set | CGB tests + cgb-acid2 + selected mooneye CGB subset`
- `[todo] [M4] [mbc] Cartridge expansion and save flow | mapper tests + mapper ROM subset + Pokemon Red save/load smoke`
- `[todo] [M5] [apu] APU and AV sync | APU tests + selected APU checks + manual AV sync acceptance`
- `[todo] [M6] [release] Compatibility and release hardening | compatibility threshold + performance budget + release artifact validation`

## Active Milestone Tasks

- `[done] [M1] [cpu] Implement DAA behavior edge-case coverage | DAA add/sub/carry regression unit tests (2026-02-18)`
- `[done] [M1] [runner] Add ROM exec mode with serial expectation checks | runner exec-mode unit tests + workspace fmt/clippy/tests (2026-02-18)`
- `[done] [M1] [runner] Add manifest-driven subset runner with mooneye signature expectation support | suite parser/executor unit tests + workspace fmt/clippy/tests (2026-02-18)`
- `[done] [M1] [infra] Download and stage local M1 conformance ROMs under roms/ | blargg cpu_instrs + mooneye acceptance binaries available locally (2026-02-18)`
- `[done] [M1] [cpu] Fix IE-push interrupt dispatch edge case for mooneye ie_push | ie_push pass-signature run + new interrupt push regression unit test (2026-02-18)`
