//! Rompepepe fixtures and the in-process load harness.

mod synthetic_fixtures;

pub use synthetic_fixtures::{
    assert_publication_bounds, evaluate_matrix, generate_canonical_locks, generate_corpus,
    AttackCase, EvalMatrix, RompepepeCorpus, DOMAIN_NAMES, FIXTURE_SEED,
};
