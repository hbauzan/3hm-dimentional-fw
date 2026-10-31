import { describe, expect, it } from 'vitest'
import { formatGate, splitFences } from './format.ts'

describe('formatGate', () => {
  it('prints the cosine comparison the breach card shows', () => {
    expect(formatGate(0.42, 0.65)).toBe('0.4200 < Threshold: 0.6500')
    expect(formatGate(0.5, 0.5)).toBe('0.5000 = Threshold: 0.5000')
    expect(formatGate(0.91, 0.5)).toBe('0.9100 > Threshold: 0.5000')
  })
})

describe('splitFences', () => {
  it('separates prose from a fenced block and strips the language tag', () => {
    expect(splitFences('hello\n```python\nprint(1)\n```\nbye')).toEqual([
      { kind: 'text', text: 'hello\n' },
      { kind: 'code', text: 'print(1)' },
      { kind: 'text', text: '\nbye' },
    ])
  })
})
