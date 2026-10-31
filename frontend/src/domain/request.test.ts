import { describe, expect, it } from 'vitest'
import { buildChatRequest } from './request.ts'
import type { ChatDraft } from './types.ts'

function draft(patch: Partial<ChatDraft> = {}): ChatDraft {
  return {
    messages: [{ role: 'user', content: 'hola' }],
    domainId: 'python',
    overridesArmed: true,
    overrides: {
      l1_threshold: 0.5,
      l2_threshold: 0.5,
      quorum_k: 103,
      epsilon: 0,
    },
    upstream: {
      provider: 'mock',
      baseUrl: '',
      model: 'mock',
      apiKey: '',
    },
    ...patch,
  }
}

describe('buildChatRequest', () => {
  it('sends live overrides and omits blank upstream fields', () => {
    expect(buildChatRequest(draft())).toEqual({
      messages: [{ role: 'user', content: 'hola' }],
      domain_id: 'python',
      overrides: {
        l1_threshold: 0.5,
        l2_threshold: 0.5,
        quorum_k: 103,
        epsilon: 0,
      },
      upstream: { provider: 'mock', model: 'mock' },
    })
  })

  it('omits overrides and domain_id when the operator disarms them', () => {
    const body = buildChatRequest(draft({ overridesArmed: false, domainId: '  ' }))
    expect(body.overrides).toBeUndefined()
    expect(body.domain_id).toBeUndefined()
  })

  it('clamps override numbers onto the studio ranges', () => {
    const body = buildChatRequest(
      draft({
        overrides: {
          l1_threshold: 1.4,
          l2_threshold: Number.NaN,
          quorum_k: 0,
          epsilon: -1,
        },
      }),
    )
    expect(body.overrides).toEqual({
      l1_threshold: 1,
      l2_threshold: 0.5,
      quorum_k: 1,
      epsilon: 0,
    })
  })

  it('includes a trimmed api key and base URL for an http provider', () => {
    const body = buildChatRequest(
      draft({
        upstream: {
          provider: 'deepseek',
          baseUrl: ' https://api.deepseek.com/v1 ',
          model: ' deepseek-chat ',
          apiKey: ' sk-test ',
        },
      }),
    )
    expect(body.upstream).toEqual({
      provider: 'deepseek',
      base_url: 'https://api.deepseek.com/v1',
      model: 'deepseek-chat',
      api_key: 'sk-test',
    })
  })
})
