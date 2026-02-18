# AGENTS.md

## Purpose

Agent operating rules for the `vibegb` project.

## Validation Handoff Requirement (Mandatory)

After each completed block of work, the agent must provide detailed validation instructions for the current project state.

Each handoff must include:

- Exact commands to run (copy/paste ready).
- Where to run them from (repo root or specific folder).
- What success looks like (expected output, behavior, or pass conditions).
- What failure looks like (common error signals).
- Manual run steps when relevant (how to launch the app, what screen/behavior to verify).
- What to check next if validation fails.

## Minimum Validation Detail

For every block, include both:

- Automated checks (for example: unit tests, lint, ROM test runner commands).
- Manual checks (for example: run emulator UI, load ROM, verify expected behavior).

Do not provide vague validation guidance. Always specify concrete commands and observable outcomes.

## Project Test Policy Reminder

- Unit tests are required for all new behavior and bug fixes.
- Validation instructions must reflect newly added tests and current milestone gates.
- Commercial ROM binaries are not stored in the repo; use local user-provided paths for smoke tests.

## Work Block Completion Gate (Mandatory)

- A work block is **not complete** until **all relevant test cases pass** (unit tests, lint/format checks, milestone ROM gates, and required smoke checks for the scope of the change).

## Local ROM Availability

- A local Pokemon Red ROM is available in the project root:
- `Pokemon - Red Version (USA, Europe) (SGB Enhanced).gb`
- Use this file for Pokemon Red smoke-validation instructions unless the user says otherwise.
