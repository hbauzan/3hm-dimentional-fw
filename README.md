# Three-Headed Semantic Firewall (sf-core)

A high-performance, deterministic, hardware-accelerated 3-tier semantic firewall and containment gateway for Large Language Models (LLMs) and vector manifolds.

## Build

Requires the Rust toolchain pinned in `rust-toolchain.toml` (1.94.1) and Cargo. From the workspace root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Copy `.env.example` to `.env` for local endpoints. `.env` stays untracked.

The gateway daemon is `sf-proxy`. It reads the same environment variables and requires `LANCEDB_URI`.

```bash
cargo run -p sf-proxy --bin sf-proxy
```

## Decoupled playground

`frontend/` is the browser client. Its only package manager is pnpm. The page calls `POST /api/v1/chat` on the gateway origin. Vite does not proxy that call. Copy `frontend/.env.example` to `frontend/.env` to point at another origin. API keys stay in `localStorage`.

```bash
cd frontend
pnpm install
pnpm test
pnpm dev
```

Open `http://127.0.0.1:5173`. The default gateway is `http://127.0.0.1:8088`. An empty `SF_CORS_ORIGIN` allows that origin. A fresh session sends `domain_id` and leaves the lock thresholds alone. Checking **Apply overrides**, or moving a slider, clones `l1_threshold`, `l2_threshold`, `quorum_k`, and `epsilon` for the next request. `provider: mock` ignores base URL and API key.

## Real TEI Embedder (BAAI/bge-m3)

To exit the mock embedder and evaluate live text prompts against real 1024-dimensional embeddings:

1. Launch Text Embeddings Inference (TEI) via Docker Compose:
   ```bash
   docker compose -f docker-compose.tei.yml up -d
   ```
2. Configure `.env`:
   ```bash
   SF_EMBEDDER=http
   TEI_HTTP_URL=http://127.0.0.1:8080
   SF_DEFAULT_DOMAIN=python
   ```
3. Calibrate domain locks with real embeddings into LanceDB:
   ```bash
   cargo run -p sf-bench --bin calibrate_real_domain
   ```
4. Or use the orchestrator `./setup.sh`:
   - **Option 1**: Automatically probes and starts TEI if `SF_EMBEDDER` is `http`/`grpc`.
   - **Option 7 -> 2**: Calibrates the `python` domain lock using live TEI embeddings.
   - **Option 11**: Quick toggle between `mock`, `http` and `grpc` embedders, and start/stop controls for TEI.

## Architecture & Roadmap

- [Deterministic AI Engineering Roadmap](roadmap/deterministic-ai-roadmap.md)
- [Architect Initial Instructions & Grounding](roadmap/Arquitect-initial-instrucctions.md)
- [Architecture spec](architecture_spec.md)
- [Domain language](CONTEXT.md)

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
