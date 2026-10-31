import { isRecord } from './json.ts'

export interface SseDecoder {
  push(chunk: string): string[]
  flush(): string[]
}

export function createSseDecoder(): SseDecoder {
  let buffer = ''
  return {
    push(chunk) {
      buffer += chunk
      const lines = buffer.split(/\r?\n/)
      buffer = lines.pop() ?? ''
      return emit(lines)
    },
    flush() {
      const pending = buffer
      buffer = ''
      if (!pending.trim()) return []
      return emit([pending])
    },
  }
}

function emit(lines: string[]): string[] {
  const pieces: string[] = []
  for (const line of lines) {
    const piece = deltaFromSseLine(line)
    if (piece) pieces.push(piece)
  }
  return pieces
}

export function deltaFromSseLine(line: string): string {
  const trimmed = line.trim()
  if (!trimmed.startsWith('data:')) return ''
  const data = trimmed.slice(5).trim()
  if (!data || data === '[DONE]') return ''
  try {
    return contentFromChunk(JSON.parse(data) as unknown, data)
  } catch {
    return data
  }
}

function contentFromChunk(chunk: unknown, raw: string): string {
  if (!isRecord(chunk)) return raw
  if (!Object.hasOwn(chunk, 'choices')) return raw
  const choices = chunk.choices
  if (!Array.isArray(choices)) return ''
  const choice = choices[0]
  if (!isRecord(choice) || !isRecord(choice.delta)) return ''
  return typeof choice.delta.content === 'string' ? choice.delta.content : ''
}
