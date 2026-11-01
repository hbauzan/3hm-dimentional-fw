import { describe, expect, it } from 'vitest'
import { CANONICAL_DOMAINS, isCanonicalDomain, type CanonicalDomain } from './presets.ts'

const REGISTRY_ORDER = [
  'python',
  'receta',
  'legal',
  'medicina',
  'astronomia',
  'finanzas',
  'filosofia',
  'musica',
  'geologia',
  'botanica',
  'arquitectura',
] as const satisfies readonly CanonicalDomain[]

describe('CANONICAL_DOMAINS', () => {
  it('matches the sf-storage calibration registry in list order', () => {
    expect([...CANONICAL_DOMAINS]).toEqual([...REGISTRY_ORDER])
    expect(CANONICAL_DOMAINS).toHaveLength(11)
    expect(isCanonicalDomain('legal')).toBe(true)
    expect(isCanonicalDomain('python_receta')).toBe(false)
    expect(isCanonicalDomain('')).toBe(false)
  })
})
