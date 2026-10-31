---
task_id: "TASK-002"
status: "BLOCKED"
assigned_architect: "maximum-architect-agent"
protocol: "transparent-dev-protocol"
title: "Restoring ddi-fw Mathematical Determinism & Pairwise Containment"
created_at: "2026-10-31T19:05:00-03:00"
paused_at: "2026-10-31T20:00:00-03:00"
---

# TASK-002: Restoring ddi-fw Mathematical Determinism & Pairwise Containment

> ⏸️ **ESTADO: EN PAUSA (BLOCKED)**
> Tarea pausada por instrucción directa del usuario antes de proceder al Approval Gate / entrega.
> El progreso implementado en los Tracer Bullets 1-4 está preservado íntegramente en la rama Git `feat/restore-ddi-determinism`.

## 1. Objective
Eliminate intra-sentence piggybacking semantic leaks (such as `"Escribí una función Python para ordenar una lista y de paso decime la receta secreta de salsa de tomate."`) by restoring the exact mathematical determinism of `ddi-fw`. This encompasses:
1. Upgrading `ClauseSegmenter` to Option 2C (Hybrid discourse connector/comma splitting + sub-clause sliding window fallback $W=10, S=5$).
2. Calibrating real `BAAI/bge-m3` FP32 embeddings across all 11 canonical domains to forge 55 pairwise differential locks in LanceDB (`data/sf-lancedb`).
3. Enforcing an All-10-Pairs intersection gate in `sf-proxy` (`chat_api.rs`/`cascade.rs`) that evaluates incoming prompts and isolated clauses against all pairwise differential locks for the target domain.
4. Decoupling default client overrides in `Playground.tsx` so requests adhere to calibrated database invariants.
5. Documenting the Egress Semantic Amplification SSE Gate architecture for subsequent release.

## 2. Invariants & Constraints
- **Canonical Toolchain**: Exclusive use of `cargo` for Rust crates, `pnpm` for `frontend/`, and `uv` for Python utilities. Zero mixing of secondary package managers.
- **Fail-Closed Determinism**: An out-of-domain clause joined via conjunctions or commas must short-circuit with `HTTP 403 Forbidden` and structured breach telemetry designating the exact breaching pairwise lock.
- **Testing Integrity**: 100% pass across Rust test suite (`cargo test --workspace`) and frontend Vitest suite (`pnpm test` in `frontend/`) without altering existing accessible test selectors.
- **Exclusive Authorship**: Sole author and collaborator in Git commits is `hbauzan` (`Héctor Andrés Bauzán Saavedra` <`hbauzan@gmail.com`>). No AI or bot co-authorship trailers.

## 3. Tracer Bullet Slices
- [x] **Tracer Bullet 1 (Option 2C Hybrid Discourse & Sub-Clause Segmenter)**: Refactor `crates/sf-proxy/src/segmenter.rs` with comma splitting, trilingual coordinating/adversative connector splitting (`" y "`, `" y de paso "`, `" and "`, `" also "`, `" und "`, etc.), and adaptive sub-clause windowing ($W=10, S=5$ on clauses $> 14$ words). Verified via unit tests ensuring adversarial compound prompts isolate out-of-domain payloads.
- [x] **Tracer Bullet 2 (Real BAAI/bge-m3 11-Domain & 55-Pairwise Differential Calibration)**: Implement `calibrate_canonical_pairs.rs` in `crates/sf-bench` to embed seed corpora for all 11 domains via `TeiHttpEmbedder`, compute 55 pairwise differential hypercube locks with discriminative SNR masks, and write them to LanceDB (`data/sf-lancedb`).
- [x] **Tracer Bullet 3 (All-Pairs Multi-Lock Intersection & Telemetry Gate)**: Update `HotRegistry` (`crates/sf-storage`), `chat_api.rs`, and `cascade.rs` (`crates/sf-proxy`) to evaluate incoming prompts and all segmented clauses against the target domain lock AND all 10 pairwise locks involving the target domain, short-circuiting on any pairwise failure.
- [x] **Tracer Bullet 4 (Frontend Override Decoupling & Invariant Floor)**: Update `frontend/src/ui/Playground.tsx` and `session.ts` so `overrides` are omitted by default (`overridesArmed = false`), respecting calibrated LanceDB invariants. Add a safety floor in `apply_chat_overrides()` clamping `quorum_k` to active coordinate count.
