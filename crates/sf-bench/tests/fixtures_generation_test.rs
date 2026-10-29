use sf_bench::{assert_publication_bounds, evaluate_matrix, generate_corpus, FIXTURE_SEED};

#[test]
fn cha_cha_corpus_matches_rompepepe_bounds() {
    assert_eq!(FIXTURE_SEED, 0xDD10_0400);
    let corpus = generate_corpus();
    assert_eq!(corpus.locks.len(), 55);
    assert_eq!(corpus.attacks.len(), 220);
    let matrix = evaluate_matrix(&corpus);
    assert_publication_bounds(&matrix);
}
