import { fail, ok } from './result.ts'
import type { ParseResult } from './types.ts'

export function resolveChatEndpoint(gatewayBase: string): ParseResult<string> {
  const trimmed = gatewayBase.trim()
  if (!trimmed) return fail('gateway base is empty')
  let url: URL
  try {
    url = new URL(trimmed)
  } catch {
    return fail('gateway base is not an absolute URL')
  }
  if (url.protocol !== 'http:' && url.protocol !== 'https:') {
    return fail('gateway base must be http or https')
  }
  const path = url.pathname.replace(/\/+$/, '').replace(/\/api\/v1\/chat$/, '')
  return ok(`${url.origin}${path}/api/v1/chat`)
}
