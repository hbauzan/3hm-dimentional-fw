import { describe, expect, it, vi } from 'vitest'
import { postChat } from './client.ts'
import type { ChatRequest } from '../domain/types.ts'

const request: ChatRequest = {
  messages: [{ role: 'user', content: 'hola' }],
  domain_id: 'python',
  upstream: { provider: 'mock', model: 'mock' },
}

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

function sseResponse(chunks: readonly string[]): Response {
  const encoder = new TextEncoder()
  return new Response(
    new ReadableStream<Uint8Array>({
      start(controller) {
        for (const chunk of chunks) controller.enqueue(encoder.encode(chunk))
        controller.close()
      },
    }),
    { status: 200, headers: { 'content-type': 'text/event-stream' } },
  )
}

describe('postChat', () => {
  it('streams SSE deltas for a PASS and concatenates them', async () => {
    const seen: string[] = []
    const sent: { url: string; init: RequestInit | undefined }[] = []
    const fetchImpl = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      sent.push({ url: String(input), init })
      return sseResponse([
        'data: {"choices":[{"delta":{"content":"He"}}]}\n\n',
        'data: {"choices":[{"delta":{"content":"llo"}}]}\n\ndata: [DONE]\n\n',
      ])
    })
    const outcome = await postChat({
      gatewayBase: 'http://127.0.0.1:8088',
      request,
      fetchImpl,
      onDelta: (piece) => {
        seen.push(piece)
      },
    })
    expect(outcome).toEqual({ kind: 'pass', text: 'Hello' })
    expect(seen).toEqual(['He', 'llo'])
    expect(sent).toHaveLength(1)
    const sentCall = sent[0]
    if (!sentCall) throw new Error('missing request')
    expect(sentCall.url).toBe('http://127.0.0.1:8088/api/v1/chat')
    expect(new Headers(sentCall.init?.headers).get('content-type')).toBe('application/json')
    expect(JSON.parse(String(sentCall.init?.body))).toEqual(request)
  })

  it('returns a breach card payload on HTTP 403 ChatBreachDto', async () => {
    const breach = {
      decision: 'BREACH',
      failing_tier: 'L2_CLAUSE_GATE',
      l1: { passed: true, cosine: 0.78, threshold: 0.65 },
      l2: {
        passed: false,
        breached_clause_index: 1,
        breached_clause_text: 'salsa de tomate',
        clause_cosine: 0.42,
        threshold: 0.65,
      },
      l3: { evaluated: false },
    }
    const outcome = await postChat({
      gatewayBase: 'http://127.0.0.1:8088/',
      request,
      fetchImpl: async () => jsonResponse(403, breach),
    })
    expect(outcome).toEqual({ kind: 'breach', breach })
  })

  it('keeps a fail-closed 403 out of the breach card', async () => {
    const outcome = await postChat({
      gatewayBase: 'http://127.0.0.1:8088',
      request,
      fetchImpl: async () =>
        jsonResponse(403, { error: 'Fail-closed: embedding subsystem error: boom' }),
    })
    expect(outcome).toEqual({
      kind: 'error',
      status: 403,
      message: 'Fail-closed: embedding subsystem error: boom',
    })
  })

  it('reports an unknown domain without calling the body a breach', async () => {
    const outcome = await postChat({
      gatewayBase: 'http://127.0.0.1:8088',
      request,
      fetchImpl: async () => jsonResponse(404, { error: "unknown domain_id 'nope'" }),
    })
    expect(outcome).toEqual({
      kind: 'error',
      status: 404,
      message: "unknown domain_id 'nope'",
    })
  })

  it('does not call fetch when the gateway base or the user text is unusable', async () => {
    const fetchImpl = vi.fn()
    const badBase = await postChat({
      gatewayBase: 'not a url',
      request,
      fetchImpl,
    })
    const blank = await postChat({
      gatewayBase: 'http://127.0.0.1:8088',
      request: { ...request, messages: [{ role: 'user', content: '   ' }] },
      fetchImpl,
    })
    expect(badBase.kind).toBe('error')
    expect(blank.kind).toBe('error')
    expect(fetchImpl).not.toHaveBeenCalled()
  })

  it('maps a network failure to status 0', async () => {
    const outcome = await postChat({
      gatewayBase: 'http://127.0.0.1:8088',
      request,
      fetchImpl: async () => {
        throw new TypeError('Failed to fetch')
      },
    })
    expect(outcome).toEqual({ kind: 'error', status: 0, message: 'Failed to fetch' })
  })
})
