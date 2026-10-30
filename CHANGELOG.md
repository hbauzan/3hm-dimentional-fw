# Changelog

## 0.2.0

- `POST /api/v1/chat` on `sf-ui`: three-tier gate, HTTP 403 breach telemetry, and unbuffered SSE to a per-request OpenAI-compatible upstream.
- Embedded Chat Playground in `sf-ui`: Diagnostic Studio / Chat Playground tabs, SSE thread, live lock overrides, and the Rayos X breach card. API keys stay in `localStorage`.
- `ReqwestChatForwarder::forward_dynamic` joins a request base URL and sends `Authorization: Bearer` without changing `/v1/chat/completions`.

## 0.1.0

- Workspace bootstrap for `sf-kernels`, `sf-storage`, `sf-proxy`, and `sf-bench` on Rust 1.94.1.
- Scalar `AlignedVec1024` gates and the `/v1/chat/completions` tracer: `X-SF-Decision: PASS` or HTTP 403 `SEMANTIC_CONTAINMENT_BREACH`.
- SIMD dispatch (AVX-512, AVX2+FMA, NEON) with 0 ULP hypercube masks and cosine parity inside `1e-14`. RFC-003 rejects the 16-ULP micro-gap that `round-4` and `f16` accept.
- LanceDB hypercube registry with SNR pruning (`--extra-pruned-dims`), SHA-256 manifests, and wait-free `ArcSwap` reloads. Arrow is pinned at 58.4.1 because DataFusion requires `^58.3`. `lancedb` feature `remote` forces tonic/prost 0.14.
- Production proxy: TEI gRPC `EmbedStream` with `normalize=true`, HTTP `/embed` fallback, wildcard AnyAllowlist / AllLocks, and SSE passthrough.
- Rompepepe 220: scalar cosine leaks 50% (inside 14.5–58.2%) and L3 Mode C contains 220/220. The 1,000 RPS gate measured p50 0.046 ms, p99 0.127 ms, RSS 10.1 MB.
