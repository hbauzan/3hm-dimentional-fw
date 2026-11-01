import { useEffect, useRef, useState, type FormEvent, type KeyboardEvent } from 'react'
import { resolveChatEndpoint } from '../domain/endpoint.ts'
import { splitFences } from '../domain/format.ts'
import { clampFloat, clampInt } from '../domain/overrides.ts'
import {
  CANONICAL_DOMAINS,
  PROMPT_PRESETS,
  isCanonicalDomain,
  isUpstreamProvider,
} from '../domain/presets.ts'
import { buildChatRequest } from '../domain/request.ts'
import {
  browserSessionStore,
  loadProviderUpstream,
  loadSession,
  saveSession,
  type SessionStore,
} from '../domain/session.ts'
import {
  UPSTREAM_PROVIDERS,
  type ChatBreachDto,
  type ChatTelemetryDto,
  type ChatMessage,
  type SpaSession,
  type UpstreamProvider,
} from '../domain/types.ts'
import { fetchUpstreamEnvCatalog, postChat } from '../gateway/client.ts'
import { BreachCard, PassCard } from './BreachCard.tsx'

const browserStore = browserSessionStore()

const fieldClass =
  'w-full border border-studio-line bg-studio-bg px-1.5 py-0.5 text-xs text-studio-text outline-none focus:border-studio-accent'

type Turn =
  | { id: string; kind: 'user'; text: string }
  | { id: string; kind: 'assistant'; text: string; streaming: boolean; telemetry?: ChatTelemetryDto }
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
  const [envLoading, setEnvLoading] = useState(false)
  const [envMessage, setEnvMessage] = useState<string | null>(null)
  const ids = useRef(0)
  const messagesContainerRef = useRef<HTMLOListElement>(null)
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const isAtBottomRef = useRef(true)
  const endpoint = resolveChatEndpoint(session.gatewayBase)

  function handleScroll() {
    const el = messagesContainerRef.current
    if (!el) return
    const threshold = 40
    isAtBottomRef.current = el.scrollHeight - el.scrollTop - el.clientHeight <= threshold
  }

  useEffect(() => {
    if (isAtBottomRef.current) {
      messagesEndRef.current?.scrollIntoView?.({ behavior: 'smooth' })
    }
  }, [turns])

  async function syncEnv(targetProvider: UpstreamProvider = session.upstream.provider) {
    setEnvLoading(true)
    setEnvMessage(null)
    try {
      const catalog = await fetchUpstreamEnvCatalog(session.gatewayBase, fetchImpl)
      if (catalog?.providers && catalog.providers[targetProvider]) {
        const info = catalog.providers[targetProvider]
        if (info.configured && info.api_key) {
          commit((current) => ({
            ...current,
            upstream: {
              ...current.upstream,
              useEnv: true,
              apiKey: info.api_key,
              baseUrl: info.base_url || current.upstream.baseUrl,
              model: info.model || current.upstream.model,
            },
          }))
          setEnvMessage(`✓ Clave cargada de .env (${info.env_key_found ?? 'detectada'})`)
        } else if (info.configured && targetProvider === 'mock') {
          commit((current) => ({
            ...current,
            upstream: { ...current.upstream, useEnv: true },
          }))
          setEnvMessage('✓ Mock no requiere API key')
        } else {
          setEnvMessage(`⚠️ Sin clave en .env para ${targetProvider}`)
        }
      } else {
        setEnvMessage('⚠️ Gateway no devolvió configuración de .env')
      }
    } catch {
      setEnvMessage('⚠️ Error conectando al gateway')
    } finally {
      setEnvLoading(false)
    }
  }

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
      onTelemetry(telemetry) {
        setTurns((current) =>
          current.map((turn) =>
            turn.kind === 'assistant' && turn.id === pendingId
              ? { ...turn, telemetry }
              : turn,
          ),
        )
      },
    })
    setTurns((current) =>
      current.map((turn) => {
        if (turn.id !== pendingId) return turn
        if (outcome.kind === 'pass') {
          return {
            id: pendingId,
            kind: 'assistant',
            text: outcome.text,
            streaming: false,
            telemetry: outcome.telemetry ?? (turn.kind === 'assistant' ? turn.telemetry : undefined),
          }
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
    <div className="flex h-[100dvh] flex-col overflow-hidden bg-studio-bg text-studio-text">
      <header className="flex flex-shrink-0 items-center justify-between border-b border-studio-line px-4 py-2.5">
        <h1 className="text-xs font-semibold tracking-widest text-studio-accent uppercase">
          SF-PROXY PLAYGROUND
        </h1>
        <p className="text-[11px] font-mono text-studio-muted">
          {endpoint.ok ? `POST ${endpoint.value}` : endpoint.message}
        </p>
      </header>
      <main className="flex flex-1 overflow-hidden">
        <section aria-label="Chat" className="flex flex-1 flex-col overflow-hidden min-w-0">
          <h2 className="sr-only">Chat</h2>
          <ol
            ref={messagesContainerRef}
            onScroll={handleScroll}
            aria-label="Messages"
            className="flex flex-1 flex-col gap-3 overflow-y-auto overscroll-contain px-4 py-4"
          >
            {turns.map((turn) => (
              <li key={turn.id}>
                <TurnView turn={turn} />
              </li>
            ))}
            <div ref={messagesEndRef} />
          </ol>
          <div className="flex flex-shrink-0 flex-wrap gap-1.5 border-t border-studio-line bg-studio-panel/40 px-3 py-1.5">
            {PROMPT_PRESETS.map((preset) => (
              <button
                key={preset.id}
                type="button"
                className="border border-studio-line px-2 py-0.5 text-[11px] text-studio-muted hover:border-studio-accent hover:text-studio-text transition-colors"
                onClick={() => setDraft(preset.text)}
              >
                {preset.label}
              </button>
            ))}
          </div>
          <form onSubmit={onSubmit} className="flex flex-shrink-0 gap-2 border-t border-studio-line bg-studio-panel p-3">
            <textarea
              aria-label="Prompt"
              value={draft}
              rows={2}
              spellCheck={false}
              className="w-full min-h-14 resize-none border border-studio-line bg-studio-bg px-2.5 py-1.5 text-xs text-studio-text outline-none focus:border-studio-accent"
              placeholder="Enter sends. Shift+Enter inserts a line."
              onChange={(event) => setDraft(event.target.value)}
              onKeyDown={onPromptKeyDown}
            />
            <button
              type="submit"
              disabled={busy || draft.trim() === ''}
              className="border border-sky-700 bg-sky-700 px-4 py-2 text-xs font-semibold text-white disabled:opacity-40 hover:bg-sky-600 transition-colors cursor-pointer disabled:cursor-not-allowed"
            >
              {busy ? 'Sending' : 'Send'}
            </button>
          </form>
        </section>
        <aside
          aria-label="Live tuning"
          className="flex w-60 flex-shrink-0 flex-col gap-2.5 overflow-y-auto border-l border-studio-line bg-studio-panel p-3"
        >
          <h2 className="text-[10px] font-semibold tracking-wider uppercase text-studio-muted">Live tuning</h2>
          <label className="flex flex-col gap-1 text-[10px] uppercase tracking-wide text-studio-muted">
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
          <label className="flex flex-col gap-1 text-[10px] uppercase tracking-wide text-studio-muted">
            Domain id
            <select
              aria-label="Domain id"
              className={fieldClass}
              value={session.domainId}
              onChange={(event) =>
                commit((current) => ({ ...current, domainId: event.target.value }))
              }
            >
              {CANONICAL_DOMAINS.map((domain) => (
                <option key={domain} value={domain}>
                  {domain}
                </option>
              ))}
              {!isCanonicalDomain(session.domainId) && session.domainId ? (
                <option value={session.domainId}>{session.domainId} (custom)</option>
              ) : null}
            </select>
          </label>
          <label className="flex items-center gap-2 cursor-pointer text-[10px] uppercase tracking-wide text-studio-muted hover:text-studio-text">
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
          <h2 className="mt-1 text-[10px] font-semibold tracking-wider uppercase text-studio-muted">Upstream</h2>
          <div className="flex flex-col gap-2 border border-studio-line bg-studio-panel/50 p-2">
            <div className="flex items-center justify-between">
              <label className="flex items-center gap-1.5 cursor-pointer text-[10px] uppercase tracking-wide text-studio-muted hover:text-studio-text">
                <input
                  type="checkbox"
                  aria-label="Usar keys de .env"
                  checked={session.upstream.useEnv}
                  onChange={(event) => {
                    const active = event.target.checked
                    commit((current) => ({
                      ...current,
                      upstream: { ...current.upstream, useEnv: active },
                    }))
                    if (active) {
                      void syncEnv(session.upstream.provider)
                    } else {
                      setEnvMessage(null)
                    }
                  }}
                />
                <span>Usar keys de .env</span>
              </label>
              <button
                type="button"
                disabled={envLoading}
                aria-label="Cargar de .env"
                className="border border-studio-line px-1.5 py-0.5 text-[10px] text-studio-muted hover:border-studio-accent hover:text-studio-text disabled:opacity-50"
                onClick={() => void syncEnv(session.upstream.provider)}
              >
                {envLoading ? 'Leyendo…' : '🔄 Cargar de .env'}
              </button>
            </div>
            {envMessage ? (
              <p
                role="status"
                className={`text-[10px] font-mono break-words ${
                  envMessage.startsWith('✓') ? 'text-emerald-400' : 'text-amber-400'
                }`}
              >
                {envMessage}
              </p>
            ) : null}
          </div>
          <div className="flex flex-col gap-1 text-[10px] uppercase tracking-wide text-studio-muted">
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
                if (session.upstream.useEnv) {
                  void syncEnv(provider)
                }
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
            <p className="text-[10px] text-studio-warn leading-tight">
              mock ignores base URL and API key. The gateway returns its own SSE chunk.
            </p>
          ) : null}
          <label className="flex flex-col gap-1 text-[10px] uppercase tracking-wide text-studio-muted">
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
          <label className="flex flex-col gap-1 text-[10px] uppercase tracking-wide text-studio-muted">
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
          <label className="flex flex-col gap-1 text-[10px] uppercase tracking-wide text-studio-muted">
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
      </main>
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
    <div className="flex flex-col gap-0.5 text-[10px] uppercase tracking-wide text-studio-muted">
      <label htmlFor={id} className="flex items-center justify-between cursor-pointer">
        <span>{label}</span>
        <output htmlFor={id} className="font-mono text-[10px] text-studio-accent">
          {shown}
        </output>
      </label>
      <input
        id={id}
        type="range"
        aria-label={label}
        min={min}
        max={max}
        step={step}
        value={value}
        className="h-1.5 w-full cursor-pointer accent-studio-accent"
        onChange={(event) => onChange(Number(event.target.value))}
      />
    </div>
  )
}

function TurnView({ turn }: { turn: Turn }) {
  if (turn.kind === 'user') {
    return (
      <div className="flex justify-end">
        <p className="max-w-[85%] whitespace-pre-wrap border border-studio-line bg-studio-card px-3 py-2 text-xs text-studio-text">
          {turn.text}
        </p>
      </div>
    )
  }
  if (turn.kind === 'breach') return <BreachCard breach={turn.breach} />
  if (turn.kind === 'error') {
    return (
      <p className="border border-studio-breach bg-studio-breach/10 px-3 py-2 text-xs font-mono text-studio-breach">
        HTTP {turn.status} {turn.message}
      </p>
    )
  }
  if (turn.streaming) {
    return (
      <div className="px-2 py-2 text-xs">
        {turn.telemetry ? <PassCard telemetry={turn.telemetry} /> : null}
        <p className="whitespace-pre-wrap">{turn.text}</p>
      </div>
    )
  }
  return (
    <div className="px-2 py-2 text-xs">
      {turn.telemetry ? <PassCard telemetry={turn.telemetry} /> : null}
      {splitFences(turn.text).map((part, index) =>
        part.kind === 'code' ? (
          <pre key={index} className="overflow-auto border border-studio-line bg-studio-bg p-2 text-xs">
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
