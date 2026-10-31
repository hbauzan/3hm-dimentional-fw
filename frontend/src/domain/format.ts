export interface TextPart {
  kind: 'text'
  text: string
}

export interface CodePart {
  kind: 'code'
  text: string
}

export type AssistantPart = TextPart | CodePart

export function formatGate(score: number, threshold: number): string {
  const operator = score < threshold ? '<' : score > threshold ? '>' : '='
  return `${score.toFixed(4)} ${operator} Threshold: ${threshold.toFixed(4)}`
}

export function splitFences(source: string): AssistantPart[] {
  const parts: AssistantPart[] = []
  const chunks = source.split('```')
  chunks.forEach((chunk, index) => {
    if (index % 2 === 1) {
      parts.push({ kind: 'code', text: stripFence(chunk) })
      return
    }
    if (chunk) parts.push({ kind: 'text', text: chunk })
  })
  return parts
}

function stripFence(chunk: string): string {
  let code = chunk.replace(/^\r?\n/, '')
  const firstBreak = code.indexOf('\n')
  if (firstBreak !== -1) {
    const lang = code.slice(0, firstBreak).trim()
    if (/^[A-Za-z0-9_+.-]*$/.test(lang)) code = code.slice(firstBreak + 1)
  }
  return code.replace(/\n$/, '')
}

export function clampUnit(value: number): number {
  if (!Number.isFinite(value)) return 0
  return Math.min(1, Math.max(0, value))
}
