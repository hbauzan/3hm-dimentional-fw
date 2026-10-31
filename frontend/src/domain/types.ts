export const FAILING_TIERS = [
  'L1_CENTROID_COSINE',
  'L2_CLAUSE_GATE',
  'L3_SPECTRAL_QUORUM',
] as const

export type FailingTier = (typeof FAILING_TIERS)[number]

export type ChatRole = 'user' | 'assistant' | 'system'

export interface ChatMessage {
  role: ChatRole
  content: string
}

export interface ChatOverrides {
  l1_threshold: number
  l2_threshold: number
  quorum_k: number
  epsilon: number
}

export const UPSTREAM_PROVIDERS = ['mock', 'deepseek', 'openai', 'gemini'] as const

export type UpstreamProvider = (typeof UPSTREAM_PROVIDERS)[number]

export interface UpstreamSelection {
  provider: UpstreamProvider
  baseUrl: string
  model: string
  apiKey: string
}

export interface ChatUpstreamBody {
  provider: UpstreamProvider
  model?: string
  base_url?: string
  api_key?: string
}

export interface ChatRequest {
  messages: ChatMessage[]
  domain_id?: string
  overrides?: ChatOverrides
  upstream: ChatUpstreamBody
}

export interface ChatDraft {
  messages: readonly ChatMessage[]
  domainId: string
  overridesArmed: boolean
  overrides: ChatOverrides
  upstream: UpstreamSelection
}

export interface Tier1Breach {
  passed: boolean
  cosine: number
  threshold: number
}

export interface Tier2Breach {
  passed: boolean
  threshold: number
  breached_clause_index?: number
  breached_clause_text?: string
  clause_cosine?: number
}

export interface Tier3Breach {
  evaluated: boolean
  passed?: boolean
  affirmative_votes?: number
  required_votes?: number
}

export interface ChatBreachDto {
  decision: 'BREACH'
  failing_tier: FailingTier
  l1: Tier1Breach
  l2: Tier2Breach
  l3: Tier3Breach
}

export type ParseResult<T> = { ok: true; value: T } | { ok: false; message: string }

export interface SpaSession {
  gatewayBase: string
  domainId: string
  overridesArmed: boolean
  overrides: ChatOverrides
  upstream: UpstreamSelection
}

export type ChatOutcome =
  | { kind: 'pass'; text: string }
  | { kind: 'breach'; breach: ChatBreachDto }
  | { kind: 'error'; status: number; message: string }
