# Architecture spec — sf-core

Toolchain: Cargo only. Active compiler pin: Rust `1.94.1` (`rust-toolchain.toml`). Crate MSRV `1.91` matches `lancedb 0.40.0` / `lance 13.0.0`. Arrow crates are exact `=58.4.1` (`=58.0.0` does not resolve: DataFusion 54 requires `arrow-schema ^58.3.0`) and are imported from `sf-storage` only as `lancedb::arrow::{arrow_array, arrow_buffer, arrow_schema}`.

## ADR-001 — RFC-003 numerical representation

Store coordinates and compare bounds in IEEE 754 binary32. Accumulate dot products and norms in binary64. The hot path does not call `round`, does not format floats, and does not narrow to `f16` or `bf16`.

Parity contract (architect Q3): hypercube masks match scalar bits (0 ULP). Cosine values satisfy \(|\mathrm{simd}-\mathrm{scalar}| < 10^{-14}\) on the `f64` result. That bound is not a `to_bits` equality.

## ADR-002 — Tier 1 categorical cosine

Each **DomainHypercubeLock** carries its own cosine threshold. Allowlist polarity `0` passes when cosine is at least the threshold. Denylist polarity `1` passes when cosine is below the threshold.

## ADR-003 — Tier 2 clause gate

**ClauseSegmenter** splits the prompt, then each clause is scored against the same centroid. A breach on clause `m` stops the cascade before Tier 3.

## ADR-004 — Tier 3 pluggable inspector

- Mode A: every active Top-K coordinate lies inside the pre-dilated box.
- Mode B: every active coordinate lies inside the box.
- Mode C: affirmative votes reach **SpectralQuorum** `K` (default 103).

`epsilon = 0` across all 1024 coordinates and PCA reduction are rejected.

## Resolved execution decisions

| Id | Decision |
| :--- | :--- |
| Q1 | Toolchain channel `1.94.1`. |
| Q2 | `X-Semantic-Domain: *` defaults to **AnyAllowlist**. **AllLocks** requires `X-SF-Wildcard-Mode: all`. |
| Q3 | Mask bits are 0 ULP. Cosine parity uses the `1e-14` absolute bound. |
| Q4 | NeMo Guardrails is out of scope. |
| Q5 | Direct pins: `clap 4.5.23`, `half 2.4.1`, `tower 0.5.2`. Arrow is `=58.4.1` because `datafusion 54` requires `arrow-schema ^58.3.0`. `lancedb 0.40.0` is built with feature `remote` because `Error::Http` is referenced outside its cfg. That feature pulls `tonic`/`prost` 0.14, so the workspace uses that line instead of 0.12.3/0.13.4. |
| Q6 | Slice 2 is green when NEON matches scalar on this Apple M4. AVX-512 and AVX2 ship `cfg`-gated and are not executed here. |
| Q7 | `manifest.json` follows the roadmap document: `vector_dim` and `alignment_bytes` live in `state_schema`. |
| Q8 | The load process links LanceDB. RSS is measured in Slice 5. A result at or above 45 MB stops at the approval gate with the measured number. |

## Hot path

Ingress reads `POST /v1/chat/completions`, segments clauses, embeds one batch of shape `[1 + M, 1024]`, then runs L1, L2, L3. A pass streams the upstream body unchanged and sets `X-SF-Decision: PASS`. A breach returns HTTP 403 `SEMANTIC_CONTAINMENT_BREACH` and does not call the upstream model.

Pre-dilated bounds (`lo - eps`, `hi + eps`) are computed when a lock is loaded, not per request on this path.

## Playground chat (`sf-ui`)

`POST /api/v1/chat` gates the latest user message on one **DomainHypercubeLock** (the registry id, or the first loaded lock when `domain_id` is omitted). Tier 1 is centroid cosine, Tier 2 is **ClauseSegmenter** cosine, Tier 3 is Mode C **SpectralQuorum**. Comparisons follow lock polarity. Coordinates stay `f32`.

The diagnostic binary does not call TEI. Python markers project onto the lock centroid inside the pre-dilated box and pass the cascade. Culinary markers project orthogonal to that centroid and breach Tier 1. A string that contains both stays in-domain at Tier 1, so the culinary clause can breach Tier 2. `PERMUTATION_NOISE` stays above the cosine floors and breaches Tier 3. Any other string keeps the SHA-256 projection.

A pass returns the upstream body as `text/event-stream` and does not buffer the SSE payload. A breach returns HTTP 403 `application/json` and does not call the upstream model. `failing_tier` is `L1_CENTROID_COSINE`, `L2_CLAUSE_GATE`, or `L3_SPECTRAL_QUORUM`.

Request overrides replace `l1_cosine_threshold`, `l2_clause_threshold`, `quorum_k`, and `epsilon` on a clone. An epsilon override rebuilds dilated bounds from the stored epsilon: raw bound = dilated bound ± stored epsilon, then dilate again with the request epsilon. The registry lock is left unchanged.

The upstream body is OpenAI chat-completions JSON (`model`, `messages`, `stream: true`). A per-request base that already ends in `/v1` or `/openai` joins `/chat/completions`. Any other host joins `/v1/chat/completions`. `api_key` is sent as `Authorization: Bearer` and is omitted from the JSON body. `/v1/chat/completions` on `sf-proxy` keeps its process-wide base (`{base}/v1/chat/completions`) and its breach envelope (`L1_CATEGORICAL_COSINE`, `L2_SLIDING_WINDOW`, `L3_DDI_INSPECTOR`).

## Gateway daemon

The `sf-proxy` binary loads a LanceDB registry and serves both ingress routes on `SF_HOST`:`SF_PORT` (default `127.0.0.1:8088`). `POST /v1/chat/completions` still returns `BreachErrorEnvelope`. `POST /api/v1/chat` returns `ChatBreachDto` or an unbuffered `text/event-stream`. The two JSON shapes stay distinct.

`SF_CORS_ORIGIN` sets `Access-Control-Allow-Origin`. An empty value or `*` allows any origin. Chat overrides clone the **DomainHypercubeLock** for that request. A registry write happens only through an explicit LanceDB reload.

The playground in `frontend/` is a separate origin. It posts to `/api/v1/chat` on this daemon and reads `ChatBreachDto` or the unbuffered event stream. The dev server does not proxy that call. API keys stay in browser `localStorage`.
