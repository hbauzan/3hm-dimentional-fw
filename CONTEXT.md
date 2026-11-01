# CONTEXT — sf-core ubiquitous language

## DomainHypercubeLock

A calibrated domain: centroid, pre-dilated coordinate bounds, noise mask, Top-K indices, quorum, cosine thresholds, and polarity.

## AlignedVec1024

A 1024-dimensional IEEE 754 binary32 tensor. Layout is 4096 bytes, aligned to 64 bytes.

## NoiseBitmask1024

A 1024-bit mask. A set bit marks an active coordinate. A clear bit marks pruned noise.

## SpectralQuorum

Tier-3 Mode C rule: an allowlist passes when affirmative in-box votes reach quorum `K` (default 103) over active coordinates.

## UniversalNoiseIndices

The nine BGE-M3 background coordinates cleared by the default mask: 292, 297, 308, 329, 386, 404, 577, 616, 780. Active count is 1015.

## ClauseSegmenter

The deterministic splitter that yields borrowed clause slices from one prompt.

## PairwiseDifferentialLock

A **DomainHypercubeLock** whose id is `left_right` for two canonical domains. The coordinate envelope belongs to `left`.

## AnyAllowlist

Wildcard policy for `X-Semantic-Domain: *`. One allowlist must contain the prompt and every clause.

## AllLocks

Wildcard policy selected by `X-SF-Wildcard-Mode: all`. Every selected lock must pass.

## Denylist

A lock with `polarity == 1`. When several denylists are in scope, all of them must pass.

## Allowlist

A lock with `polarity == 0`. Containment means the vector stays inside that lock's gates.

## _Avoid_

- "embedding box", "filter", "rule" — use **DomainHypercubeLock**
- "vector", "tensor buffer" — use **AlignedVec1024** when the value is a 1024-d binary32 tensor
- "threshold vote" — use **SpectralQuorum**
