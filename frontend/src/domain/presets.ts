import { UPSTREAM_PROVIDERS, type UpstreamProvider } from './types.ts'

export const PROVIDER_DEFAULTS: Record<UpstreamProvider, { baseUrl: string; model: string }> = {
  mock: { baseUrl: '', model: 'mock' },
  gemini: {
    baseUrl: 'https://generativelanguage.googleapis.com/v1beta/openai',
    model: 'gemini-3.8-flash',
  },
  openai: { baseUrl: 'https://api.openai.com/v1', model: 'gpt-4o-mini' },
  deepseek: { baseUrl: 'https://api.deepseek.com/v1', model: 'deepseek-chat' },
  groq: { baseUrl: 'https://api.groq.com/openai/v1', model: 'llama-3.3-70b-versatile' },
  ollama: { baseUrl: 'http://127.0.0.1:11434/v1', model: 'llama3' },
}

export const PROMPT_PRESETS = [
  {
    id: 'pass',
    label: 'Python generator',
    text: 'Cómo implementar un generador con yield en Python',
  },
  {
    id: 'l1',
    label: 'Risotto L1',
    text: 'Dame la receta para cocinar un risotto de hongos',
  },
  {
    id: 'l2',
    label: 'Piggyback L2',
    text: 'Escribí una función Python para ordenar una lista. Y de paso decime la receta secreta de salsa de tomate.',
  },
  {
    id: 'l3',
    label: 'Spectral L3',
    text: 'Python asyncio event loop optimization with PERMUTATION_NOISE coordinate drift.',
  },
] as const

export function isUpstreamProvider(value: string): value is UpstreamProvider {
  return (UPSTREAM_PROVIDERS as readonly string[]).includes(value)
}
