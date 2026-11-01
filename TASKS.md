---
task_id: "TASK-005"
status: "DONE"
assigned_architect: "maximum-architect-agent"
protocol: "transparent-dev-protocol"
title: "WebGUI E2E Testing, Mock Stabilization & Comprehensive Audit"
created_at: "2026-11-01T12:55:00-03:00"
---

# TASK-005: WebGUI Comprehensive E2E Testing, Mock Stabilization & Override Diagnostics

## 1. Objective
Establish an automated end-to-end (E2E) testing framework with Playwright in `frontend/` alongside browser subagent interactive verification, systematically validating every function of the WebGUI (Playground). Eliminate the HTTP 403 unhandled crash when tuning the L2 Clause Threshold perilla under mock mode, normalize pairwise breach envelopes into structured visual `BreachCard`s, and ensure zero regressions across Vitest (36 tests) and Cargo suites.

## 2. Invariants & Constraints
- **Canonical Toolchain**: Use `pnpm` exclusively in `frontend/` (`pnpm test`, `pnpm test:e2e`, `pnpm run lint`, `pnpm run build`) and `cargo` in Rust.
- **Testing Integrity**: 100% pass across Playwright E2E suite, existing Vitest suites, and `cargo test --workspace`.
- **High Autonomy**: Solve UI state bugs, DTO parsing, Playwright selectors, and mock forwarder wiring in `sf-proxy` autonomously via TDD. Consult only if changes demand re-calibrating LanceDB physical schemas or RFC-003 numerical kernels.
- **Exclusive Authorship**: Sole author and collaborator in Git commits is `hbauzan` (`Héctor Andrés Bauzán Saavedra` <`hbauzan@gmail.com`>). No AI or bot co-authorship trailers.

## 3. Tracer Bullet Slices
- [x] **Tracer Bullet 1 (Mock Gateway & Pairwise Telemetry Normalization)**: Update `crates/sf-proxy/src/chat_api.rs` and `cascade.rs` so mock mode does not false-breach on pairwise differential locks; normalize `PairwiseBreachEnvelope` in `frontend/src/domain/breach.ts` so `BreachCard` renders cleanly on all 403s; fix default epsilon preservation in `Playground.tsx`.
- [x] **Tracer Bullet 2 (Playwright Headless Suite Setup)**: Install `@playwright/test` and chromium with `pnpm`; configure `frontend/playwright.config.ts`; add `test:e2e` to `package.json`.
- [x] **Tracer Bullet 3 (Comprehensive E2E Test Suite & Defect Remediation)**: Write `frontend/e2e/playground.spec.ts` testing all 6 WebGUI functional areas (presets, sliders, domain dropdown, upstream switching, .env sync, shortcuts); fix any defects discovered during execution.
- [x] **Tracer Bullet 4 (Browser Subagent Validation & macOS Notification)**: Run interactive `browser_subagent` session against live WebGUI; dispatch native notification via `/Users/hbauzan/treepwood/fabrica_de_software/agent.sh notify`; present Approval Gate.
