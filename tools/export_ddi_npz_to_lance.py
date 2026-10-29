#!/usr/bin/env python3
"""Bridge a NumPy ``rows.npz`` matrix into a LanceDB table.

The Rust workspace does not read NPZ. This script is the offline ingest path
for a real ``ddi_fw/out/rows.npz`` (array key ``rows``, shape ``[N, 1024]``,
``float32``). It writes the raw little-endian layout that ``sf-ingest
--input-f32`` reads: ``u64`` row count, then ``N * 1024`` binary32 values.

Run with uv from a environment that has ``numpy``::

    uv run --with numpy python tools/export_ddi_npz_to_lance.py \
        --npz ddi_fw/out/rows.npz --out data/rows.f32
"""

from __future__ import annotations

import argparse
import struct
from pathlib import Path


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--npz", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--key", default="rows")
    args = parser.parse_args()

    import numpy as np

    archive = np.load(args.npz)
    rows = np.asarray(archive[args.key], dtype=np.float32)
    if rows.ndim != 2 or rows.shape[1] != 1024:
        raise SystemExit(f"expected [N, 1024] float32, got {rows.shape} {rows.dtype}")
    if not np.isfinite(rows).all():
        raise SystemExit("rows contain NaN or Inf")
    payload = struct.pack("<Q", rows.shape[0]) + rows.tobytes(order="C")
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_bytes(payload)
    print(f"wrote {rows.shape[0]} x 1024 to {args.out}")


if __name__ == "__main__":
    main()
