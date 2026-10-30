# QUALITY ASSURANCE & SYSTEM REVIEW WORKFLOWS

Follow these protocols to maintain spec compliance, code quality, and clear, durable issue reporting.

---

## 1. QUALITY ASSURANCE & ISSUE FILING

When the user describes problems during QA, or when planning a feature, translate them into durable, clear issues on the project issue tracker.

### 1.1. Rules for Filing Issues
- **Durable**: Issues should remain valid after major refactors.
- **Describe behaviors, not code**: Say *"the proxy fails to return 403 on piggybacked clauses"* instead of *"proxy.rs throws on line 142"*.
- **No file paths or line numbers**: These go stale rapidly.
- **Use the project's domain language**: terms as defined in `CONTEXT.md` (the ubiquitous-language glossary). Contracts and expected behavior come from the originating issue, ADR, or `architecture_spec.md`.
- **Reproduction steps are mandatory**: List concrete, numbered steps a developer can follow, including exact payloads, inputs, flags, or configuration.

### 1.2. Single Issue vs Breakdown
- **Single Issue**: Use when it is a single behavior wrong in one place, or multiple symptoms caused by the exact same root behavior.
- **Breakdown**: Break a task or report into multiple issues when:
  - The work spans multiple independent subsystems.
  - Slices are independently fixable and verifiable.
  - There are blocker/dependency relationships (e.g. Issue B cannot be tested until Issue A is completed).
- **Tracer Bullet Breakdown**: When breaking down a plan, create **vertical slices** (tracer bullets) that cut through all layers (schema -> core logic -> transport/API -> tests) and are demoable. Publish issues in dependency order (blockers first).

---

## 2. TWO-AXIS REVIEW WORKFLOW

To verify changes before merging, perform a review of the diff against `HEAD` along two distinct, independent axes:

### 2.1. The Two Review Axes
1. **Standards Axis**: Checks if the diff conforms to this repository's documented coding standards (`CODING_STANDARDS.md` when it exists, otherwise this protocol: [SKILL.md](./SKILL.md) §3, [code-design.md](./code-design.md), canonical linter, formatter, and static type/borrow checker rules).
2. **Spec Axis**: Checks if the diff faithfully implements the originating issue, spec, or PRD.
   - Detects missing/partial requirements.
   - Detects scope creep (behavior in the diff that wasn't asked for).
   - Detects incorrect implementations of requirements or numerical/architectural invariants.

### 2.2. Parallel Evaluation Protocol
- **Isolated passes**: Run Standards and Spec as two passes that do not see each other's notes. Each pass gets the diff plus only its own rubric. Parallel sub-agents are the default when the harness supports them.
- **Sequential fallback**: If the harness cannot run sub-agents, run Standards to completion, then Spec in a fresh pass. Do not carry findings from the first pass into the second.
- **Integration**: Aggregate both reports under `## Standards` and `## Spec` headers verbatim. Do not merge or rerank their findings, as a change can pass one axis while failing the other (e.g., standard-compliant code that implements the wrong feature).
- **Epistemic form**: only when the user triggered [epistemic.md](./epistemic.md). Tag the claims; still keep the two sections unmerged. A normal review stays in this format, without tags.
