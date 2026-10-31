# Changelog

## Unreleased

- Mock upstream isolation: decoupled `ChatUpstream::Mock` in `sf-proxy` via `mock_upstream` (`MockUpstreamForwarder`), ensuring offline deterministic SSE streaming on `/api/v1/chat` without falling back to unreachable network upstreams.
- Decoupled playground in `frontend/`: the browser calls `POST /api/v1/chat` on `sf-proxy`, renders upstream SSE on PASS, and draws the Rayos X card from `ChatBreachDto` on HTTP 403. Live overrides and the upstream selector stay in the page. API keys stay in `localStorage`.
- Upstream credentials from `.env`: `sf-proxy` serves `GET /api/v1/upstream/env` resolving environment variables and `.env` files for Gemini (`GEMINI_API_KEY`, `GOOGLE_API_KEY`), DeepSeek, OpenAI, Groq, and Ollama.
- UI `.env` integration: `Playground` in `frontend/` and embedded studio in `sf-ui` include an "Usar keys de .env" toggle and "Cargar de .env" sync button, showing live detection status and automatically forwarding credentials to upstream.
- Provider defaults: default Gemini model upgraded to `gemini-3.8-flash`, added presets for Groq (`llama-3.3-70b-versatile`) and Ollama (`llama3`).
- `sf-proxy` daemon serves `POST /v1/chat/completions` and `POST /api/v1/chat` on one process. `SF_CORS_ORIGIN` selects the browser origin; an empty value allows `*`.
- `/api/v1/chat` on the daemon emits `ChatBreachDto` and streams upstream SSE without buffering. `/v1/chat/completions` still emits `BreachErrorEnvelope`. Request overrides stay ephemeral.

## 0.2.0

- `POST /api/v1/chat` on `sf-ui`: three-tier gate, HTTP 403 breach telemetry, and unbuffered SSE to a per-request OpenAI-compatible upstream.
- Embedded Chat Playground in `sf-ui`: Diagnostic Studio / Chat Playground tabs, SSE thread, live lock overrides, and the Rayos X breach card. API keys stay in `localStorage`.
- `ReqwestChatForwarder::forward_dynamic` joins a request base URL and sends `Authorization: Bearer` without changing `/v1/chat/completions`.
- Calibrated chat embedder for the standalone studio: Python prompts pass into upstream SSE, culinary prompts breach at L1, piggybacked clauses breach at L2, and `PERMUTATION_NOISE` breaches the L3 spectral quorum. The Chat Playground loads those four prompts from preset chips.

## 0.1.0

- Workspace bootstrap for `sf-kernels`, `sf-storage`, `sf-proxy`, and `sf-bench` on Rust 1.94.1.
- Scalar `AlignedVec1024` gates and the `/v1/chat/completions` tracer: `X-SF-Decision: PASS` or HTTP 403 `SEMANTIC_CONTAINMENT_BREACH`.
- SIMD dispatch (AVX-512, AVX2+FMA, NEON) with 0 ULP hypercube masks and cosine parity inside `1e-14`. RFC-003 rejects the 16-ULP micro-gap that `round-4` and `f16` accept.
- LanceDB hypercube registry with SNR pruning (`--extra-pruned-dims`), SHA-256 manifests, and wait-free `ArcSwap` reloads. Arrow is pinned at 58.4.1 because DataFusion requires `^58.3`. `lancedb` feature `remote` forces tonic/prost 0.14.
- Production proxy: TEI gRPC `EmbedStream` with `normalize=true`, HTTP `/embed` fallback, wildcard AnyAllowlist / AllLocks, and SSE passthrough.
- Rompepepe 220: scalar cosine leaks 50% (inside 14.5–58.2%) and L3 Mode C contains 220/220. The 1,000 RPS gate measured p50 0.046 ms, p99 0.127 ms, RSS 10.1 MB.
