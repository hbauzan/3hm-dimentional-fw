import { isRecord } from './json.ts'
import { fail, ok } from './result.ts'
import { FAILING_TIERS, type ChatBreachDto, type ChatTelemetryDto, type FailingTier, type ParseResult, type Tier1Breach, type Tier2Breach, type Tier3Breach } from './types.ts'

export function parseChatBreach(value: unknown): ParseResult<ChatBreachDto> {
  if (!isRecord(value)) return fail('breach payload is not an object')
  if (value.decision !== 'BREACH') return fail('decision is not BREACH')
  if (!isFailingTier(value.failing_tier)) return fail('failing_tier is not a chat head')
  const l1 = parseTier1(value.l1)
  if (!l1.ok) return l1
  const l2 = parseTier2(value.l2)
  if (!l2.ok) return l2
  const l3 = parseTier3(value.l3)
  if (!l3.ok) return l3
  return ok({
    decision: 'BREACH',
    failing_tier: value.failing_tier,
    l1: l1.value,
    l2: l2.value,
    l3: l3.value,
  })
}

export function parseChatTelemetry(value: unknown): ParseResult<ChatTelemetryDto> {
  if (!isRecord(value)) return fail('telemetry payload is not an object')
  if (value.decision !== 'BREACH' && value.decision !== 'PASS') return fail('decision is invalid')
  const l1 = parseTier1(value.l1)
  if (!l1.ok) return l1
  const l2 = parseTier2(value.l2)
  if (!l2.ok) return l2
  const l3 = parseTier3(value.l3)
  if (!l3.ok) return l3
  return ok({
    decision: value.decision,
    failing_tier: typeof value.failing_tier === 'string' ? value.failing_tier : undefined,
    l1: l1.value,
    l2: l2.value,
    l3: l3.value,
  })
}

function isFailingTier(value: unknown): value is FailingTier {
  return typeof value === 'string' && (FAILING_TIERS as readonly string[]).includes(value)
}

function parseTier1(value: unknown): ParseResult<Tier1Breach> {
  if (!isRecord(value)) return fail('l1 is missing')
  const passed = readBoolean(value.passed)
  const cosine = readFinite(value.cosine)
  const threshold = readFinite(value.threshold)
  if (passed === undefined || cosine === undefined || threshold === undefined) {
    return fail('l1 cosine gate is incomplete')
  }
  return ok({ passed, cosine, threshold })
}

function parseTier2(value: unknown): ParseResult<Tier2Breach> {
  if (!isRecord(value)) return fail('l2 is missing')
  const passed = readBoolean(value.passed)
  const threshold = readFinite(value.threshold)
  if (passed === undefined || threshold === undefined) return fail('l2 clause gate is incomplete')
  const index = readOptionalInt(value, 'breached_clause_index')
  if (!index.ok) return index
  const text = readOptionalString(value, 'breached_clause_text')
  if (!text.ok) return text
  const cosine = readOptionalFinite(value, 'clause_cosine')
  if (!cosine.ok) return cosine
  const tier: Tier2Breach = { passed, threshold }
  if (index.value !== undefined) tier.breached_clause_index = index.value
  if (text.value !== undefined) tier.breached_clause_text = text.value
  if (cosine.value !== undefined) tier.clause_cosine = cosine.value
  return ok(tier)
}

function parseTier3(value: unknown): ParseResult<Tier3Breach> {
  if (!isRecord(value)) return fail('l3 is missing')
  const evaluated = readBoolean(value.evaluated)
  if (evaluated === undefined) return fail('l3.evaluated is not a boolean')
  const passed = readOptionalBoolean(value, 'passed')
  if (!passed.ok) return passed
  const votes = readOptionalInt(value, 'affirmative_votes')
  if (!votes.ok) return votes
  const required = readOptionalInt(value, 'required_votes')
  if (!required.ok) return required
  if (
    evaluated &&
    (passed.value === undefined || votes.value === undefined || required.value === undefined)
  ) {
    return fail('evaluated l3 is missing quorum votes')
  }
  const tier: Tier3Breach = { evaluated }
  if (passed.value !== undefined) tier.passed = passed.value
  if (votes.value !== undefined) tier.affirmative_votes = votes.value
  if (required.value !== undefined) tier.required_votes = required.value
  return ok(tier)
}

function readBoolean(value: unknown): boolean | undefined {
  return typeof value === 'boolean' ? value : undefined
}

function readFinite(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined
}

function readInt(value: unknown): number | undefined {
  const number = readFinite(value)
  return number !== undefined && Number.isInteger(number) && number >= 0 ? number : undefined
}

function readOptionalFinite(
  record: Record<string, unknown>,
  key: string,
): ParseResult<number | undefined> {
  if (!Object.hasOwn(record, key) || record[key] === undefined) return ok(undefined)
  const number = readFinite(record[key])
  if (number === undefined) return fail(`${key} is not a finite number`)
  return ok(number)
}

function readOptionalInt(
  record: Record<string, unknown>,
  key: string,
): ParseResult<number | undefined> {
  if (!Object.hasOwn(record, key) || record[key] === undefined) return ok(undefined)
  const number = readInt(record[key])
  if (number === undefined) return fail(`${key} is not a non-negative integer`)
  return ok(number)
}

function readOptionalBoolean(
  record: Record<string, unknown>,
  key: string,
): ParseResult<boolean | undefined> {
  if (!Object.hasOwn(record, key) || record[key] === undefined) return ok(undefined)
  const flag = readBoolean(record[key])
  if (flag === undefined) return fail(`${key} is not a boolean`)
  return ok(flag)
}

function readOptionalString(
  record: Record<string, unknown>,
  key: string,
): ParseResult<string | undefined> {
  if (!Object.hasOwn(record, key) || record[key] === undefined) return ok(undefined)
  if (typeof record[key] !== 'string') return fail(`${key} is not a string`)
  return ok(record[key])
}
