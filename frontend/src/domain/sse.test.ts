import { describe, expect, it } from 'vitest'
import { createSseDecoder } from './sse.ts'

function delta(content: string): string {
  return `data: ${JSON.stringify({ choices: [{ delta: { content } }] })}\n\n`
}

describe('createSseDecoder', () => {
  it('joins a delta split across chunks and ignores [DONE]', () => {
    const decoder = createSseDecoder()
    expect(decoder.push('data: {"choices":[{"delta":{"content":"Ho')).toEqual([])
    expect(decoder.push('la"}}]}\n\n')).toEqual(['Hola'])
    expect(decoder.push('data: [DONE]\r\n\r\n')).toEqual([])
    expect(decoder.flush()).toEqual([])
  })

  it('emits each completed content delta in order', () => {
    const decoder = createSseDecoder()
    const pieces = decoder.push(`${delta('He')}${delta('llo')}data: [DONE]\n\n`)
    expect(pieces).toEqual(['He', 'llo'])
  })

  it('surfaces a mock chunk that has no choices array', () => {
    const decoder = createSseDecoder()
    const raw = '{"object":"chat.completion.chunk","mock":true}'
    expect(decoder.push(`data: ${raw}\n\n`)).toEqual([raw])
  })

  it('ignores comment lines and flushes a trailing data line', () => {
    const decoder = createSseDecoder()
    expect(decoder.push(': keep-alive\n')).toEqual([])
    expect(decoder.push(delta('Z').replace(/\n\n$/, ''))).toEqual([])
    expect(decoder.flush()).toEqual(['Z'])
  })
})
