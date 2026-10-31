# Three-Headed Semantic Firewall (sf-core)

A high-performance, deterministic, hardware-accelerated 3-tier semantic firewall and containment gateway for Large Language Models (LLMs) and 1024-dimensional vector manifolds.

[![Rust](https://img.shields.io/badge/Rust-1.94.1-orange?logo=rust)](rust-toolchain.toml)
[![License](https://img.shields.io/badge/License-Apache--2.0-blue.svg)](LICENSE)
[![DNPI](https://img.shields.io/badge/Registro%20DNPI-N%C2%BA%20226-green)](NOTICE)
[![Latency](https://img.shields.io/badge/p99%20Latency-0.127ms%20%40%201000%20RPS-brightgreen)](reports/rompepepe_v3_matrix.json)
[![Containment](https://img.shields.io/badge/Rompepepe--220-100%25%20Containment-purple)](reports/rompepepe_v3_matrix.json)

---

## 3-Tier Defense Mechanism

`sf-core` intercepts incoming prompts, generates 1024-dimensional embeddings (via BAAI/bge-m3 or deterministic mock), and executes a strict 3-tier semantic gate before allowing upstream LLM inference:

```
Ingress Prompt ──> [ ClauseSegmenter ] ──> [ Embedder (1+M, 1024) ]
                                                   │
  ┌────────────────────────────────────────────────┴──────────────────────────────┐
  ▼                                                                               ▼
[ Tier 1: Centroid Cosine ]                                            [ Tier 2: Clause Gate ]
Cosine proximity to domain centroid.                                   Scores every individual segmented clause.
Breach if out-of-domain.                                               Blocks prompt injection / piggybacking.
  │ (PASS)                                                                        │ (PASS)
  └───────────────────────────────────────┬───────────────────────────────────────┘
                                          ▼
                             [ Tier 3: Spectral Quorum ]
           Bitmasked bounding-box evaluation over 1015 active coordinates
                 (9 universal noise dimensions pruned: 292, 297, 308,
                     329, 386, 404, 577, 616, 780).
           Affirmative in-box coordinate votes must reach Quorum K (default: 103).
                                          │
                   ┌──────────────────────┴──────────────────────┐
                   ▼ (PASS)                                      ▼ (BREACH)
         Upstream LLM Streaming                      HTTP 403 Forbidden
     (OpenAI, Gemini, DeepSeek, etc.)           (Structured ChatBreachDto)
```

---

## Workspace Topology

The repository is structured as a Cargo workspace with 5 specialized Rust crates and a decoupled web frontend:

| Component | Path | Description |
| :--- | :--- | :--- |
| **`sf-kernels`** | [`crates/sf-kernels`](crates/sf-kernels) | Hardware-accelerated SIMD kernels (NEON, AVX2+FMA, AVX-512) and IEEE 754 binary32 numeric routines. 0 ULP bitmask parity and cosine precision within $10^{-14}$. |
| **`sf-storage`** | [`crates/sf-storage`](crates/sf-storage) | Columnar domain hypercube registry on LanceDB (pinned Arrow 58.4.1), wait-free `ArcSwap` hot-reloading, SHA-256 lock manifests, and SNR pruning. |
| **`sf-proxy`** | [`crates/sf-proxy`](crates/sf-proxy) | High-throughput reverse proxy daemon (:8088). Serves OpenAI-compatible `/v1/chat/completions`, streaming `/api/v1/chat`, and upstream `.env` discovery. |
| **`sf-bench`** | [`crates/sf-bench`](crates/sf-bench) | Adversarial evaluation harness (Rompepepe 220), 1,000 RPS load benchmark (SLA p99 < 2ms), and domain lock calibrators. |
| **`sf-ui`** | [`crates/sf-ui`](crates/sf-ui) | Embedded diagnostic studio server (:3000) with Axum handlers for in-depth containment analysis. |
| **`frontend`** | [`frontend/`](frontend) | Decoupled Vite/React/TypeScript playground (:5173) with live parameter tuning, preset test chips, SSE streaming, and breach telemetry. |

---

## Quickstart: Interactive Control Panel (`./setup.sh`)

The workspace includes a unified CLI orchestrator that automates deployment, tests, benchmarks, and LanceDB management:

```bash
chmod +x setup.sh
./setup.sh
```

### Main Menu Options:
- **`1`**: **🚀 Desplegar Stack Completo**: Launches `sf-proxy` (:8088), the `frontend` SPA (:5173), and auto-seeds 55 canonical locks into LanceDB.
- **`2`**: **Iniciar Servicio en Primer Plano**: Runs `sf-proxy`, `frontend`, or `sf-ui` in the foreground.
- **`3`**: **Ejecutar Suite de Tests Unificada**: Runs `cargo test --workspace` + Vitest in `frontend/`.
- **`4`**: **Quality Gates & Linters Estrictos**: Executes `cargo fmt`, `cargo clippy`, TypeScript checks, and Oxlint.
- **`5`**: **Evaluar Ataques Rompepepe 220**: Runs the 220 adversarial attack test matrix against 55 locks.
- **`6`**: **Benchmark de Carga & SLA 1000 RPS**: Verifies the p99 latency SLA (< 2ms) under concurrent load.
- **`7`**: **Gestión de LanceDB & Catálogo**: Seeds canonical locks, calibrates domains, and runs `sf-ingest`.
- **`9`**: **Ver Logs en Vivo**: Tail individual or unified logs (`sf_proxy.log`, `sf_spa.log`, `sf_ui.log`).
- **`10`**: **Detener Todos los Servicios**: Clean termination of all running background daemons.
- **`11`**: **Configuración del Entorno & TEI**: Configures `.env` and toggles embedders (`mock`, `http`, `grpc`).
- **`13`**: **Guía Interactiva de Pruebas WebGUI**: Guided walkthrough for PASS and breach validation.

---

## Manual Build & Services

### 1. Prerequisites & Environment
Requires the Rust toolchain pinned in `rust-toolchain.toml` (1.94.1) and pnpm:

```bash
# Workspace environment setup
cp .env.example .env
```

### 2. Rust Workspace
```bash
# Code formatting and quality gates
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

# Execute test suite
cargo test --workspace
```

### 3. Run Gateway Daemon (`sf-proxy`)
```bash
cargo run -p sf-proxy --bin sf-proxy
```
By default, `sf-proxy` listens on `http://127.0.0.1:8088`. It loads the calibrated hypercube locks from `LANCEDB_URI` (`data/lancedb`).

### 4. Run Decoupled Playground (`frontend/`)
```bash
cd frontend
pnpm install
pnpm test
pnpm dev
```
Open `http://127.0.0.1:5173` in your browser.

---

## Web GUI Playground (`http://127.0.0.1:5173`)

The frontend playground provides full interactive control and real-time visualization of the containment gateway:

- **Live Chat & Streaming SSE**: Valid prompts (`PASS`) stream LLM responses in real-time.
- **Visual Breach Diagnostics (`BreachCard`)**: On HTTP 403, displays the failing tier (`L1_CENTROID_COSINE`, `L2_CLAUSE_GATE`, or `L3_SPECTRAL_QUORUM`), cosine proximity bar, the exact breached clause text/index, and in-box votes vs. Quorum $K$.
- **Live Tuning Panel**: Ephemeral overrides for the next request without altering persistent LanceDB records:
  - *L1 cosine threshold* (0.00 – 1.00)
  - *L2 clause threshold* (0.00 – 1.00)
  - *Quorum K* (1 – 1015 active coordinates)
  - *Epsilon dilation* (0.000 – 0.500)
- **Upstream LLM Integration**:
  - Providers: `mock`, `gemini` (default: `gemini-3.8-flash`), `openai` (`gpt-4o-mini`), `deepseek` (`deepseek-chat`), `groq` (`llama-3.3-70b-versatile`), and `ollama` (`llama3`).
  - **1-Click .env Auto-Sync**: Queries `/api/v1/upstream/env` to read credentials directly from the server's `.env`.
  - Sensitive API keys remain secured and isolated in browser `localStorage`.
- **Preconfigured Test Chips**:
  - `Python generator`: Nominal PASS for the `python` domain.
  - `Risotto L1`: Out-of-domain prompt triggering Tier 1 Centroid Cosine containment.
  - `Piggyback L2`: Multi-clause prompt sneaking an unrelated query triggering Tier 2 Clause Gate.
  - `Spectral L3`: Coordinate drift prompt triggering Tier 3 Spectral Quorum containment.

---

## Real TEI Embedder (BAAI/bge-m3)

By default, the gateway operates with the zero-dependency deterministic `mock` embedder. To evaluate prompts against real 1024-dimensional embeddings:

1. **Launch Text Embeddings Inference (TEI) via Docker Compose**:
   ```bash
   docker compose -f docker-compose.tei.yml up -d
   ```
2. **Configure `.env`**:
   ```bash
   SF_EMBEDDER=http
   TEI_HTTP_URL=http://127.0.0.1:8080
   SF_DEFAULT_DOMAIN=python
   ```
3. **Calibrate the eleven canonical domains and the 55 pairwise locks with live TEI embeddings**:
   ```bash
   cargo run -p sf-bench --bin calibrate_canonical_pairs
   ```
   `calibrate_real_domain` still writes one domain lock and keeps the other rows.
*(Or use `./setup.sh` -> Option 11 to switch embedders and Option 7 -> 2 to calibrate automatically).*

---

## API Endpoints (`sf-proxy`)

The gateway daemon exposes the following HTTP interfaces on port `8088`:

| Method | Endpoint | Description |
| :--- | :--- | :--- |
| `POST` | `/v1/chat/completions` | OpenAI drop-in proxy. Passes upstream response on PASS or returns HTTP 403 `BreachErrorEnvelope`. |
| `POST` | `/api/v1/chat` | Streaming SSE endpoint for the playground. Returns unbuffered `text/event-stream` on PASS, HTTP 403 `ChatBreachDto` when the primary lock fails, or HTTP 403 `kind: ddi_ingress_breach` when a pairwise lock fails. |
| `GET` | `/api/v1/upstream/env` | Discovery endpoint reporting configured upstream API keys and models available in the daemon `.env`. |
| `GET` | `/health` | Gateway health check. |

---

## Benchmarks & Performance SLA

Evaluated on Apple Silicon (M-series, ARM64 NEON) and x86_64:

- **Rompepepe 220 Adversarial Matrix**:
  - Cosine similarity alone leaks 50.0% of adversarial bypass attempts (110/220).
  - Tier 3 Mode C **Spectral Quorum achieves 100% containment** (220/220 breaches blocked).
- **1,000 RPS Concurrent Load Benchmark**:
  - **Latency p50**: 0.046 ms
  - **Latency p99**: 0.127 ms (SLA constraint: < 2.0 ms)
  - **Memory RSS**: 10.1 MB (SLA constraint: < 45 MB)

---

## Architecture & Roadmap

- [Deterministic AI Engineering Roadmap](roadmap/deterministic-ai-roadmap.md)
- [Architect Initial Instructions & Grounding](roadmap/Arquitect-initial-instrucctions.md)
- [Architecture Spec](architecture_spec.md)
- [Ubiquitous Domain Language (CONTEXT)](CONTEXT.md)
- [Context Map](CONTEXT-MAP.md)

---

## License & Intellectual Property

Licensed under the Apache License, Version 2.0 — see [LICENSE](LICENSE) and [NOTICE](NOTICE).  
Copyright © 2026 Héctor Andrés Bauzán Saavedra.

### Official Software Registration / Registro Oficial

• **Obra / Work**: Three-Headed Semantic Firewall  
• **Titular / Author**: Héctor Andrés Bauzán Saavedra  
• **Número de Inscripción**: Nº 226  
• **Fecha de Inscripción**: 04/08/2026 (August 4, 2026)  
• **Clasificación / Type**: Programa de ordenador (Software)  
• **Organismo / Registry**: Dirección Nacional de la Propiedad Industrial (DNPI - MIEM) / Registro de Software, República Oriental del Uruguay  
• **Marco Legal**: Ley 9.739 de 17/12/1937 (conforme Ley N° 20.212 y Decreto N° 39/2025)  

Pursuant to Section 4(d) of the Apache License, Version 2.0, the [NOTICE](NOTICE) file must be retained and distributed with any reproduction or derivative work.
