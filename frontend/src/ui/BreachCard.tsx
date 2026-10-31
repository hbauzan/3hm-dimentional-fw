import { clampUnit, formatGate } from '../domain/format.ts'
import type { ChatBreachDto, Tier1Breach, Tier3Breach } from '../domain/types.ts'

export function BreachCard({ breach }: { breach: ChatBreachDto }) {
  return (
    <article
      aria-label="Semantic containment breach"
      className="border border-studio-warn bg-studio-card px-3 py-3 text-xs"
    >
      <p className="text-[11px] tracking-wide text-studio-warn">[SEMANTIC CONTAINMENT BREACH]</p>
      <p className="mt-1 font-semibold text-studio-breach">{breach.failing_tier}</p>
      <Tier1 tier={breach.l1} />
      <Tier2 breach={breach} />
      <Tier3 tier={breach.l3} />
    </article>
  )
}

function Tier1({ tier }: { tier: Tier1Breach }) {
  const width = `${clampUnit(tier.cosine) * 100}%`
  const mark = `${clampUnit(tier.threshold) * 100}%`
  return (
    <section aria-label="Tier 1 centroid cosine" className="mt-3">
      <h3 className="text-studio-muted">Tier 1 — Centroid Cosine</h3>
      <p>Cosine: {formatGate(tier.cosine, tier.threshold)}</p>
      <StatusPill passed={tier.passed} />
      <div
        role="img"
        aria-label="centroid cosine proximity"
        className="relative mt-1 h-2 border border-studio-line bg-studio-bg"
      >
        <div
          className={tier.passed ? 'h-full bg-studio-pass' : 'h-full bg-studio-breach'}
          style={{ width }}
        />
        <div className="absolute top-0 h-full w-px bg-studio-accent" style={{ left: mark }} />
      </div>
    </section>
  )
}

function Tier2({ breach }: { breach: ChatBreachDto }) {
  const tier = breach.l2
  return (
    <section aria-label="Tier 2 clause gate" className="mt-3">
      <h3 className="text-studio-muted">Tier 2 — Clause Gate</h3>
      {breach.failing_tier === 'L1_CENTROID_COSINE' ? (
        <p>Skipped (cascaded at L1).</p>
      ) : tier.passed ? (
        <>
          <StatusPill passed />
          <p>Threshold: {tier.threshold.toFixed(4)}</p>
        </>
      ) : (
        <>
          <StatusPill passed={false} />
          <p className="text-studio-breach">
            #{tier.breached_clause_index ?? '?'} {tier.breached_clause_text ?? ''}
            {tier.clause_cosine === undefined
              ? ''
              : ` — Cosine: ${formatGate(tier.clause_cosine, tier.threshold)}`}
          </p>
        </>
      )}
    </section>
  )
}

function Tier3({ tier }: { tier: Tier3Breach }) {
  return (
    <section aria-label="Tier 3 spectral quorum" className="mt-3">
      <h3 className="text-studio-muted">Tier 3 — Spectral Quorum</h3>
      {tier.evaluated &&
      tier.affirmative_votes !== undefined &&
      tier.required_votes !== undefined ? (
        <>
          <p>
            Affirmative votes {tier.affirmative_votes} / K {tier.required_votes}
          </p>
          {typeof tier.passed === 'boolean' ? <StatusPill passed={tier.passed} /> : null}
        </>
      ) : (
        <p>Skipped</p>
      )}
    </section>
  )
}

function StatusPill({ passed }: { passed: boolean }) {
  return (
    <p className={passed ? 'text-studio-pass' : 'text-studio-breach'}>
      {passed ? '[PASS]' : '[BREACH]'}
    </p>
  )
}
