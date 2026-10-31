import { normalizeOverrides } from './overrides.ts'
import type {
  ChatDraft,
  ChatMessage,
  ChatRequest,
  ChatUpstreamBody,
  UpstreamSelection,
} from './types.ts'

export function buildChatRequest(draft: ChatDraft): ChatRequest {
  const request: ChatRequest = {
    messages: draft.messages.map((message) => ({
      role: message.role,
      content: message.content,
    })),
    upstream: upstreamBody(draft.upstream),
  }
  const domainId = draft.domainId.trim()
  if (domainId) request.domain_id = domainId
  if (draft.overridesArmed) request.overrides = normalizeOverrides(draft.overrides)
  return request
}

export function hasUserText(messages: readonly ChatMessage[]): boolean {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index]
    if (message?.role === 'user' && message.content.trim()) return true
  }
  return false
}

function upstreamBody(selection: UpstreamSelection): ChatUpstreamBody {
  const body: ChatUpstreamBody = { provider: selection.provider }
  const model = selection.model.trim()
  const baseUrl = selection.baseUrl.trim()
  const apiKey = selection.apiKey.trim()
  if (model) body.model = model
  if (baseUrl) body.base_url = baseUrl
  if (apiKey) body.api_key = apiKey
  return body
}
