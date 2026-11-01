# TASK SPECIFICATION: REPLACE DOCKER CPU TEI WITH NATIVE APPLE M4 METAL GPU (`mps`) EMBEDDER DAEMON

## 1. Project Context & Current State (Ground Truth)

* **Repository**: `3hm-dimentional-fw` (Three-Headed Semantic Firewall).
* **Hardware Architecture**: Apple Silicon (Apple M4, `aarch64-apple-darwin`) with unified memory and dedicated Metal Performance Shaders (`mps`) GPU cores.
* **Current Bottleneck & Empirical Baseline**:
  * Docker container `sf-tei` (`ghcr.io/huggingface/text-embeddings-inference:cpu-1.6`) runs inside a Linux virtual machine without hardware access to Apple Metal GPU (`mps`).
  * `BAAI/bge-m3` runs exclusively on virtualized CPU cores, consuming ~3 GB of VM RAM and adding `150–600 ms` of embedding latency per batch.
* **Consumer Gateway Contract**:
  * `crates/sf-proxy/src/tei.rs` (`TeiHttpEmbedder`) communicates with `.env` `TEI_HTTP_URL=http://127.0.0.1:8080`.
  * Wire format: HTTP `POST /embed` with JSON payload `{"inputs": [...], "truncate": true, "normalize": true}`.
  * Expected response: JSON matrix `[[f32; 1024], ...]`.
* **Canonical Toolchain**:
  * Python utilities & daemons: `uv` (exclusively; using PEP 723 inline script metadata `/// script`).
  * Rust Workspace: `cargo` (exclusively).
  * Frontend: `pnpm` (exclusively).

---

## 2. Objective, Scope & Design Decision

### 2.1 Primary Objectives
1. **Drop-in Wire-Compatible Native Daemon**:
   Replace the Docker CPU container on `127.0.0.1:8080` with a native Python daemon (`tools/bge_m3_mps_server.py`) executed via `uv run` on Apple Metal GPU (`mps`).
2. **RFC-003 Mathematical Precision**:
   Enforce strict IEEE 754 binary32 (`torch.float32`) end-to-end. FP16 and BF16 downcasting are strictly forbidden to protect hypercube coordinate micro-margins ($10^{-4} \dots 10^{-6}$).
3. **Sub-30ms Latency via Metal Shader Warmup**:
   Pre-compile Apple Metal shaders during process initialization using warmup passes, eliminating runtime JIT stutter on the first inbound request.
4. **Control Panel (`setup.sh`) Integration**:
   Update `setup.sh` so that on macOS Darwin `arm64`, it stops and removes Docker TEI and manages the native daemon (`sf_tei_mps.log`, `.sf_tei_mps.pid`, clean shutdown on option 10).

### 2.2 Out of Scope
* Modifying Rust client code in `crates/sf-proxy/src/tei.rs` (the Python server must remain 100% drop-in compatible).
* Quantizing or altering model weights for `BAAI/bge-m3`.

---

## 3. Ground Truth: Files, Contracts & Invariants

### 3.1 Primary Files
* `tools/bge_m3_mps_server.py` (New: Native Apple Metal Embedder Server).
* `setup.sh` (Modified: Integrate native daemon startup, health probing, and shutdown).
* `docker-compose.tei.yml` (Decommissioned on Apple Silicon).

### 3.2 Invariants & Quality Contracts
1. **Toolchain Hygiene**: Managed exclusively via `uv run --script` with inline dependency specification (PEP 723). Zero manual virtualenv or global pip modifications.
2. **Deterministic Precision (RFC-003)**:
   * Global torch default dtype: `torch.float32`.
   * Model parameters: strictly `torch.float32`.
   * Output vectors: normalized to unit length ($\|v\|_2 = 1.0 \pm 10^{-5}$).
3. **Exclusive Authorship**: Sole author and collaborator in Git commits is `hbauzan` (`Héctor Andrés Bauzán Saavedra` <`hbauzan@gmail.com`>). Zero AI or bot co-authorship trailers.

---

## 4. Architectural Layout & Algorithmic Specification

### 4.1 Topology Diagram

```
+-----------------------------------------------------------+
| Apple M4 Host (macOS Darwin aarch64)                      |
|                                                           |
|  +--------------------+         HTTP POST /embed          |
|  | sf-proxy (:8088)   | --------------------------------+ |
|  | (Rust Gateway)     |                                 | |
|  +--------------------+                                 | |
|                                                         v |
|  +------------------------------------------------------+ |
|  | Native MPS Daemon (:8080)                            | |
|  | tools/bge_m3_mps_server.py (uv run)                  | |
|  |                                                      | |
|  | - Device: 'mps' (Metal Performance Shaders)          | |
|  | - Precision: torch.float32 (Strict RFC-003)          | |
|  | - Memory: Unified Apple M4 Memory (Zero VM Overhead) | |
|  | - Endpoints: GET /health, POST /embed                | |
|  +------------------------------------------------------+ |
|                           |                               |
|                           x (Replaces Docker VM on :8080) |
+-----------------------------------------------------------+
```

### 4.2 Server Implementation (`tools/bge_m3_mps_server.py`)

```python
#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = [
#     "torch>=2.4.0",
#     "sentence-transformers>=3.2.0",
#     "fastapi>=0.115.0",
#     "uvicorn>=0.32.0",
#     "pydantic>=2.9.0",
# ]
# ///

import os
import time
from typing import List, Optional, Union
import torch
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from sentence_transformers import SentenceTransformer
import uvicorn

MODEL_ID = os.environ.get("BGE_M3_MODEL_ID", "BAAI/bge-m3")
HOST = os.environ.get("TEI_HOST", "127.0.0.1")
PORT = int(os.environ.get("TEI_PORT", "8080"))

def select_device() -> str:
    if torch.backends.mps.is_available() and torch.backends.mps.is_built():
        return "mps"
    if torch.cuda.is_available():
        return "cuda"
    return "cpu"

DEVICE = select_device()

# RFC-003 MANDATORY INVARIANT: Strict IEEE 754 binary32 (torch.float32).
torch.set_default_dtype(torch.float32)

print(f"[sf-bge-m3-mps] Loading {MODEL_ID} on device='{DEVICE}' with strict dtype=torch.float32...")
t0 = time.perf_counter()
model = SentenceTransformer(
    MODEL_ID,
    device=DEVICE,
    model_kwargs={"dtype": torch.float32},
)
model.to(dtype=torch.float32)
model.eval()

# Warmup pass to pre-compile Apple Metal (MPS) shaders before accepting traffic
with torch.inference_mode():
    _ = model.encode(
        ["warmup python def sort(x): return sorted(x)", "warmup receta salsa de tomate"],
        batch_size=2,
        normalize_embeddings=True,
        convert_to_numpy=True,
    )
    if DEVICE == "mps":
        torch.mps.synchronize()

load_ms = (time.perf_counter() - t0) * 1000.0
print(f"[sf-bge-m3-mps] Ready on http://{HOST}:{PORT} (device={DEVICE}, dtype=float32, startup={load_ms:.1f}ms)")

app = FastAPI(title="Three-Headed Semantic Firewall — Native Apple Metal BGE-M3 Server")

class EmbedRequest(BaseModel):
    inputs: Union[str, List[str]]
    truncate: Optional[bool] = True
    normalize: Optional[bool] = True

@app.get("/health")
def health():
    return {
        "status": "ok",
        "model_id": MODEL_ID,
        "device": DEVICE,
        "mps_available": bool(torch.backends.mps.is_available()),
        "dtype": str(next(model.parameters()).dtype),
        "dim": 1024,
    }

@app.post("/embed")
def embed(req: EmbedRequest) -> List[List[float]]:
    texts = [req.inputs] if isinstance(req.inputs, str) else req.inputs
    if not texts:
        raise HTTPException(status_code=400, detail="Empty inputs list")

    with torch.inference_mode():
        embeddings = model.encode(
            texts,
            batch_size=min(len(texts), 32),
            normalize_embeddings=True if req.normalize is None else req.normalize,
            convert_to_numpy=True,
            show_progress_bar=False,
        )
        if DEVICE == "mps":
            torch.mps.synchronize()

    # Enforce strict float32 representation before returning
    f32_matrix = embeddings.astype("float32", copy=False)
    return f32_matrix.tolist()

if __name__ == "__main__":
    uvicorn.run(app, host=HOST, port=PORT, log_level="warning")
```

### 4.3 Control Panel (`setup.sh`) Logic Overhaul

In `setup.sh`:
* On macOS Darwin `arm64`:
  1. Define PID file: `PID_FILE_TEI_MPS=".sf_tei_mps.pid"`.
  2. Define Log file: `LOG_TEI_MPS="sf_tei_mps.log"`.
  3. `probe_tei()`: Detect if the native process is running via PID/port or if Docker container is running.
  4. `start_tei_daemon()`:
     * Check if Docker container `sf-tei` is active; if so, stop it via `docker compose -f docker-compose.tei.yml down`.
     * Launch `tools/bge_m3_mps_server.py` using `launch_detached . "$LOG_TEI_MPS" uv run tools/bge_m3_mps_server.py`.
     * Store PID in `.sf_tei_mps.pid`.
     * Call `wait_tei_healthy()`.
  5. `stop_tei_daemon()`:
     * If `.sf_tei_mps.pid` exists, send `SIGTERM`, wait for exit, remove PID file.
     * Also ensure `docker compose -f docker-compose.tei.yml down 2>/dev/null || true` is executed.

---

## 5. Development Protocol, Quality Gate & Definition of Done

### 5.1 Protocol Invariants
* **Git Branch**: `feat/native-m4-metal-embedder` created from `main`.
* **Authorship**: Exclusively `hbauzan` (`Héctor Andrés Bauzán Saavedra` <`hbauzan@gmail.com`>).
* **Approval Gate**: Full self-verification before requesting human push/merge.

### 5.2 Verification Gates (Execution Ledger Required)

```bash
# Gate 1: Ensure Docker TEI container is stopped on port 8080
docker compose -f docker-compose.tei.yml down 2>/dev/null || true

# Gate 2: Start Native Apple M4 Metal (MPS) BGE-M3 Server and verify /health
chmod +x tools/bge_m3_mps_server.py
uv run tools/bge_m3_mps_server.py > sf_tei_mps.log 2>&1 &
sleep 5
curl -s http://127.0.0.1:8080/health

# Gate 3: Benchmark native MPS embedding latency & verify 1024D float32 unit norm
python3 -c '
import urllib.request, json, time, math
url = "http://127.0.0.1:8080/embed"
payload = json.dumps({"inputs": ["Escribí una función Python para ordenar una lista", "decime la receta secreta de salsa de tomate"], "truncate": True, "normalize": True}).encode()
t0 = time.perf_counter()
req = urllib.request.Request(url, data=payload, headers={"Content-Type": "application/json"})
resp = json.loads(urllib.request.urlopen(req).read().decode())
dt_ms = (time.perf_counter() - t0) * 1000.0
norm0 = math.sqrt(sum(x*x for x in resp[0]))
print(f"batch_size={len(resp)} dim={len(resp[0])} norm0={norm0:.6f} mps_latency_ms={dt_ms:.2f}")
assert len(resp) == 2 and len(resp[0]) == 1024
assert abs(norm0 - 1.0) < 1e-5
'

# Gate 4: Verify sf-proxy & calibrate_real_domain work seamlessly against the MPS server
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
export SDKROOT="$(xcrun --sdk macosx --show-sdk-path)"
cargo test --workspace --locked
```

### 5.3 Definition of Done
- [ ] Docker container `sf-tei` stopped and port `127.0.0.1:8080` reclaimed.
- [ ] `tools/bge_m3_mps_server.py` implemented with PEP 723 `uv` metadata.
- [ ] `/health` endpoint responds with `"device": "mps"` and `"dim": 1024`.
- [ ] Measured batch latency on Apple M4 Metal is $< 30\text{ ms}$ (vs. $150\text{--}600\text{ ms}$ on Docker CPU).
- [ ] `setup.sh` manages background native daemon lifecycle cleanly on macOS.
- [ ] 100% test pass across Rust workspace with zero regressions.
