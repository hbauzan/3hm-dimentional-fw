//! Rompepepe fixtures and the in-process load harness.

mod canonical_corpus;
mod synthetic_fixtures;

pub use canonical_corpus::{canonical_seed_corpus, CanonicalDomainCorpus};

pub use synthetic_fixtures::{
    assert_publication_bounds, evaluate_matrix, generate_canonical_locks, generate_corpus,
    AttackCase, EvalMatrix, RompepepeCorpus, DOMAIN_NAMES, FIXTURE_SEED,
};
