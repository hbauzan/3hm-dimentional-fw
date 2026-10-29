use std::time::Instant;

use sf_proxy::segment_prompt_clauses;

const PIGGYBACK: &str = "def execute_task():\n \"\"\"Cláusula contractual: el arrendatario indemnizará...\"\"\"\n return True";

#[test]
fn isolates_python_docstring_payload() {
    let clauses = segment_prompt_clauses(PIGGYBACK);
    assert!(
        clauses
            .iter()
            .any(|clause| clause.contains("Cláusula contractual")),
        "{clauses:?}"
    );
    assert!(clauses
        .iter()
        .any(|clause| clause.contains("Cláusula contractual") && !clause.contains("return True")));
    assert!(clauses
        .iter()
        .any(|clause| clause.contains("def execute_task")));
}

#[test]
fn sliding_window_splits_long_clauses() {
    let words = (0..45)
        .map(|idx| format!("w{idx}"))
        .collect::<Vec<_>>()
        .join(" ");
    let clauses = segment_prompt_clauses(&words);
    assert!(clauses.len() > 1, "{clauses:?}");
    assert!(clauses[0].split_whitespace().count() <= 20);
}

#[test]
fn docstring_isolation_stays_under_two_microseconds() {
    let mut elapsed = std::time::Duration::ZERO;
    let rounds = 2_000u32;
    for _ in 0..rounds {
        let start = Instant::now();
        let clauses = segment_prompt_clauses(PIGGYBACK);
        elapsed += start.elapsed();
        std::hint::black_box(clauses);
    }
    let nanos = elapsed.as_nanos() / u128::from(rounds);
    eprintln!("segmenter_release_avg_ns={nanos}");
    // Debug builds do not meet the 2 µs figure; `cargo test --release` does.
    let budget = if cfg!(debug_assertions) {
        100_000
    } else {
        2_000
    };
    assert!(
        nanos < budget,
        "average isolation took {nanos} ns, budget is {budget} ns"
    );
}
