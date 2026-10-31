import type { ChatOverrides } from './types.ts'

export function clampFloat(
  value: number,
  min: number,
  max: number,
  digits: number,
  fallback: number,
): number {
  if (!Number.isFinite(value)) return fallback
  const clamped = Math.min(max, Math.max(min, value))
  const factor = 10 ** digits
  return Math.round(clamped * factor) / factor
}

export function clampInt(value: number, min: number, max: number, fallback: number): number {
  if (!Number.isFinite(value)) return fallback
  return Math.min(max, Math.max(min, Math.round(value)))
}

export function normalizeOverrides(overrides: ChatOverrides): ChatOverrides {
  return {
    l1_threshold: clampFloat(overrides.l1_threshold, 0, 1, 2, 0.5),
    l2_threshold: clampFloat(overrides.l2_threshold, 0, 1, 2, 0.5),
    quorum_k: clampInt(overrides.quorum_k, 1, 1015, 103),
    epsilon: clampFloat(overrides.epsilon, 0, 0.5, 3, 0),
  }
}

export function readStoredFloat(
  raw: string | null,
  fallback: number,
  min: number,
  max: number,
  digits: number,
): number {
  if (raw === null || raw.trim() === '') return fallback
  return clampFloat(Number(raw), min, max, digits, fallback)
}

export function readStoredInt(
  raw: string | null,
  fallback: number,
  min: number,
  max: number,
): number {
  if (raw === null || raw.trim() === '') return fallback
  return clampInt(Number(raw), min, max, fallback)
}
