---
task_id: "TASK-001"
status: "DONE"
assigned_architect: "maximum-architect-agent"
protocol: "transparent-dev-protocol"
title: "Frontend Chat Layout Overhaul (Option A - Dense Studio)"
created_at: "2026-10-31T10:35:00-03:00"
---

# TASK-001: Frontend Chat Layout Overhaul (Option A - Dense Studio)

## 1. Objective
Refactor `frontend/src/ui/Playground.tsx` from a document-scrolling layout into a modern full-viewport (`100dvh`) conversational studio interface with pinned-bottom chat dock, smart auto-scrolling with user-scroll freeze detection, and an ultra-dense fixed right live-tuning panel (`w-60` / 240px).

## 2. Invariants & Constraints
- Canonical toolchain: `pnpm` exclusively in `frontend/`.
- Zero test regressions: 100% pass on Vitest test suite (`frontend/src/ui/Playground.test.tsx`), preserving every accessible name, role, label, and input id verbatim.
- Full viewport containment: Outer browser window has no scrollbar (`h-[100dvh] overflow-hidden`).
- Exclusive authorship: Sole author and collaborator is `hbauzan` (`Héctor Andrés Bauzán Saavedra` <`hbauzan@gmail.com`>). No bot or co-author trailers.

## 3. Tracer Bullet Slices
- [x] **Tracer Bullet 1 (Full Viewport Containment)**: Root layout contained within `h-[100dvh] overflow-hidden` with fixed header and non-scrolling document body.
- [x] **Tracer Bullet 2 (Pinned-Bottom Dock)**: Message list flexes with independent vertical scroll (`overflow-y-auto overscroll-contain`), while prompt presets and prompt input dock remain permanently pinned at the bottom.
- [x] **Tracer Bullet 3 (Smart Auto-scroll)**: Smooth auto-scroll to latest turn on SSE delta/message updates, automatically freezing auto-scroll when user manually scrolls up past threshold.
- [x] **Tracer Bullet 4 (Compact Right Panel w-60)**: Ultra-dense live tuning panel constrained to `w-60` (240px) with independent vertical scroll, high-density typography, compact sliders, and complete form synchronization.
