import { useRef, useState, type FormEvent, type KeyboardEvent } from 'react'
import { resolveChatEndpoint } from '../domain/endpoint.ts'
import { splitFences } from '../domain/format.ts'
import { clampFloat, clampInt } from '../domain/overrides.ts'
import { PROMPT_PRESETS, isUpstreamProvider } from '../domain/presets.ts'
import { buildChatRequest } from '../domain/request.ts'
import {
  browserSessionStore,
  loadProviderUpstream,
  loadSession,
  saveSession,
  type SessionStore,
} from '../domain/session.ts'
import { UPSTREAM_PROVIDERS, type ChatBreachDto, type ChatMessage, type SpaSession } from '../domain/types.ts'
import { postChat } from '../gateway/client.ts'
import { BreachCard } from './BreachCard.tsx'

const browserStore = browserSessionStore()

const fieldClass =
  'w-full border border-studio-line bg-studio-bg px-2 py-1 text-xs text-studio-text outline-none focus:border-studio-accent'

type Turn =
  | { id: string; kind: 'user'; text: string }
  | { id: string; kind: 'assistant'; text: string; streaming: boolean }
  | { id: string; kind: 'breach'; breach: ChatBreachDto }
  | { id: string; kind: 'error'; status: number; message: string }

export function Playground({
  fetchImpl,
  store = browserStore,
}: {
  fetchImpl?: typeof fetch
  store?: SessionStore
}) {
  const [session, setSession] = useState(() => loadSession(store))
  const [draft, setDraft] = useState('')
  const [turns, setTurns] = useState<Turn[]>([])
  const [busy, setBusy] = useState(false)
  const ids = useRef(0)
  const endpoint = resolveChatEndpoint(session.gatewayBase)

  function nextId(): string {
    ids.current += 1
    return `turn-${ids.current}`
  }

  function commit(recipe: (current: SpaSession) => SpaSession) {
    const next = recipe(session)
    saveSession(store, next)
    setSession(next)
  }

  function tune(patch: Partial<SpaSession['overrides']>) {
    commit((current) => ({
      ...current,
      overridesArmed: true,
      overrides: { ...current.overrides, ...patch },
    }))
  }

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    const prompt = draft.trim()
    if (!prompt || busy) return
    const userTurn: Turn = { id: nextId(), kind: 'user', text: prompt }
    const pendingId = nextId()
    const history = [...turns, userTurn]
    setTurns([...history, { id: pendingId, kind: 'assistant', text: '', streaming: true }])
    setDraft('')
    setBusy(true)
    const outcome = await postChat({
      gatewayBase: session.gatewayBase,
      request: buildChatRequest({
        messages: messagesFrom(history),
        domainId: session.domainId,
        overridesArmed: session.overridesArmed,
        overrides: session.overrides,
        upstream: session.upstream,
      }),
      fetchImpl,
      onDelta(piece) {
        setTurns((current) =>
          current.map((turn) =>
            turn.kind === 'assistant' && turn.id === pendingId
              ? { ...turn, text: turn.text + piece }
              : turn,
          ),
        )
      },
    })
    setTurns((current) =>
      current.map((turn) => {
        if (turn.id !== pendingId) return turn
        if (outcome.kind === 'pass') {
          return { id: pendingId, kind: 'assistant', text: outcome.text, streaming: false }
        }
        if (outcome.kind === 'breach') return { id: pendingId, kind: 'breach', breach: outcome.breach }
        return { id: pendingId, kind: 'error', status: outcome.status, message: outcome.message }
      }),
    )
    setBusy(false)
  }

  function onPromptKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key !== 'Enter' || event.shiftKey) return
    event.preventDefault()
    const form = event.currentTarget.form
    if (form && typeof form.requestSubmit === 'function') form.requestSubmit()
  }

  return (
    <div className="min-h-screen bg-studio-bg text-studio-text">
      <header className="flex flex-wrap items-baseline justify-between gap-3 border-b border-studio-line px-4 py-3">
        <h1 className="text-sm tracking-widest text-studio-accent">SF-PROXY PLAYGROUND</h1>
        <p className="text-[11px] text-studio-muted">
          {endpoint.ok ? `POST ${endpoint.value}` : endpoint.message}
        </p>
      </header>
      <div className="grid gap-4 p-4 lg:grid-cols-[minmax(0,1fr)_22rem]">
        <section aria-label="Chat" className="flex min-h-[70vh] flex-col border border-studio-line bg-studio-panel">
          <h2 className="border-b border-studio-line px-3 py-2 text-[11px] tracking-wide text-studio-muted">
            Chat
          </h2>
          <ol aria-label="Messages" className="flex flex-1 flex-col gap-3 overflow-auto px-3 py-3">
            {turns.map((turn) => (
              <li key={turn.id}>
                <TurnView turn={turn} />
              </li>
            ))}
          </ol>
          <div className="flex flex-wrap gap-2 border-t border-studio-line px-3 py-2">
            {PROMPT_PRESETS.map((preset) => (
              <button
                key={preset.id}
                type="button"
                className="border border-studio-line px-2 py-1 text-[11px] text-studio-muted hover:border-studio-accent hover:text-studio-text"
                onClick={() => setDraft(preset.text)}
              >
                {preset.label}
              </button>
            ))}
          </div>
          <form onSubmit={onSubmit} className="flex gap-2 border-t border-studio-line p-3">
            <textarea
              aria-label="Prompt"
              value={draft}
              rows={3}
              spellCheck={false}
              className={`${fieldClass} min-h-16 resize-y`}
              placeholder="Enter sends. Shift+Enter inserts a line."
              onChange={(event) => setDraft(event.target.value)}
              onKeyDown={onPromptKeyDown}
            />
            <button
              type="submit"
              disabled={busy || draft.trim() === ''}
              className="border border-sky-700 bg-sky-700 px-3 py-2 text-xs font-semibold text-white disabled:opacity-40"
            >
              {busy ? 'Sending' : 'Send'}
            </button>
          </form>
        </section>
        <aside aria-label="Live tuning" className="flex flex-col gap-4 border border-studio-line bg-studio-panel p-3">
          <h2 className="text-[11px] tracking-wide text-studio-muted">Live tuning</h2>
          <label className="flex flex-col gap-1 text-[11px] uppercase tracking-wide text-studio-muted">
            Gateway base
            <input
              className={fieldClass}
              value={session.gatewayBase}
              spellCheck={false}
              autoComplete="off"
              onChange={(event) =>
                commit((current) => ({ ...current, gatewayBase: event.target.value }))
              }
            />
          </label>
          <label className="flex flex-col gap-1 text-[11px] uppercase tracking-wide text-studio-muted">
            Domain id
            <input
              className={fieldClass}
              value={session.domainId}
              spellCheck={false}
              autoComplete="off"
              onChange={(event) =>
                commit((current) => ({ ...current, domainId: event.target.value }))
              }
            />
          </label>
          <label className="flex items-center gap-2 text-[11px] uppercase tracking-wide text-studio-muted">
            <input
              type="checkbox"
              checked={session.overridesArmed}
              onChange={(event) =>
                commit((current) => ({ ...current, overridesArmed: event.target.checked }))
              }
            />
            Apply overrides
          </label>
          <Slider
            label="L1 cosine threshold"
            min={0}
            max={1}
            step={0.01}
            digits={2}
            value={session.overrides.l1_threshold}
            onChange={(value) => tune({ l1_threshold: clampFloat(value, 0, 1, 2, 0.5) })}
          />
          <Slider
            label="L2 clause threshold"
            min={0}
            max={1}
            step={0.01}
            digits={2}
            value={session.overrides.l2_threshold}
            onChange={(value) => tune({ l2_threshold: clampFloat(value, 0, 1, 2, 0.5) })}
          />
          <Slider
            label="Quorum K"
            min={1}
            max={1015}
            step={1}
            digits={0}
            value={session.overrides.quorum_k}
            onChange={(value) => tune({ quorum_k: clampInt(value, 1, 1015, 103) })}
          />
          <Slider
            label="Epsilon"
            min={0}
            max={0.5}
            step={0.005}
            digits={3}
            value={session.overrides.epsilon}
            onChange={(value) => tune({ epsilon: clampFloat(value, 0, 0.5, 3, 0) })}
          />
          <h2 className="text-[11px] tracking-wide text-studio-muted">Upstream</h2>
          <div className="flex flex-col gap-1 text-[11px] uppercase tracking-wide text-studio-muted">
            <label htmlFor="upstream-provider">Provider</label>
            <select
              id="upstream-provider"
              className={fieldClass}
              value={session.upstream.provider}
              onChange={(event) => {
                const provider = event.target.value
                if (!isUpstreamProvider(provider)) return
                const upstream = loadProviderUpstream(provider, store)
                commit((current) => ({ ...current, upstream }))
              }}
            >
              {UPSTREAM_PROVIDERS.map((provider) => (
                <option key={provider} value={provider}>
                  {provider}
                </option>
              ))}
            </select>
          </div>
          {session.upstream.provider === 'mock' ? (
            <p className="text-[11px] text-studio-warn">
              mock ignores base URL and API key. The gateway returns its own SSE chunk.
            </p>
          ) : null}
          <label className="flex flex-col gap-1 text-[11px] uppercase tracking-wide text-studio-muted">
            Base URL
            <input
              className={fieldClass}
              value={session.upstream.baseUrl}
              spellCheck={false}
              autoComplete="off"
              onChange={(event) =>
                commit((current) => ({
                  ...current,
                  upstream: { ...current.upstream, baseUrl: event.target.value },
                }))
              }
            />
          </label>
          <label className="flex flex-col gap-1 text-[11px] uppercase tracking-wide text-studio-muted">
            Model
            <input
              className={fieldClass}
              value={session.upstream.model}
              spellCheck={false}
              autoComplete="off"
              onChange={(event) =>
                commit((current) => ({
                  ...current,
                  upstream: { ...current.upstream, model: event.target.value },
                }))
              }
            />
          </label>
          <label className="flex flex-col gap-1 text-[11px] uppercase tracking-wide text-studio-muted">
            API key
            <input
              type="password"
              className={fieldClass}
              value={session.upstream.apiKey}
              spellCheck={false}
              autoComplete="off"
              onChange={(event) =>
                commit((current) => ({
                  ...current,
                  upstream: { ...current.upstream, apiKey: event.target.value },
                }))
              }
            />
          </label>
        </aside>
      </div>
    </div>
  )
}

function Slider({
  label,
  min,
  max,
  step,
  digits,
  value,
  onChange,
}: {
  label: string
  min: number
  max: number
  step: number
  digits: number
  value: number
  onChange: (value: number) => void
}) {
  const shown = digits === 0 ? String(value) : value.toFixed(digits)
  const id = `tune-${label.toLowerCase().replace(/[^a-z0-9]+/g, '-')}`
  return (
    <div className="flex flex-col gap-1 text-[11px] uppercase tracking-wide text-studio-muted">
      <label htmlFor={id}>
        {label} <output htmlFor={id}>{shown}</output>
      </label>
      <input
        id={id}
        type="range"
        aria-label={label}
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(event) => onChange(Number(event.target.value))}
      />
    </div>
  )
}

function TurnView({ turn }: { turn: Turn }) {
  if (turn.kind === 'user') {
    return <p className="whitespace-pre-wrap border border-studio-line bg-studio-bg px-2 py-2">{turn.text}</p>
  }
  if (turn.kind === 'breach') return <BreachCard breach={turn.breach} />
  if (turn.kind === 'error') {
    return (
      <p className="border border-studio-breach px-2 py-2 text-studio-breach">
        HTTP {turn.status} {turn.message}
      </p>
    )
  }
  if (turn.streaming) {
    return <p className="whitespace-pre-wrap px-2 py-2">{turn.text}</p>
  }
  return (
    <div className="px-2 py-2">
      {splitFences(turn.text).map((part, index) =>
        part.kind === 'code' ? (
          <pre key={index} className="overflow-auto border border-studio-line bg-studio-bg p-2">
            <code>{part.text}</code>
          </pre>
        ) : (
          <p key={index} className="whitespace-pre-wrap">
            {part.text}
          </p>
        ),
      )}
    </div>
  )
}

function messagesFrom(history: readonly Turn[]): ChatMessage[] {
  const messages: ChatMessage[] = []
  for (const turn of history) {
    if (turn.kind === 'user') messages.push({ role: 'user', content: turn.text })
    if (turn.kind === 'assistant' && !turn.streaming && turn.text) {
      messages.push({ role: 'assistant', content: turn.text })
    }
  }
  return messages
}
