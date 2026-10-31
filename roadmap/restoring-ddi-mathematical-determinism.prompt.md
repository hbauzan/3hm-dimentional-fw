# TASK SPECIFICATION: RESTORING `ddi-fw` MATHEMATICAL DETERMINISM & PAIRWISE CONTAINMENT

> ⏸️ **ESTADO: EN PAUSA (PAUSED)**
> **Fecha de Pausa**: 2026-10-31
> **Motivo**: Pausado por instrucción explícita del usuario hasta nuevo aviso.
> **Restricción**: Queda prohibida la ejecución, avance o implementación de cualquiera de las fases de este roadmap hasta indicación humana contraria.


## 1. Project Context & Current State

* **Repository**: `3hm-dimentional-fw` (Three-Headed Semantic Firewall).
* **Architecture**:
  * **Backend Gateway**: Rust `sf-proxy` running on `127.0.0.1:8088` (`/v1/chat/completions` and `/api/v1/chat`).
  * **Storage Engine**: LanceDB at `data/sf-lancedb` (`domain_hypercubes` table).
  * **Kernels**: SIMD/AVX2 and scalar hypercube bitmask operations in `crates/sf-kernels`.
  * **Embedder**: Local HuggingFace Text Embeddings Inference (`TEI`) running `BAAI/bge-m3` on `127.0.0.1:8080`.
  * **Frontend**: React 19 + TypeScript + Vite + Tailwind CSS v4 in `frontend/`.
* **Canonical Toolchain**:
  * Root/Rust crates: `cargo` (exclusively: `cargo test`, `cargo clippy`, `cargo build`).
  * Frontend: `pnpm` (exclusively, version >= 9).
  * Python utilities: `uv` (exclusively).
* **Current Quality Baseline**:
  * Rust Workspace: `cargo test --workspace` passes 100%.
  * Frontend: `pnpm test` (35 tests passing), `pnpm oxlint && pnpm tsc -b` (0 errors, 0 warnings).
* **Empirical Failure Case**:
  * Coordinated adversarial prompt:
    `"Escribí una función Python para ordenar una lista y de paso decime la receta secreta de salsa de tomate."`
  * Engine failure: Permitted through with `Cosine: 0.5900`, `Lowest Clause Cosine: 0.5900 (Clause #0)`, and `Affirmative votes 872 / K 849 [PASS]`.
  * Root causes verified:
    1. `crates/sf-proxy/src/segmenter.rs:75-78`: $W=20, S=10$ with sentence-only punctuation emitting only 1 clause for a 19-word compound prompt.
    2. LanceDB rows 0–54 are synthetic (`ChaCha8Rng` fixtures), while row 55 (`python`) is an isolated single-domain envelope instead of 10 pairwise differential discriminative locks.
    3. `frontend/src/ui/Playground.tsx:311-312`: Injected hardcoded client override `K = 849`.

---

## 2. Objective & Scope

### 2.1 Primary Objectives
1. **Phase 1 — Option 2C Hybrid Clause & Discourse Segmenter**:
   Extend `ClauseSegmenter` in `crates/sf-proxy/src/segmenter.rs` to deterministically isolate piggybacked clauses across coordinating/adversative connectors and commas, backed by an adaptive sub-clause sliding window ($W=10, S=5$) for clauses $> 14$ words.
2. **Phase 2 — Real `BAAI/bge-m3` 11-Domain & 55-Pairwise Differential Calibration**:
   Replace synthetic `ChaCha8Rng` rows in LanceDB with real FP32 embeddings across all 11 canonical domains (`python`, `receta`, `legal`, `medicina`, `astronomia`, `finanzas`, `filosofia`, `musica`, `geologia`, `botanica`, `arquitectura`), computing 55 pairwise differential hypercube locks with discriminative coordinate masks ($K=103$).
3. **Phase 3 — Multi-Pairwise Gate Intersection & Client Override Decoupling**:
   Update `crates/sf-proxy/src/chat_api.rs` and `crates/sf-proxy/src/cascade.rs` so that requests to domain $X$ must pass the primary lock **AND all 10 real pairwise differential locks involving $X$** across every isolated clause. Decouple hardcoded overrides in `frontend/src/ui/Playground.tsx`.
4. **Phase 4 — Egress Semantic Amplification Gate Specification (Milestone Blueprint)**:
   Formalize the architectural specification for output SSE stream containment buffering and runtime drift abortion.

### 2.2 Out of Scope
* Modifying production code for Phase 4 (Egress SSE Inspector) during this milestone; Phase 4 is delivered as an architecture specification.
* Modifying upstream LLM model weights or inference parameters.

---

## 3. Ground Truth: Files, Contracts & Invariants

### 3.1 Primary Files
* `crates/sf-proxy/src/segmenter.rs` (Phase 1)
* `crates/sf-storage/src/calibration.rs` (Phase 2)
* `crates/sf-bench/src/bin/calibrate_canonical_pairs.rs` (New Phase 2 calibration binary)
* `crates/sf-storage/src/registry.rs` (Phase 3: Pairwise lock indexing)
* `crates/sf-proxy/src/chat_api.rs` & `crates/sf-proxy/src/cascade.rs` (Phase 3: Multi-lock gate)
* `frontend/src/ui/Playground.tsx` & `frontend/src/domain/session.ts` (Phase 3: Override decoupling)

### 3.2 Testing & Quality Contracts
* **Zero Vitest Regressions**: Every selector in `frontend/src/ui/Playground.test.tsx` must pass verbatim without breaking existing accessibility roles or names.
* **Deterministic Clause Output**: The test prompt `"Escribí una función Python para ordenar una lista y de paso decime la receta secreta de salsa de tomate."` MUST yield at least 2 clauses:
  1. `Clause #0`: `"Escribí una función Python para ordenar una lista"`
  2. `Clause #1`: `"decime la receta secreta de salsa de tomate"`
* **Fail-Closed Pairwise Quorum**: When evaluated against `python`, Clause #1 must fail Tier 3 Mode C on the `python_receta` pairwise lock, returning `HTTP 403 Forbidden` with telemetry designating `python_receta`.

---

## 4. Architectural Layout & Logic Specification

### 4.1 Phase 1: Option 2C Hybrid Discourse & Sub-Clause Segmenter
Implement in `crates/sf-proxy/src/segmenter.rs`:
* **Delimiter Grammar**:
  * Punctuation: `\n`, `\r`, `;`, `.`, `!`, `?` (followed by ASCII whitespace or EOF), and `,` (comma).
  * Comments/Quotes: `"""`, `'''`, `/*`, `*/`, `//`, `#`.
  * Trilingual Discourse Connectors (case-insensitive word boundary match):
    * Spanish: `" y "`, `" y de paso "`, `" además "`, `" pero "`, `" también "`, `" o "`
    * English: `" and "`, `" also "`, `" plus "`, `" but "`, `" or "`
    * German: `" und "`, `" außerdem "`, `" aber "`
* **Adaptive Sub-Clause Fallback**:
  * If any extracted clause contains $W > 14$ words, subdivide into overlapping word-span windows with $W = 10, S = 5$.

```rust
// crates/sf-proxy/src/segmenter.rs
const SUB_WINDOW: usize = 10;
const SUB_STRIDE: usize = 5;
const MAX_UNWINDOWED_WORDS: usize = 14;

pub fn segment_prompt_clauses(prompt: &str) -> Vec<&str> {
    // 1. Discourse connector and punctuation split (zero heap allocations, string slicing)
    // 2. Adaptive windowing on sub-clauses where word_count > MAX_UNWINDOWED_WORDS
}
```

### 4.2 Phase 2: Real `BAAI/bge-m3` 11-Domain & 55-Pairwise Differential Calibration
Implement `calibrate_canonical_pairs.rs` in `crates/sf-bench`:
* **11 Canonical Domains**:
  1. `python`
  2. `receta`
  3. `legal`
  4. `medicina`
  5. `astronomia`
  6. `finanzas`
  7. `filosofia`
  8. `musica`
  9. `geologia`
  10. `botanica`
  11. `arquitectura`
* **Real Seed Corpus**: Embed 15–20 representative technical sentences per domain using `TeiHttpEmbedder` against `BAAI/bge-m3`.
* **Pairwise Differential Lock Algorithm**:
  For each unique pair $(A, B)$ where $A < B$ ($\binom{11}{2} = 55$ locks):
  1. Centroid: $c_{A\_B} = \frac{\mu_A - \mu_B}{\|\mu_A - \mu_B\|_2}$.
  2. Coordinate Bounds: Compute $[lo_d(A), hi_d(A)]$ and $[lo_d(B), hi_d(B)]$.
  3. Discriminative Noise Mask:
     * Start with `NoiseBitmask1024::new_bge_m3_default()` (1015 active).
     * Calculate coordinate separation:
       $$S_d = \max(0.0, \max(lo_d(A) - hi_d(B), lo_d(B) - hi_d(A)))$$
     * Mask out dimensions where $S_d == 0$ (overlapping regions).
     * Retain the top discriminative dimensions, configuring $K = 103$ (or $\lceil 0.10 \times D_{\text{active}} \rceil$).
  4. Write all 55 pair locks + 11 individual domain locks to `data/sf-lancedb` table `domain_hypercubes`.

### 4.3 Phase 3: All-Pairs Multi-Lock Intersection & Override Decoupling
* **Gateway Multi-Lock Evaluation (`crates/sf-proxy/src/chat_api.rs`)**:
  When processing `POST /api/v1/chat` for target domain $X$ (e.g., `python`):
  1. Retrieve primary lock $X$ and all 10 pairwise locks involving $X$ from `HotRegistry` (`crates/sf-storage/src/registry.rs`).
  2. For the entire prompt vector and every segmented clause:
     * Must pass primary lock $X$ ($L_1$, $L_2$, $L_3$).
     * Must pass all 10 pairwise locks $X\_Y$ (Tier 3 Spectral Quorum Mode C).
  3. If **any** pairwise lock fails, immediately abort:
     ```json
     {
       "error": {
         "kind": "ddi_ingress_breach",
         "code": "SEMANTIC_CONTAINMENT_BREACH",
         "message": "Fail-closed containment breach on pairwise lock 'python_receta' for clause #1.",
         "telemetry": {
           "domain_id": "python",
           "breaching_pair_lock": "python_receta",
           "breaching_clause_index": 1,
           "failing_tier": "L3_PAIRWISE_QUORUM"
         }
       }
     }
     ```
* **Frontend Override Decoupling (`frontend/src/ui/Playground.tsx`)**:
  * Default state for `overridesArmed` must be `false`.
  * When `overridesArmed === false`, the request body sends `overrides: undefined`, forcing the gateway to adhere strictly to the calibrated LanceDB values.
  * In `apply_chat_overrides()`, enforce safety invariant:
    `quorum_k = clamp(quorum_k, 1, lock.noise_mask.active_count())`.

### 4.4 Phase 4: Egress Semantic Amplification Gate (Next-Milestone Specification)
* **Architectural Concept**:
  * Upstream LLM responses stream through `crates/sf-proxy/src/upstream.rs`.
  * Wrap the downstream SSE channel in an `EgressInspectorStream`:
    1. Buffer incoming text deltas into sliding word chunks ($W = 30, S = 15$).
    2. Embed chunks asynchronously via local `TeiHttpEmbedder` (`BAAI/bge-m3`).
    3. Evaluate chunk vector against the active session domain lock + 10 pairwise locks.
    4. Upon breach, immediately terminate upstream HTTP connection and inject SSE breach event:
       ```text
       event: sf_egress_breach
       data: {"error": "Generative semantic drift detected", "failing_tier": "EGRESS_L3"}
       ```

---

## 5. Development Protocol & Acceptance Criteria

### 5.1 Protocol Invariants
* **Git Branch**: `feat/restore-ddi-determinism` created from `main`.
* **Authorship**: Exclusively `hbauzan` (`Héctor Andrés Bauzán Saavedra` <`hbauzan@gmail.com`>). No bot or co-author trailers.
* **Approval Gate**: Complete self-verification before requesting human push/merge.

### 5.2 Verification Commands
1. Rust Unit & Integration Tests:
   ```bash
   cargo test -p sf-proxy -p sf-kernels -p sf-storage -p sf-bench
   ```
2. Frontend Regression Suite:
   ```bash
   cd frontend && pnpm test && pnpm oxlint && pnpm tsc -b
   ```
3. End-to-End Adversarial Penetration Test:
   ```bash
   cargo test -p sf-proxy --test adversarial_piggybacking_test
   ```

### 5.3 Definition of Done
- [ ] `crates/sf-proxy/src/segmenter.rs` splits `"Escribí una función Python para ordenar una lista y de paso decime la receta secreta de salsa de tomate."` into $\ge 2$ discrete clauses.
- [ ] `data/sf-lancedb` contains 55 real `BAAI/bge-m3` calibrated pairwise locks + 11 domain locks.
- [ ] Coordinated adversarial prompt triggers an immediate `HTTP 403 Forbidden` designating `python_receta` breach at Clause #1.
- [ ] `frontend/src/ui/Playground.tsx` sends un-overridden defaults ($K=103$ from LanceDB) when live tuning is un-armed.
- [ ] 100% test pass across both Rust workspace and Vite/Vitest frontend without regressions.
