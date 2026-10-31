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
