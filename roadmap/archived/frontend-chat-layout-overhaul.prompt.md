# TASK SPECIFICATION & LATENT CONTEXT: FRONTEND CHAT LAYOUT OVERHAUL (OPTION A)

## 1. Project Context & Current State

* **Repository**: `3hm-dimentional-fw` (Three-Headed Semantic Firewall).
* **Architecture**:
  * **Backend Gateway**: Rust `sf-proxy` running on port `8088` (`http://127.0.0.1:8088/api/v1/chat`).
  * **Frontend**: React 19 + TypeScript + Vite + Tailwind CSS v4 in `frontend/` running on port `5173` (`http://127.0.0.1:5173`).
* **Canonical Toolchain**:
  * Root/Rust: `cargo` (exclusively).
  * Frontend: `pnpm` (exclusively, version >= 9). Do NOT use `npm` or `yarn`.
* **Current Quality Baseline** (Empirically verified):
  * Tests: `pnpm test` executes Vitest across 9 files (35 tests, all green).
  * TypeCheck & Linter: `pnpm tsc -b && pnpm oxlint` (0 errors, 0 warnings).

---

## 2. Objective & Scope

Refactor `frontend/src/ui/Playground.tsx` to transform the current document-scrolling page layout into a modern, full-viewport conversational studio interface:
1. **Full Viewport Containment (`100dvh`)**: Eliminate window-level body scrolling. The entire UI must fit cleanly in `100dvh` (`overflow-hidden`).
2. **Pinned-Bottom Chat Dock**: The message history flows above, while the prompt input bar and prompt preset chips remain permanently anchored at the bottom (`flex-shrink-0`), mimicking ChatGPT / Claude ergonomics.
3. **Smart Auto-scroll**: Auto-scroll smoothly to the bottom on new incoming SSE stream deltas and turn insertions, but automatically freeze auto-scrolling if the user has manually scrolled up to inspect previous turns or breach telemetry cards.
4. **Ultra-Dense Fixed Right Panel (Option A)**:
   * Constrain panel width to **`w-60` (240px / 15rem)** (reduced from legacy 352px).
   * Fixed in place with its own independent vertical scroll (`overflow-y-auto`).
   * High-density typography and controls (`text-[10px]` / `text-[11px]`, compact inline sliders, tight key-value pairings).
5. **Zero Test Regressions**: Every single accessible name, role, label, and input id required by `frontend/src/ui/Playground.test.tsx` must be preserved verbatim.

---

## 3. Ground Truth: Files & Contracts

### 3.1 Primary File to Modify
* `frontend/src/ui/Playground.tsx`

### 3.2 Testing Contract (Do NOT break these selectors)
Inspected from `frontend/src/ui/Playground.test.tsx`:
| Control / Target | Selector used in Vitest | Invariant Requirement |
| :--- | :--- | :--- |
| Gateway Base input | `screen.getByRole('textbox', { name: 'Gateway base' })` | Accessible name must remain `"Gateway base"` |
| Domain ID input | `screen.getByRole('textbox', { name: 'Domain id' })` | Accessible name must remain `"Domain id"` |
| Prompt textarea | `screen.getByRole('textbox', { name: 'Prompt' })` | Must retain `aria-label="Prompt"` |
| Send button | `screen.getByRole('button', { name: 'Send' })` | Text / name must remain `"Send"` (or `"Sending"` when busy) |
| Provider select | `screen.getByRole('combobox', { name: 'Provider' })` | Accessible name must remain `"Provider"` |
| Base URL input | `screen.getByRole('textbox', { name: 'Base URL' })` | Accessible name must remain `"Base URL"` |
| Model input | `screen.getByRole('textbox', { name: 'Model' })` | Accessible name must remain `"Model"` |
| API key input | `screen.getByLabelText('API key')` | Label text must remain `"API key"` |
| Overrides checkbox | `screen.getByRole('checkbox', { name: 'Apply overrides' })` | Must retain accessible name `"Apply overrides"` |
| L1 slider | `screen.getByRole('slider', { name: /L1 cosine threshold/ })` | Must retain `aria-label` matching `"L1 cosine threshold"` |
| Env sync button | `screen.getByRole('button', { name: 'Cargar de .env' })` | Must retain `aria-label="Cargar de .env"` |
| Env checkbox | `screen.getByRole('checkbox', { name: 'Usar keys de .env' })` | Must retain `aria-label="Usar keys de .env"` |
| Endpoint display | `screen.getByText('POST http://127.0.0.1:8088/api/v1/chat')` | Endpoint status text in header must remain rendered |
| Preset buttons | `screen.getByRole('button', { name: 'Risotto L1' })` | Preset labels rendered from `PROMPT_PRESETS` |

### 3.3 Design Tokens (Tailwind v4 in `frontend/src/index.css`)
* `--color-studio-bg`: `#0b0f17`
* `--color-studio-panel`: `#111827`
* `--color-studio-card`: `#161f30`
* `--color-studio-line`: `#273549`
* `--color-studio-text`: `#f1f5f9`
* `--color-studio-muted`: `#94a3b8`
* `--color-studio-accent`: `#38bdf8`
* `--color-studio-pass`: `#10b981`
* `--color-studio-breach`: `#ef4444`
* `--color-studio-warn`: `#f59e0b`

---

## 4. Architectural Layout Specification

```text
Root (<div className="h-[100dvh] flex flex-col overflow-hidden bg-studio-bg text-studio-text">)
├── <header className="flex-shrink-0 flex items-center justify-between border-b border-studio-line px-4 py-2.5">
│     SF-PROXY PLAYGROUND [Title + Endpoint Status]
└── <main className="flex-1 flex overflow-hidden">
      ├── <section aria-label="Chat" className="flex-1 flex flex-col overflow-hidden min-w-0">
      │     ├── <ol aria-label="Messages" className="flex-1 overflow-y-auto overscroll-contain px-4 py-4 flex flex-col gap-3">
      │     │     {turns.map(...)}
      │     │     <div ref={messagesEndRef} />
      │     ├── <div className="flex-shrink-0 flex flex-wrap gap-1.5 px-3 py-1.5 border-t border-studio-line bg-studio-panel/40">
      │     │     {PROMPT_PRESETS.map(...)}
      │     └── <form onSubmit={onSubmit} className="flex-shrink-0 flex gap-2 border-t border-studio-line p-3 bg-studio-panel">
      │           <textarea aria-label="Prompt" rows={2} ... />
      │           <button type="submit">Send</button>
      └── <aside aria-label="Live tuning" className="flex-shrink-0 w-60 border-l border-studio-line bg-studio-panel overflow-y-auto p-3 flex flex-col gap-2.5">
            [High-density parameters & upstream controls]
```

### 4.1 Auto-scroll Stickiness Logic
Implement in `Playground.tsx`:
```tsx
const messagesContainerRef = useRef<HTMLOListElement>(null)
const isAtBottomRef = useRef(true)

function handleScroll() {
  const el = messagesContainerRef.current
  if (!el) return
  const threshold = 40 // px tolerance
  isAtBottomRef.current = el.scrollHeight - el.scrollTop - el.clientHeight <= threshold
}

useEffect(() => {
  if (isAtBottomRef.current) {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }
}, [turns])
```

### 4.2 Compact Tuning Panel Specs
* Width: `w-60` (15rem / 240px).
* Labels: `text-[10px] tracking-wider uppercase text-studio-muted`.
* Sliders: compact layout with label + value in a single line, slider track height `h-1.5` or `h-1`.
* Inputs: `fieldClass` with `py-0.5 px-1.5 text-xs`.
* Preserves all form state sync (`commit`, `tune`, `syncEnv`).

---

## 5. Development Protocol & Acceptance Criteria

### 5.1 Protocol Invariants
* **Git Branch**: Create `feat/chat-layout-overhaul` from `main` before editing.
* **Exclusivity of Authorship**: Commits must strictly set author `hbauzan` (`Héctor Andrés Bauzán Saavedra` <`hbauzan@gmail.com`>). Absolutely NO co-author trailers (`Co-authored-by:`) and NO AI/bot attributions.
* **Approval Gate**: Stop after running all verification checks; report exact diff and test outputs. Do NOT push or merge without explicit human approval.

### 5.2 Verification Commands
Run inside `frontend/`:
1. `pnpm test` -> Must pass all 9 test suites / 35 tests.
2. `pnpm tsc -b` -> Must exit with code 0 (zero type errors).
3. `pnpm oxlint` -> Must exit with code 0 (zero lint warnings/errors).

### 5.3 Definition of Done
1. [ ] Outer browser window has no scrollbar (`100dvh` fixed).
2. [ ] Chat input and presets remain pinned at the bottom at all times.
3. [ ] Right tuning panel is fixed at `w-60` (240px) with independent vertical scroll.
4. [ ] Streaming SSE responses auto-scroll down smoothly, but freeze if the user scrolls up.
5. [ ] 100% of Vitest tests in `frontend/src/ui/Playground.test.tsx` pass without modifying test files.
6. [ ] Zero linter or type errors.
