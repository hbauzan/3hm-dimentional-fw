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

  it('maps a pairwise envelope onto L3_PAIRWISE_QUORUM', () => {
    const parsed = parseChatBreach({
      error: {
        kind: 'ddi_ingress_breach',
        code: 'SEMANTIC_CONTAINMENT_BREACH',
        message:
          "Fail-closed containment breach on pairwise lock 'python_receta' for clause #1.",
        telemetry: {
          domain_id: 'python',
          breaching_pair_lock: 'python_receta',
          breaching_clause_index: 1,
          failing_tier: 'L3_PAIRWISE_QUORUM',
        },
      },
    })
    expect(parsed.ok).toBe(true)
    if (!parsed.ok) return
    expect(parsed.value.decision).toBe('BREACH')
    expect(parsed.value.failing_tier).toBe('L3_PAIRWISE_QUORUM')
    expect(parsed.value.l2.passed).toBe(false)
    expect(parsed.value.l2.breached_clause_text).toBe('python_receta')
    expect(parsed.value.l2.breached_clause_index).toBe(1)

    const promptLevel = parseChatBreach({
      error: {
        kind: 'ddi_ingress_breach',
        code: 'SEMANTIC_CONTAINMENT_BREACH',
        message: 'full prompt',
        telemetry: {
          domain_id: 'python',
          breaching_pair_lock: 'python_receta',
          breaching_clause_index: null,
          failing_tier: 'L3_PAIRWISE_QUORUM',
        },
      },
    })
    expect(promptLevel.ok).toBe(true)
    if (!promptLevel.ok) return
    expect(promptLevel.value.l2.breached_clause_index).toBeUndefined()
    expect(promptLevel.value.l2.breached_clause_text).toBe('python_receta')
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
