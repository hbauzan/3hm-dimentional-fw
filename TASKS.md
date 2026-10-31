---
task_id: "TASK-003"
status: "PENDING"
assigned_architect: "maximum-architect-agent"
protocol: "transparent-dev-protocol"
title: "Native Apple M4 Metal GPU (mps) BGE-M3 Embedder Daemon"
created_at: "2026-10-31T20:08:00-03:00"
---

# TASK-003: Native Apple M4 Metal GPU (mps) BGE-M3 Embedder Daemon

## 1. Objective
Replace the slow Docker CPU TEI container (`sf-tei`) on macOS Apple Silicon with a 100% native Metal Performance Shaders (`mps`) embedder daemon (`tools/bge_m3_mps_server.py`) running on `127.0.0.1:8080`, reducing embedding latency from 150–600 ms to < 30 ms while enforcing strict RFC-003 `torch.float32` precision. Integrate its full lifecycle into `setup.sh`.

## 2. Invariants & Constraints
- **Canonical Toolchain**: Use `uv` exclusively for Python execution (`uv run --script tools/bge_m3_mps_server.py`) and `cargo` for Rust workspace. Zero global pip pollution.
- **Strict IEEE 754 Precision (RFC-003)**: `torch.float32` mandatory. Zero FP16/BF16 downcasting.
- **Wire Compatibility**: Drop-in compatible with `POST /embed` and `GET /health` expected by `crates/sf-proxy/src/tei.rs`.
- **Exclusive Authorship**: Sole author and collaborator in Git commits is `hbauzan` (`Héctor Andrés Bauzán Saavedra` <`hbauzan@gmail.com`>). No AI or bot co-authorship trailers.

## 3. Tracer Bullet Slices
- [ ] **Tracer Bullet 1 (Native Metal Embedder Daemon)**: Create `tools/bge_m3_mps_server.py` with inline PEP 723 `uv` metadata, strict `torch.float32`, startup Metal shader warmup, and drop-in `/health` and `/embed` endpoints.
- [ ] **Tracer Bullet 2 (Docker TEI Decommissioning & setup.sh Integration)**: Stop Docker container `sf-tei`, reclaim port `127.0.0.1:8080`, and update `setup.sh` to start/stop/probe the native background daemon (`.sf_tei_mps.pid`, `sf_tei_mps.log`) on macOS Darwin arm64.
- [ ] **Tracer Bullet 3 (Deterministic Verification Gates)**: Pass Gate 1 (Docker down), Gate 2 (`/health` mps), Gate 3 (unit norm + latency < 30 ms benchmark), and Gate 4 (`cargo test --workspace --locked`).
