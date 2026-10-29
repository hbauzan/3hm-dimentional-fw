# CODEBASE DESIGN AND IMPLEMENTATION PRINCIPLES

All codebase changes should follow deep module design principles and a vertical, test-driven implementation workflow (tracer bullets).

---

## 1. DEEP MODULE DESIGN

Aim to design **deep modules**: modules that put a lot of behavior behind a small interface, placed at a clean seam, testable through that interface.

### 1.1. Glossary of Terms
Do not substitute generic terms (like "component," "service," "API," or "boundary") for these design elements:
- **Module**: Anything with an interface and an implementation (function, struct, class, crate, package, or tier-spanning slice).
- **Interface**: Everything a caller must know to use the module correctly (types, invariants, ordering constraints, error modes, configuration, and performance characteristics).
- **Implementation**: What is inside the module (its body of code).
- **Adapter**: A concrete object/code that satisfies an interface at a seam. Describes the *role*, not substance.
- **Depth**: Leverage at the interface. A module is **deep** when a large amount of behavior sits behind a small interface. It is **shallow** when the interface is as complex as the implementation (avoid shallow modules).
- **Seam**: A place where you can alter behavior without editing in that place (the location of a module's interface).
- **Leverage**: Capability per unit of interface learned (one implementation pays back across N call sites).
- **Locality**: Concentration of change, bugs, and knowledge in one place rather than spreading across callers.

### 1.2. Design Principles
- **Depth is a property of the interface, not the implementation**: A deep module can be internally composed of small, mockable, swappable parts, as long as they remain private to its implementation.
- **The Deletion Test**: If you delete the module and the complexity vanishes, it was a pass-through (shallow). If the complexity reappears across N callers, it was earning its keep (deep).
- **The interface is the test surface**: Callers and tests cross the same seam. If you need to test past the interface, the module is probably the wrong shape.
- **One adapter = hypothetical seam; Two adapters = real seam**: Don't introduce interfaces or seams unless something actually varies across them.
  - *Canonical LLM/Model example*: a **local** model backend (e.g. TEI / Ollama / in-process) and a **remote** API (e.g. OpenAI / Anthropic) are two adapters behind one provider interface — that is a real seam, so the abstraction earns its keep. A single provider with no alternative does not (yet) justify one. See [SKILL.md](./SKILL.md) §3.2.
- **Concurrency & Boundary Validation**:
  - *Streaming & Concurrency*: When streaming tokens, processing batches, or running concurrent tasks, provider calls must use non-blocking asynchronous clients to avoid starving thread pools or stalling the event loop/scheduler.
  - *Boundary Validation*: Validate raw external outputs (JSON, Protobuf, model strings) at the adapter boundary using schema validators, typestate, or strict deserializers *before* constructing domain objects. Domain types must never ingest raw unvalidated external representations.
- **Design for Testability**:
  1. *Accept dependencies, don't create them* (pass collaborators in).
  2. *Return results, don't produce side effects* where possible (pure computations).
  3. *Small surface area* (fewer methods/parameters = simpler setup and fewer tests).

---

## 2. IMPLEMENTATION WORKFLOW: VERTICAL SLICES

### 2.1. Tracer Bullets vs Horizontal Slicing (Anti-Pattern)
- **Anti-Pattern (Horizontal Slicing)**: Writing all tests first, then writing all implementation code, or implementing layer-by-layer (e.g. database/kernels first, then domain logic, then API/proxy, then benchmarks). This leads to late integration failures, fragile interfaces, and components that check imagined shapes rather than actual behavior.
- **Correct Approach (Vertical Slicing)**: Implement features via **tracer bullets**. Each issue or step is a thin vertical slice cutting through all integration layers (schema, core logic, transport/API, tests) end-to-end. One slice should be demoable and verifiable immediately.

```
WRONG (Horizontal Slicing):
  RED:   test1, test2, test3, test4
  GREEN: impl1, impl2, impl3, impl4
  (Risk: No working end-to-end system until the very final step)

RIGHT (Vertical Slicing / Tracer Bullets):
  RED→GREEN: slice1_test → slice1_impl (End-to-end working skeleton)
  RED→GREEN: slice2_test → slice2_impl (Hardened feature / acceleration)
  RED→GREEN: slice3_test → slice3_impl (Persistence / multi-domain)
```

---

## 3. TEST-DRIVEN DEVELOPMENT (TDD)

Tests must verify behavior through public interfaces, not internal implementation details.

### 3.1. The TDD Cycle
1. **Planning**:
   - Confirm interface changes and behavior priorities with the user.
   - List the behaviors to test (not implementation steps).
2. **Tracer Bullet**:
   - Write ONE test for the first behavior -> Watch it fail (**RED**).
   - Write the minimal code to pass -> Watch it pass (**GREEN**).
3. **Incremental Loop**:
   - For each remaining behavior, repeat: Write next test (RED) -> Minimal code to pass (GREEN). Do not anticipate future tests or write speculative features.
4. **Refactor**:
   - Extract duplication, deepen modules, and apply SOLID principles.
   - **Never refactor while RED**. Get to GREEN first, then refactor, running the canonical test runner after each step.
