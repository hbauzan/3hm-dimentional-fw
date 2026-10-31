import { describe, expect, it } from 'vitest'
import { PROVIDER_DEFAULTS } from './presets.ts'
import {
  DEFAULT_OVERRIDES_ARMED,
  apiKeyStorageKey,
  loadProviderUpstream,
  loadSession,
  memorySessionStore,
  saveSession,
} from './session.ts'
import type { SpaSession } from './types.ts'

function session(patch: Partial<SpaSession> = {}): SpaSession {
  return {
    gatewayBase: 'http://127.0.0.1:8088',
    domainId: 'python',
    overridesArmed: true,
    overrides: {
      l1_threshold: 0.42,
      l2_threshold: 0.4,
      quorum_k: 80,
      epsilon: 0.05,
    },
    upstream: {
      provider: 'openai',
      baseUrl: 'https://api.openai.com/v1',
      model: 'gpt-4o-mini',
      apiKey: 'sk-live',
    },
    ...patch,
  }
}

describe('session store', () => {
  it('keeps each provider API key in its own localStorage slot', () => {
    const store = memorySessionStore()
    saveSession(store, session())
    saveSession(
      store,
      session({
        upstream: {
          provider: 'deepseek',
          baseUrl: 'https://api.deepseek.com/v1',
          model: 'deepseek-chat',
          apiKey: 'sk-deep',
        },
      }),
    )

    expect(store.snapshot()[apiKeyStorageKey('openai')]).toBe('sk-live')
    expect(store.snapshot()[apiKeyStorageKey('deepseek')]).toBe('sk-deep')
    const nonSecret = Object.entries(store.snapshot()).filter(([key]) => !key.includes('apiKey'))
    expect(nonSecret.some(([, value]) => value.includes('sk-'))).toBe(false)

    expect(loadProviderUpstream('openai', store).apiKey).toBe('sk-live')
    expect(loadProviderUpstream('gemini', store)).toMatchObject({
      provider: 'gemini',
      baseUrl: PROVIDER_DEFAULTS.gemini.baseUrl,
      model: PROVIDER_DEFAULTS.gemini.model,
      apiKey: '',
    })
  })

  it('restores tuning and falls back when a stored number is corrupt', () => {
    const store = memorySessionStore({
      'sf.spa.gatewayBase': 'http://10.0.0.8:8088',
      'sf.spa.domainId': 'python_receta',
      'sf.spa.overridesArmed': '0',
      'sf.spa.l1': 'nope',
      'sf.spa.provider': 'gemini',
    })
    const loaded = loadSession(store)
    expect(loaded.gatewayBase).toBe('http://10.0.0.8:8088')
    expect(loaded.domainId).toBe('python_receta')
    expect(loaded.overridesArmed).toBe(false)
    expect(loaded.overrides.l1_threshold).toBe(0.5)
    expect(loaded.overrides.quorum_k).toBe(103)
    expect(loaded.upstream.provider).toBe('gemini')
    expect(loaded.upstream.model).toBe(PROVIDER_DEFAULTS.gemini.model)
  })

  it('defaults an empty store to the local gateway without overriding the lock', () => {
    const loaded = loadSession(memorySessionStore())
    expect(loaded.gatewayBase).toBe('http://127.0.0.1:8088')
    expect(loaded.domainId).toBe('python')
    expect(loaded.overridesArmed).toBe(DEFAULT_OVERRIDES_ARMED)
    expect(loaded.upstream.provider).toBe('mock')
    expect(loaded.upstream.apiKey).toBe('')
  })

  it('drops a cleared API key instead of keeping the previous secret', () => {
    const store = memorySessionStore()
    saveSession(store, session())
    saveSession(
      store,
      session({
        upstream: { ...session().upstream, apiKey: '' },
      }),
    )
    expect(store.snapshot()[apiKeyStorageKey('openai')]).toBeUndefined()
    expect(loadProviderUpstream('openai', store).apiKey).toBe('')
  })
})
