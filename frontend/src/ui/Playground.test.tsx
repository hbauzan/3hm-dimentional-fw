import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import { apiKeyStorageKey, memorySessionStore } from '../domain/session.ts'
import { Playground } from './Playground.tsx'

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

function delta(content: string): string {
  return `data: ${JSON.stringify({ choices: [{ delta: { content } }] })}\n\n`
}

interface Sent {
  url: string
  body: unknown
}

function recordingFetch(respond: (index: number) => Response | Promise<Response>): {
  fetchImpl: typeof fetch
  sent: Sent[]
} {
  const sent: Sent[] = []
  const fetchImpl: typeof fetch = async (input, init) => {
    sent.push({ url: String(input), body: JSON.parse(String(init?.body)) })
    return respond(sent.length)
  }
  return { fetchImpl, sent }
}

function fieldValue(element: HTMLElement): string {
  if (
    element instanceof HTMLInputElement ||
    element instanceof HTMLTextAreaElement ||
    element instanceof HTMLSelectElement
  ) {
    return element.value
  }
  throw new Error(`expected a form field, got ${element.tagName}`)
}

describe('Playground', () => {
  it('posts the live panel to the gateway and renders the SSE pass', async () => {
    const user = userEvent.setup()
    const store = memorySessionStore()
    const { fetchImpl, sent } = recordingFetch((index) =>
      index > 1
        ? sseResponse([delta('again')])
        : sseResponse([delta('He'), delta('llo'), 'data: [DONE]\n\n']),
    )
    render(<Playground fetchImpl={fetchImpl} store={store} />)

    expect(fieldValue(screen.getByRole('textbox', { name: 'Gateway base' }))).toBe(
      'http://127.0.0.1:8088',
    )
    expect(screen.getByText('POST http://127.0.0.1:8088/api/v1/chat')).toBeTruthy()

    await user.type(screen.getByRole('textbox', { name: 'Prompt' }), 'hola')
    await user.click(screen.getByRole('button', { name: 'Send' }))

    expect(await screen.findByText('Hello')).toBeTruthy()
    const body = sent[0]?.body as {
      overrides?: { l1_threshold: number }
      domain_id: string
      upstream: { provider: string }
    }
    expect(body.domain_id).toBe('python')
    expect(body.upstream.provider).toBe('mock')
    expect(body.overrides).toBeUndefined()
    expect(sent[0]?.url).toBe('http://127.0.0.1:8088/api/v1/chat')

    await user.type(screen.getByRole('textbox', { name: 'Prompt' }), 'de nuevo')
    await user.click(screen.getByRole('button', { name: 'Send' }))
    expect(await screen.findByText('again')).toBeTruthy()
    const second = sent[1]?.body as {
      messages: { role: string; content: string }[]
    }
    expect(second.messages).toEqual([
      { role: 'user', content: 'hola' },
      { role: 'assistant', content: 'Hello' },
      { role: 'user', content: 'de nuevo' },
    ])
  })

  it('renders the Rayos X card on HTTP 403 and keeps the blocked turn out of the assistant history', async () => {
    const user = userEvent.setup()
    const { fetchImpl, sent } = recordingFetch((index) =>
      index === 1
        ? jsonResponse(403, {
            decision: 'BREACH',
            failing_tier: 'L3_SPECTRAL_QUORUM',
            l1: { passed: true, cosine: 0.91, threshold: 0.5 },
            l2: { passed: true, threshold: 0.5 },
            l3: { evaluated: true, passed: false, affirmative_votes: 12, required_votes: 103 },
          })
        : sseResponse([delta('ok')]),
    )
    render(<Playground fetchImpl={fetchImpl} store={memorySessionStore()} />)

    const prompt = screen.getByRole('textbox', { name: 'Prompt' })
    await user.type(prompt, 'ruido')
    await user.click(screen.getByRole('button', { name: 'Send' }))
    expect(await screen.findByText('Affirmative votes 12 / K 103')).toBeTruthy()
    expect(screen.queryByText('Hello')).toBeNull()

    await user.type(prompt, 'despues')
    await user.click(screen.getByRole('button', { name: 'Send' }))
    await screen.findByText('ok')
    const second = sent[1]?.body as { messages: { role: string; content: string }[] }
    expect(second.messages).toEqual([
      { role: 'user', content: 'ruido' },
      { role: 'user', content: 'despues' },
    ])
  })

  it('stores the API key for the selected provider and sends that upstream', async () => {
    const user = userEvent.setup()
    const store = memorySessionStore()
    const { fetchImpl, sent } = recordingFetch(() => sseResponse([delta('pong')]))
    render(<Playground fetchImpl={fetchImpl} store={store} />)

    await user.selectOptions(screen.getByRole('combobox', { name: 'Provider' }), 'deepseek')
    expect(fieldValue(screen.getByRole('textbox', { name: 'Base URL' }))).toBe(
      'https://api.deepseek.com/v1',
    )
    await user.type(screen.getByLabelText('API key'), 'sk-test')
    await user.type(screen.getByRole('textbox', { name: 'Prompt' }), 'hola')
    await user.click(screen.getByRole('button', { name: 'Send' }))
    await screen.findByText('pong')

    const body = sent[0]?.body as {
      upstream: { provider: string; api_key?: string; base_url?: string; model?: string }
    }
    expect(body.upstream).toEqual({
      provider: 'deepseek',
      base_url: 'https://api.deepseek.com/v1',
      model: 'deepseek-chat',
      api_key: 'sk-test',
    })
    expect(sent[0]?.url ?? '').not.toContain('sk-test')
    expect(store.snapshot()[apiKeyStorageKey('deepseek')]).toBe('sk-test')

    await user.selectOptions(screen.getByRole('combobox', { name: 'Provider' }), 'openai')
    expect(fieldValue(screen.getByLabelText('API key'))).toBe('')
    await user.selectOptions(screen.getByRole('combobox', { name: 'Provider' }), 'deepseek')
    expect(fieldValue(screen.getByLabelText('API key'))).toBe('sk-test')
  })

  it('omits overrides when the live panel is disarmed and fills a corpus chip', async () => {
    const user = userEvent.setup()
    const { fetchImpl, sent } = recordingFetch(() => sseResponse([delta('streamed')]))
    render(<Playground fetchImpl={fetchImpl} store={memorySessionStore()} />)

    const overrides = screen.getByRole('checkbox', { name: 'Apply overrides' })
    if (!(overrides instanceof HTMLInputElement)) throw new Error('overrides control is not a checkbox')
    expect(overrides.checked).toBe(false)
    await user.click(screen.getByRole('button', { name: 'Risotto L1' }))
    expect(fieldValue(screen.getByRole('textbox', { name: 'Prompt' }))).toBe(
      'Dame la receta para cocinar un risotto de hongos',
    )
    expect(sent).toHaveLength(0)
    await user.click(screen.getByRole('button', { name: 'Send' }))
    await screen.findByText('streamed')
    const body = sent[0]?.body as {
      overrides?: unknown
      messages: { content: string }[]
    }
    expect(body.overrides).toBeUndefined()
    expect(body.messages[0]?.content).toBe('Dame la receta para cocinar un risotto de hongos')
  })

  it('shows a gateway error without painting a breach card', async () => {
    const user = userEvent.setup()
    render(
      <Playground
        fetchImpl={async () => {
          throw new TypeError('Failed to fetch')
        }}
        store={memorySessionStore()}
      />,
    )
    await user.type(screen.getByRole('textbox', { name: 'Prompt' }), 'hola')
    await user.click(screen.getByRole('button', { name: 'Send' }))
    expect(await screen.findByText(/HTTP 0 Failed to fetch/)).toBeTruthy()
    expect(screen.queryByText('[SEMANTIC CONTAINMENT BREACH]')).toBeNull()
  })

  it('omits epsilon until that perilla moves, then posts the pinned value', async () => {
    const user = userEvent.setup()
    const { fetchImpl, sent } = recordingFetch(() => sseResponse([delta('x')]))
    render(<Playground fetchImpl={fetchImpl} store={memorySessionStore()} />)
    fireEvent.change(screen.getByRole('slider', { name: 'L2 clause threshold' }), {
      target: { value: '0.8' },
    })
    await user.type(screen.getByRole('textbox', { name: 'Prompt' }), 'hola')
    await user.click(screen.getByRole('button', { name: 'Send' }))
    await screen.findByText('x')
    const tuned = sent[0]?.body as {
      overrides: { l2_threshold: number; epsilon?: number }
    }
    expect(tuned.overrides.l2_threshold).toBe(0.8)
    expect(tuned.overrides.epsilon).toBeUndefined()

    fireEvent.change(screen.getByRole('slider', { name: 'Epsilon' }), {
      target: { value: '0.05' },
    })
    await user.type(screen.getByRole('textbox', { name: 'Prompt' }), 'otra')
    await user.click(screen.getByRole('button', { name: 'Send' }))
    await screen.findAllByText('x')
    const pinned = sent[1]?.body as {
      overrides: { epsilon?: number; l2_threshold: number }
    }
    expect(pinned.overrides.l2_threshold).toBe(0.8)
    expect(pinned.overrides.epsilon).toBe(0.05)
  })

  it('moves the L1 slider into the next request body', async () => {
    const user = userEvent.setup()
    const { fetchImpl, sent } = recordingFetch(() => sseResponse([delta('x')]))
    render(<Playground fetchImpl={fetchImpl} store={memorySessionStore()} />)
    fireEvent.change(screen.getByRole('slider', { name: /L1 cosine threshold/ }), {
      target: { value: '1' },
    })
    await user.type(screen.getByRole('textbox', { name: 'Prompt' }), 'hola')
    await user.click(screen.getByRole('button', { name: 'Send' }))
    await screen.findByText('x')
    const body = sent[0]?.body as {
      overrides: { l1_threshold: number }
    }
    expect(body.overrides.l1_threshold).toBe(1)
  })

  it('loads .env keys when requested and sends use_env on chat requests', async () => {
    const user = userEvent.setup()
    const store = memorySessionStore()
    const catalogResponse = {
      use_env_available: true,
      providers: {
        gemini: {
          provider: 'gemini',
          configured: true,
          api_key: 'AQ.from_env_gemini',
          base_url: 'https://generativelanguage.googleapis.com/v1beta/openai',
          model: 'gemini-3.8-flash',
          env_key_found: 'GEMINI_API_KEY',
        },
      },
    }

    const sentRequests: { url: string; body: unknown }[] = []
    const fetchImpl: typeof fetch = async (input, init) => {
      const url = String(input)
      if (url.endsWith('/api/v1/upstream/env')) {
        return jsonResponse(200, catalogResponse)
      }
      sentRequests.push({ url, body: JSON.parse(String(init?.body)) })
      return sseResponse([delta('hola desde gemini'), 'data: [DONE]\n\n'])
    }

    render(<Playground fetchImpl={fetchImpl} store={store} />)

    await user.selectOptions(screen.getByRole('combobox', { name: 'Provider' }), 'gemini')
    expect(fieldValue(screen.getByRole('textbox', { name: 'Model' }))).toBe('gemini-3.8-flash')

    await user.click(screen.getByRole('button', { name: 'Cargar de .env' }))
    expect(await screen.findByText(/✓ Clave cargada de .env \(GEMINI_API_KEY\)/)).toBeTruthy()
    expect(fieldValue(screen.getByLabelText('API key'))).toBe('AQ.from_env_gemini')

    await user.type(screen.getByRole('textbox', { name: 'Prompt' }), 'hola')
    await user.click(screen.getByRole('button', { name: 'Send' }))
    expect(await screen.findByText('hola desde gemini')).toBeTruthy()

    const body = sentRequests[0]?.body as {
      upstream: { provider: string; api_key?: string; model?: string; use_env?: boolean }
    }
    expect(body.upstream.provider).toBe('gemini')
    expect(body.upstream.api_key).toBe('AQ.from_env_gemini')
    expect(body.upstream.model).toBe('gemini-3.8-flash')
    expect(body.upstream.use_env).toBe(true)
  })

  it('sends the selected canonical domain on POST /api/v1/chat', async () => {
    const user = userEvent.setup()
    const { fetchImpl, sent } = recordingFetch(() => sseResponse([delta('ok')]))
    render(<Playground fetchImpl={fetchImpl} store={memorySessionStore()} />)

    await user.selectOptions(screen.getByRole('combobox', { name: 'Domain id' }), 'legal')
    await user.type(screen.getByRole('textbox', { name: 'Prompt' }), 'hola')
    await user.click(screen.getByRole('button', { name: 'Send' }))
    await screen.findByText('ok')

    const body = sent[0]?.body as { domain_id: string }
    expect(body.domain_id).toBe('legal')
    expect(sent[0]?.url).toBe('http://127.0.0.1:8088/api/v1/chat')
  })

  it('keeps a stored custom domain as an extra option', () => {
    render(
      <Playground
        fetchImpl={async () => sseResponse([delta('x')])}
        store={memorySessionStore({ 'sf.spa.domainId': 'python_receta' })}
      />,
    )

    const domain = screen.getByRole('combobox', { name: 'Domain id' })
    expect(fieldValue(domain)).toBe('python_receta')
    expect(screen.getByRole('option', { name: 'python_receta (custom)' })).toBeTruthy()
  })
})

