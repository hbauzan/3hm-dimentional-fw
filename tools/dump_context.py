#!/usr/bin/env python3
"""
Context & Codebase Dumper for Three-Headed Semantic / Dimensional Firewall (sf-core).
Generates 3hm-context.txt containing full architectural context, specifications,
domain ubiquitous language, configuration, and Rust source code for AI assistant analysis.
"""

import os
import sys
import argparse
from datetime import datetime
from pathlib import Path

SOURCE_EXTENSIONS = {
    ".rs", ".toml", ".proto", ".md", ".json", ".py", ".sh", ".yaml", ".yml", ".html", ".css", ".js"
}

BINARY_EXTENSIONS = {
    ".png", ".jpg", ".jpeg", ".gif", ".ico", ".pdf", ".zip", ".tar", ".gz",
    ".npz", ".lance", ".arrow", ".bin", ".dat"
}

EXCLUDE_DIRS = {
    ".git", "target", ".cargo", "data", ".agents", "skills", ".gemini", ".claude",
    ".DS_Store", "__pycache__", "reports/archive"
}

EXCLUDE_FILES = {
    "3hm-context.txt", "3hm_dev.log", "sf_firewall_dev.log", "Cargo.lock"
}

PRIORITY_DOCS = [
    "manifest.json",
    "README.md",
    "CONTEXT.md",
    "CONTEXT-MAP.md",
    "architecture_spec.md",
    "Cargo.toml",
    "rust-toolchain.toml",
    ".env.example",
    "roadmap/deterministic-ai-roadmap.md",
    "roadmap/architect-sf-ui-execution-report.md"
]

def is_source_file(path: Path) -> bool:
    if path.name in EXCLUDE_FILES:
        return False
    if path.suffix.lower() in BINARY_EXTENSIONS:
        return False
    if path.suffix.lower() in SOURCE_EXTENSIONS:
        return True
    return False

def should_skip_dir(dirname: str) -> bool:
    return dirname in EXCLUDE_DIRS or dirname.startswith(".")

def gather_files(root: Path):
    priority_paths = []
    for rel_str in PRIORITY_DOCS:
        p = root / rel_str
        if p.is_file():
            priority_paths.append(p)

    seen = set(priority_paths)
    other_files = []

    for current_root, dirs, files in os.walk(root):
        dirs[:] = [d for d in dirs if not should_skip_dir(d)]
        for f in sorted(files):
            full_path = Path(current_root) / f
            if full_path in seen:
                continue
            if is_source_file(full_path):
                seen.add(full_path)
                other_files.append(full_path)

    return priority_paths + sorted(other_files)

def dump_codebase(root: Path, output_file: Path):
    all_files = gather_files(root)
    total_bytes = 0
    total_lines = 0

    with open(output_file, "w", encoding="utf-8", errors="replace") as out:
        out.write("===============================================================================\n")
        out.write("THREE-HEADED SEMANTIC FIREWALL (sf-core) — CODEBASE CONTEXT DUMP\n")
        out.write(f"Generated on: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n")
        out.write(f"Total candidate files: {len(all_files)}\n")
        out.write("«¡Mira detrás de ti, un mono de tres cabezas!» — Lucasfilm Games 1990\n")
        out.write("===============================================================================\n\n")

        for fpath in all_files:
            rel = fpath.relative_to(root)
            try:
                content = fpath.read_text(encoding="utf-8", errors="replace")
                lines = content.splitlines()
                total_lines += len(lines)
                total_bytes += len(content.encode("utf-8"))

                out.write(f"\n{'=' * 78}\n")
                out.write(f"FILE: {rel}\n")
                out.write(f"LINES: {len(lines)}\n")
                out.write(f"{'=' * 78}\n")
                out.write(content)
                if not content.endswith("\n"):
                    out.write("\n")
            except Exception as e:
                out.write(f"\n[ERROR reading {rel}: {e}]\n")

    print(f"Dump complete: {len(all_files)} files, {total_lines} lines, {total_bytes / 1024:.1f} KB -> {output_file}")

def main():
    parser = argparse.ArgumentParser(description="Three-Headed Semantic Firewall Codebase Dumper")
    parser.add_argument("--output", "-o", default="3hm-context.txt", help="Output text file path")
    args = parser.parse_args()

    root = Path(__file__).resolve().parent.parent
    output_path = root / args.output
    dump_codebase(root, output_path)

if __name__ == "__main__":
    main()
