import { describe, expect, it } from 'vitest'
import { resolveChatEndpoint } from './endpoint.ts'

describe('resolveChatEndpoint', () => {
  it('joins the gateway origin to POST /api/v1/chat', () => {
    const resolved = resolveChatEndpoint('http://127.0.0.1:8088')
    expect(resolved).toEqual({
      ok: true,
      value: 'http://127.0.0.1:8088/api/v1/chat',
    })
  })

  it('drops a trailing slash and a pasted chat path', () => {
    const slashed = resolveChatEndpoint('http://127.0.0.1:8088/')
    const pasted = resolveChatEndpoint('http://127.0.0.1:8088/api/v1/chat')
    expect(slashed).toEqual({ ok: true, value: 'http://127.0.0.1:8088/api/v1/chat' })
    expect(pasted).toEqual({ ok: true, value: 'http://127.0.0.1:8088/api/v1/chat' })
  })

  it('rejects a non-http base before any request is sent', () => {
    expect(resolveChatEndpoint('ftp://127.0.0.1:8088').ok).toBe(false)
    expect(resolveChatEndpoint('/api/v1/chat').ok).toBe(false)
    expect(resolveChatEndpoint('').ok).toBe(false)
  })
})
