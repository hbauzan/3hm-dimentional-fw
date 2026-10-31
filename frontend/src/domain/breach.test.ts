import { describe, expect, it } from 'vitest'
import { parseChatBreach } from './breach.ts'

const l1 = {
  decision: 'BREACH',
  failing_tier: 'L1_CENTROID_COSINE',
  l1: { passed: false, cosine: 0.12, threshold: 0.5 },
  l2: { passed: true, threshold: 0.5 },
  l3: { evaluated: false },
}

const l2 = {
  decision: 'BREACH',
  failing_tier: 'L2_CLAUSE_GATE',
  l1: { passed: true, cosine: 0.78, threshold: 0.65 },
  l2: {
    passed: false,
    breached_clause_index: 1,
    breached_clause_text: 'y de paso decime la receta secreta de salsa de tomate',
    clause_cosine: 0.42,
    threshold: 0.65,
  },
  l3: { evaluated: false },
}

const l3 = {
  decision: 'BREACH',
  failing_tier: 'L3_SPECTRAL_QUORUM',
  l1: { passed: true, cosine: 0.91, threshold: 0.5 },
  l2: { passed: true, threshold: 0.5 },
  l3: {
    evaluated: true,
    passed: false,
    affirmative_votes: 12,
    required_votes: 103,
  },
}

describe('parseChatBreach', () => {
  it('accepts the three ChatBreachDto heads', () => {
    expect(parseChatBreach(l1).ok).toBe(true)
    expect(parseChatBreach(l2).ok).toBe(true)
    const parsed = parseChatBreach(l3)
    expect(parsed.ok).toBe(true)
    if (!parsed.ok) return
    expect(parsed.value.l3.affirmative_votes).toBe(12)
    expect(parsed.value.l3.required_votes).toBe(103)
    expect(parsed.value.failing_tier).toBe('L3_SPECTRAL_QUORUM')
  })

  it('rejects a fail-closed error body and a partial breach', () => {
    expect(parseChatBreach({ error: 'Fail-closed: embedding subsystem error: boom' }).ok).toBe(
      false,
    )
    expect(parseChatBreach({ decision: 'BREACH' }).ok).toBe(false)
    expect(
      parseChatBreach({
        ...l1,
        l1: { passed: false, cosine: 'low', threshold: 0.5 },
      }).ok,
    ).toBe(false)
    expect(
      parseChatBreach({
        ...l3,
        l3: { evaluated: true, passed: false },
      }).ok,
    ).toBe(false)
  })
})
