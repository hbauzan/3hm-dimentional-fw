import type { ParseResult } from './types.ts'

export function ok<T>(value: T): ParseResult<T> {
  return { ok: true, value }
}

export function fail<T>(message: string): ParseResult<T> {
  return { ok: false, message }
}
