---
task_id: "TASK-004"
status: "PENDING"
assigned_architect: "maximum-architect-agent"
protocol: "transparent-dev-protocol"
title: "Frontend Canonical Domain Selector in Playground"
created_at: "2026-11-01T12:00:00-03:00"
---

# TASK-004: Frontend Canonical Domain Selector in Playground

## 1. Objective
Replace the raw free-text `<input>` under "Domain id" in `frontend/src/ui/Playground.tsx` with a styled `<select>` dropdown populated with the 11 canonical domains of `ddi-fw` (`python`, `receta`, `legal`, `medicina`, `astronomia`, `finanzas`, `filosofia`, `musica`, `geologia`, `botanica`, `arquitectura`), preserving custom domains if present and guaranteeing zero regressions across existing Vitest suites.

## 2. Invariants & Constraints
- **Canonical Toolchain**: Use `pnpm` exclusively in `frontend/` (`pnpm test`, `pnpm run lint`, `pnpm run build`). Zero mixing of npm/yarn.
- **Visual Design System**: Use existing `fieldClass` styling adhering to studio dark palette without breaking viewport containment.
- **Testing Integrity**: 100% pass across Vitest suite with new tests asserting domain selection and request payload propagation.
- **Exclusive Authorship**: Sole author and collaborator in Git commits is `hbauzan` (`Héctor Andrés Bauzán Saavedra` <`hbauzan@gmail.com`>). No AI or bot co-authorship trailers.

## 3. Tracer Bullet Slices
- [ ] **Tracer Bullet 1 (Canonical Domain Constants)**: Export `CANONICAL_DOMAINS` and `CanonicalDomain` type from `frontend/src/domain/presets.ts`.
- [ ] **Tracer Bullet 2 (Playground Selector & Custom Domain Fallback)**: Replace `<input>` with `<select aria-label="Domain id">` in `frontend/src/ui/Playground.tsx`, mapping canonical domains and rendering custom domain fallback if loaded from session.
- [ ] **Tracer Bullet 3 (Verification & Regression Tests)**: Add unit test in `frontend/src/ui/Playground.test.tsx` asserting selection changes propagate to POST `/api/v1/chat` body; verify 100% pass across `pnpm test`, `pnpm run lint`, and `pnpm run build`.
