import { parseChatBreach } from '../domain/breach.ts'
import { resolveChatEndpoint } from '../domain/endpoint.ts'
import { isRecord } from '../domain/json.ts'
import { hasUserText } from '../domain/request.ts'
import { createSseDecoder } from '../domain/sse.ts'
import type { ChatOutcome, ChatRequest } from '../domain/types.ts'

export interface PostChatArgs {
  gatewayBase: string
  request: ChatRequest
  onDelta?: (piece: string) => void
  fetchImpl?: typeof fetch
  signal?: AbortSignal
}

export async function postChat(args: PostChatArgs): Promise<ChatOutcome> {
  if (!hasUserText(args.request.messages)) {
    return { kind: 'error', status: 0, message: 'missing user message' }
  }
  const endpoint = resolveChatEndpoint(args.gatewayBase)
  if (!endpoint.ok) return { kind: 'error', status: 0, message: endpoint.message }

  const fetchImpl = args.fetchImpl ?? globalThis.fetch
  let response: Response
  try {
    response = await fetchImpl(endpoint.value, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(args.request),
      signal: args.signal,
    })
  } catch (error) {
    const message = error instanceof Error ? error.message : 'gateway request failed'
    return { kind: 'error', status: 0, message }
  }

  if (response.status === 200) {
    const contentType = response.headers.get('content-type') ?? ''
    if (!contentType.toLowerCase().startsWith('text/event-stream')) {
      const body = await response.text()
      return { kind: 'error', status: 200, message: body || 'expected text/event-stream' }
    }
    return readSse(response, args.onDelta)
  }

  const payload = await readUnknown(response)
  if (response.status === 403) {
    const parsed = parseChatBreach(payload)
    if (parsed.ok) return { kind: 'breach', breach: parsed.value }
  }
  return { kind: 'error', status: response.status, message: explain(payload) }
}

async function readSse(
  response: Response,
  onDelta: ((piece: string) => void) | undefined,
): Promise<ChatOutcome> {
  if (!response.body) return { kind: 'error', status: 200, message: 'empty stream' }
  const reader = response.body.getReader()
  const decoder = new TextDecoder()
  const sse = createSseDecoder()
  let text = ''
  const emit = (pieces: readonly string[]) => {
    for (const piece of pieces) {
      text += piece
      onDelta?.(piece)
    }
  }
  while (true) {
    const step = await reader.read()
    if (step.done) break
    emit(sse.push(decoder.decode(step.value, { stream: true })))
  }
  emit(sse.push(decoder.decode()))
  emit(sse.flush())
  return { kind: 'pass', text }
}

async function readUnknown(response: Response): Promise<unknown> {
  const text = await response.text()
  if (!text) return null
  try {
    return JSON.parse(text) as unknown
  } catch {
    return text
  }
}

function explain(payload: unknown): string {
  if (typeof payload === 'string' && payload.trim()) return payload.trim()
  if (isRecord(payload)) {
    const error = payload.error
    if (typeof error === 'string' && error.trim()) return error.trim()
    if (isRecord(error) && typeof error.message === 'string' && error.message.trim()) {
      return error.message.trim()
    }
  }
  if (payload === null) return 'empty error body'
  return JSON.stringify(payload)
}
