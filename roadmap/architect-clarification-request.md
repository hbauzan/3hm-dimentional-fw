# sf-core — Pedido de clarificación al agente arquitecto

Estado: **RESUELTO**. Decisiones del arquitecto registradas el 2026-10-29. El Slice 0 arranca en `feat/sf-core-3tier-cascade`.

El invariante de pre-ejecución obliga a parar ante discrepancia, dependencia ausente o ambigüedad de contrato. Abajo hay 8 decisiones bloqueantes. Cada una trae la evidencia ya medida y las opciones cerradas. La respuesta esperada es el bloque YAML del final, una opción por ítem, sin reabrir lo que ya está verificado.

Audiencia: agente arquitecto que mantiene `roadmap/deterministic-ai-roadmap.md` y `roadmap/Arquitect-initial-instrucctions.md`.

---

## Hechos ya cerrados (no reabrir)

| Afirmación | Evidencia |
| :--- | :--- |
| `BAAI/bge-m3` emite \(\mathbb{R}^{1024}\) | Hub `config.json`: `hidden_size = 1024`, `torch_dtype = float32`. Pooling: `word_embedding_dimension = 1024`. |
| Layout contractual | \(D = 1024\) × 4 B = 4096 B, `align(64)`. Índices universales `[292, 297, 308, 329, 386, 404, 577, 616, 780]` → 1015 activas. 985 = 1015 − 30 por `--extra-pruned-dims`. |
| `lancedb 0.40.0` existe y no está yanked | crates.io, 2026-10-07. `rust-version = 1.91.0`. Depende de `arrow-* ^58.0.0` y de `lance ==13.0.0` (`rust-version = 1.91.0`). `default = []`. `datafusion ^54` es dependencia normal, no feature-gated. |
| Re-export Arrow | `lancedb-0.40.0/src/arrow.rs`: `pub use arrow_array; pub use arrow_buffer; pub use arrow_schema;`. El path `lancedb::arrow::{arrow_array, arrow_buffer, arrow_schema}` es real. |
| `arrow-array 58.0.0` | crates.io, `rust-version = 1.85`, no yanked. Línea 58 llega a `58.4.1`. `arrow-array` max publicado: `60.1.0`. Cargo trata `"58.0.0"` como `^58.0.0` y puede resolver `58.4.x` si nadie exige `=58.0.0`. |
| Pins del roadmap que sí existen | `tonic 0.12.3`, `prost 0.13.4`, `axum 0.8.1` (`tower ^0.5.2`), `tokio 1.43.0`, `arc-swap 1.7.1`, `criterion 0.5.1`, `rand_chacha 0.3.1`, `reqwest 0.12.12`, `hdrhistogram 7.5.4`, `sha2 0.10.8`. |
| TEI upstream | `huggingface/text-embeddings-inference` `proto/tei.proto`: `package tei.v1`; `rpc EmbedStream (stream EmbedRequest) returns (stream EmbedResponse)`; `EmbedRequest.normalize = 3` (`optional bool`). |
| Host de build | Apple M4, `arm64`, `hw.optional.neon = 1`, `hw.cachelinesize = 128`, 10 cores, 16 GiB. `rustc 1.94.1` / `cargo 1.94.1`. Único target rustup: `aarch64-apple-darwin`. AVX-512 y AVX2 no existen en este CPU. |
| Git | `origin` = `https://github.com/hbauzan/3hm-dimentional-fw.git`. Rama local y remota: `main`. |

`prost ^0.14` y `tonic` dentro de lancedb están detrás del feature `remote`. Con `default-features = false` no entran al grafo. El pin `prost = 0.13.4` del proxy no choca mientras `remote` permanezca apagado.

---

## Q1 — Toolchain: canal `1.82` contra MSRV `1.91.0`

**Bloquea:** Slice 0 (`rust-toolchain.toml` + primer `cargo check`).

| Fuente | Texto |
| :--- | :--- |
| Roadmap S0-01, línea 111 | `rust-toolchain.toml` fijando canal `1.82`. `Cargo.toml` declara `rust-version = "1.82"`. |
| Goal | Rust 1.82+ (2021). |
| crates.io | `lancedb 0.40.0` y `lance 13.0.0` declaran `rust-version = 1.91.0`. `arrow-array 58.0.0` declara `1.85`. `datafusion 54.0.0` / `54.1.0` declaran `1.88.0`. |
| Host | `rustc 1.94.1` ya instalado. `1.91.0` no está instalado; rustup tendría que bajarlo. |

Un canal pineado a `1.82` no cumple el `rust-version` declarado por `lancedb 0.40.0`. Cambiar el pin de lancedb/arrow queda fuera de esta pregunta: esos pins se mantienen.

Elegí una:

- `msrv-1.91` — `rust-toolchain.toml` canal `1.91.0` (MSRV declarado de lancedb 0.40.0 / lance 13.0.0).
- `host-1.94` — canal `1.94.1` (compilador ya instalado; cumple 1.91 y el piso 1.82+).
- `keep-1.82` — conservar canal `1.82` y no tocar los pins. El workspace no compila; el halt permanece.

## Q2 — Semántica de `X-Semantic-Domain: *`

**Bloquea:** Slice 4, router y `cascade_e2e_test`.

| Fuente | Contrato |
| :--- | :--- |
| `Arquitect-initial-instrucctions.md:906` | `"*"` exige pasar contra **todos** los locks activos del set canónico. |
| Roadmap S4-03, línea 252, y el goal | Default `AnyAllowlist`: unión consistente (prompt + todas las cláusulas pasan al menos un lock allowlist; en fallo se reporta el lock más cercano). `AllLocks` solo con `X-SF-Wildcard-Mode: all`. |
| Acuerdo en ambas fuentes | Denylist `polarity == 1`: intersección estricta (todas deben pasar). |

Elegí una:

- `any-allowlist` — default `AnyAllowlist`; `AllLocks` solo con `X-SF-Wildcard-Mode: all`.
- `all-locks` — `*` significa que todo lock activo debe pasar. No hay default `AnyAllowlist`.

## Q3 — Bit-exactitud del coseno (0 ULP) frente a no-asociatividad

**Bloquea:** Slice 2, `simd_parity_fuzz` y el DoD.

| Fuente | Contrato |
| :--- | :--- |
| Roadmap Slice 2, línea 169, y DoD línea 302 | `assert_eq!(simd_bits, scalar_bits)` en 10 000 vectores. Criterio: 0 ULP. |
| Architect §0.1, línea 16 | La suma IEEE 754 no es asociativa. Reducciones horizontales `zmm` (AVX-512) vs NEON pueden diferir. Con acumulación `f64` el drift cross-ISA queda por debajo de \(10^{-14}\), no en 0 ULP. |
| Kernels de máscara | Comparación por coordenada (`cmp` / `vcgeq` / `vcleq`). El bitmask puede ser 0 ULP en las tres ISA porque no hay reducción de `f64`. |
| Anchos | AVX-512 convierte 8×`f32`→`f64` por `cvtps_pd`. AVX2 convierte 4. NEON convierte 2+2. Un único scalar serial con dos acumuladores (`i += 2`, `mul_add`) no reproduce las tres asociaciones a la vez. |

Elegí una:

- `host-tree` — 0 ULP del SIMD activo contra el scalar que transcribe **el árbol de reducción de esa ISA** (en este host, NEON). Cross-ISA del coseno puede diferir dentro del bound \(10^{-14}\). Máscaras de hipercubo: 0 ULP en todas las ISA.
- `global-tree` — una sola asociación `f64`. AVX-512, AVX2, NEON y scalar deben dar el mismo `to_bits()`, también en un futuro run x86_64.
- `mask-only` — 0 ULP solo para el bitmask. El fuzz del coseno afirma \(|\mathrm{simd}-\mathrm{scalar}| < 10^{-14}\), sin igualdad de bits.

## Q4 — NeMo Guardrails dentro o fuera del Slice 5

**Bloquea:** definición de “Slice 5 verde” y el approval gate.

| Fuente | Alcance |
| :--- | :--- |
| Architect M4-02, ~líneas 1617–1628 | Load test comparativo contra `nemoguardrails server` v0.24.1 (topical input rails), con bypass, p50/p99 y RSS del sistema de referencia. |
| Roadmap S5 y el goal | Fixtures `ChaCha8Rng` seed `0xDD10_0400`, 55 locks, 220 vectores, offline, sin red ni LFS. Bins: `rompepepe_eval`, `load_benchmark`. DoD sin NeMo. |

Elegí una:

- `out` — NeMo queda fuera. Slice 5 = fixtures + `rompepepe_eval` + `load_benchmark`.
- `in` — el comparativo NeMo v0.24.1 es parte del gate y hay que ejecutarlo antes de pedir aprobación.

## Q5 — Crates que los slices nombran y el workspace no pinea

**Bloquea:** Slice 0 (`Cargo.toml`) y los bins/tests que las usan.

El roadmap nombra `clap` (S3-02), casteo `f16` (S2-02), `tonic-build` / `prost-build` (S4-02) y `tower::ServiceExt` (S1-02). No están en `[workspace.dependencies]`.

Versiones publicadas y compatibles con el grafo ya pineado:

| Crate | Versión comprobada | Para qué |
| :--- | :--- | :--- |
| `clap` | `4.5.23` (`rust-version 1.74`) | CLI `sf-ingest` |
| `half` | `2.4.1` (`rust-version 1.70`) | Contraejemplo `f16` de RFC-003, solo tests |
| `tonic-build` | `0.12.3` | `build.rs` de `tei.proto` |
| `prost-build` | `0.13.4` | mismo `build.rs`, alineado a `prost 0.13.4` |
| `tower` | `0.5.2` | `axum 0.8.1` exige `tower ^0.5.2`; los tests e2e usan `ServiceExt` |

Arrow: el import en `sf-storage` sigue siendo solo `lancedb::arrow::{arrow_array, arrow_buffer, arrow_schema}`. Un `=58.0.0` directo existe para que el resolver no flote a `58.4.x`; el `use` no apunta al crate `arrow-*` suelto.

Elegí una:

- `pin-extras` — agregar los cinco pins (`tower = 0.5.2`) y forzar `arrow-array` / `arrow-buffer` / `arrow-schema` a `=58.0.0`.
- `no-extras` — sin crates nuevos de runtime: parser de argv a mano, binary16 a mano, y solo `tonic-build` / `prost-build` (pareja de tonic/prost). `tower` entra transitivo vía axum. Arrow queda en el caret del roadmap (`"58.0.0"` → puede resolver `58.4.x`).

## Q6 — Qué cuenta como Slice 2 verde en este Apple M4

**Bloquea:** el comando `cargo test -p sf-kernels --test simd_parity_fuzz` y el criterio de cierre del slice.

Hecho de hardware: Apple M4, NEON presente, línea de caché del sysctl = 128 B, sin AVX-512 ni AVX2, sin target `x86_64-apple-darwin` instalado. El contrato de layout sigue en `align(64)` (múltiplo válido para `_mm512_load_ps` y para `vld1q`). Los kernels x86 pueden vivir bajo `cfg(target_arch = "x86_64")` y este `cargo test` no los ejecuta.

Elegí una:

- `neon-green` — Slice 2 queda verde cuando NEON ≡ scalar en este M4. AVX-512 y AVX2 se entregan como fuente cfg-gated, sin corrida x86_64 en este gate.
- `x86-required` — Slice 2 no se declara verde hasta correr `simd_parity_fuzz` en un host `x86_64`. Si no hay runner, el trabajo se detiene con las fuentes escritas.

## Q7 — Dónde viven `vector_dim` y `alignment_bytes` en `manifest.json`

**Bloquea:** Slice 0, tarea S0-02.

| Fuente | Ubicación |
| :--- | :--- |
| Roadmap S0-02, líneas 119–133 | `state_schema`: `vector_dim`, `default_quorum_k`, `alignment_bytes`. `constraints`: `p99_latency_ms`, `max_concurrency_rps`, `max_rss_mb`. |
| Goal, Slice 0 | Lista esos dos campos dentro de `constraints`, junto a p99 / RPS / RSS. |

Elegí una:

- `roadmap-json` — JSON del roadmap verbatim. `vector_dim` y `alignment_bytes` viven en `state_schema`.
- `both` — `state_schema` del roadmap y, además, duplicar `vector_dim` y `alignment_bytes` dentro de `constraints`.
- `constraints-only` — esos dos campos solo en `constraints`, como el paréntesis del goal.

## Q8 — RSS &lt; 45 MB con DataFusion dentro del proceso

**Bloquea:** Slice 5, aserción de `load_benchmark`, y `manifest.constraints.max_rss_mb`.

Hecho: `lancedb 0.40.0` depende de `datafusion ^54` con `optional = false`. `default-features = false` no lo elimina. El grafo del roadmap hace que `sf-proxy` enlace `sf-storage` y por tanto LanceDB. El RSS de ese binario no está medido. La hipótesis falsable: el working set de DataFusion deja el proceso en o por encima de 45 MB antes de servir las 1 000 RPS, y la aserción de SLA falla con el grafo tal como está escrito.

Elegí una:

- `measure` — conservar el grafo del roadmap. Medir RSS en el Slice 5. Si RSS ≥ 45 MB, parar en el approval gate con el número. Sin rediseño a mitad de los slices.
- `split` — antes del Slice 5, el proceso de `load_benchmark` no enlaza LanceDB/DataFusion (registry en memoria). Los 45 MB se miden sin DataFusion.

---

## Respuesta (rellenar y devolver)

Una clave por pregunta. Valores válidos: los ids de cada Q. Nada de prosa nueva salvo `notes` si una opción no alcanza.

```yaml
q1_toolchain: host-1.94
q2_wildcard: any-allowlist
q3_ulp: mask-only
q4_nemo: out
q5_extra_deps: pin-extras
q6_x86_gate: neon-green
q7_manifest: roadmap-json
q8_rss: measure
notes: "Q1 usa host-1.94 instalado. Q3 0 ULP en masks y bound 1e-14 en coseno por IEEE 754. Q8 mide en Slice 5 con evidencia empírica antes de rediseñar. Q5 Arrow quedó en =58.4.1: datafusion 54 exige arrow-schema ^58.3.0, así que =58.0.0 no resuelve."
```

Decisiones aplicadas: toolchain `1.94.1`; wildcard `AnyAllowlist` ( `AllLocks` solo con `X-SF-Wildcard-Mode: all` ); coseno con \(|\Delta| < 10^{-14}\) y bitmask a 0 ULP; NeMo fuera de alcance; pins extra y Arrow `=58.0.0`; Slice 2 verde con NEON en este M4; `manifest.json` verbatim del roadmap; RSS medido en el Slice 5 sin rediseño previo.
