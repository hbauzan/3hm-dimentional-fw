# DOCUMENTATION SYNCHRONIZATION WORKFLOW

Update documentation **only when the change actually affects that asset**. Do not touch every file on every micro-fix — that wastes tokens, creates noisy diffs, and causes context drift.

These documents are the recovery core of the codebase. Keep them accurate; do not keep them busy.

| File | Update when… | Do **not** update when… |
| :--- | :--- | :--- |
| **`manifest.json`** | Version bumps, or `state_schema` / `constraints` change (new config field, default, range, memory or token limit). | Pure refactors, bug fixes that leave the config contract unchanged. |
| **`CHANGELOG.md`** | Releases or **notable** capability changes (new backend, API surface, breaking change, security posture). Append a short section. | Every minor commit, typo fix, or internal cleanup. |
| **`architecture_spec.md`** | Contracts change: API shapes, provider interfaces, memory/data schemas, concurrency models, security/fail-closed policies, performance invariants. | Implementation details that stay within an existing contract. |
| **`README.md`** | How to install, configure, or run the system changes (toolchains, build commands, prerequisites). | Internal code changes that do not affect setup or usage. |
| **`CONTEXT.md`** | Domain language changes (new term, renamed concept, retired alias). This file **is** the ubiquitous-language glossary. | Code-only changes that use existing terms. |

### Manifest shape (slim)

`manifest.json` holds **current state only**:

- `project`, `version`
- `state_schema` (live configuration contract)
- `constraints` (e.g. `p99_latency_ms`, `max_input_tokens`, `request_timeout_seconds`)

It is **not** a historical feature-flag ledger. Capability history lives in `CHANGELOG.md`.

### Agent handoff

A handoff is a fresh-session document that points at paths (PRDs, ADRs, specs, issues, diffs). Write it when [SKILL.md](./SKILL.md) §4 says to compact between phases. Do not generate a concatenated mega-dump of code, and do not invent pack scripts that are not in the repo.

---

## CONTEXT & BLUEPRINT WORKFLOW

By default, the codebase domain context is managed in a glossary format to ensure clean, ubiquitous domain definitions. However, a specialized blueprint mode exists for tracking codebase layout.

### 1. CONTEXT.md (Standard Run - Default)
Behaves strictly as a **Domain Model Glossary & Ubiquitous Language** reference, devoid of code or implementation details.

Canonical name: `CONTEXT.md`, paired with `CONTEXT.blueprint.md` and `CONTEXT-MAP.md`. If a repo already has `UBIQUITOUS_LANGUAGE.md` and no `CONTEXT.md`, use that existing file as this asset. Do not create a second glossary and do not rename the protocol's canonical file.

- **Structure**: Define terms precisely under subheadings. Keep definitions tight (1-2 sentences max defining what a concept *is*, not what it *does*).
- **Aliases**: Be opinionated. If multiple words exist for the same concept, pick the canonical term and list the others under an `_Avoid_` section.
- **Relationships**: Show bold term names and express cardinality/relationships between concepts.
- **Context Mapping**: For repositories with multiple subdomains, crates, or modules, a `CONTEXT-MAP.md` at the root maps out each context (e.g. `[Storage](./crates/sf-storage/CONTEXT.md)`) and their relationships.
- **Update Frequency**: Update terms inline as decisions are made; do not batch glossary updates. Skip entirely if no domain terms changed.

### 2. CONTEXT.blueprint.md (Bootstrap Run)
The **Bootstrap Run** is a specialized mode containing a comprehensive codebase re-generation blueprint.

- **Activation**: Activated in one of two ways:
  1. Setting `"bootstrap_run": true` in `manifest.json`.
  2. Explicit declaration by the user in the prompt.
- **Output File**: Write/update the blueprint in a separate file: **`CONTEXT.blueprint.md`**.
- **Content Requirements**:
  - File-by-file inventory and mapping.
  - Complete structural dependencies between modules/crates.
  - Technical debt analysis and context anchors.
  - Absolute context density to enable repository regeneration from zero.
