use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::Request;
use bytes::Bytes;
use clap::Parser;
use hdrhistogram::Histogram;
use http_body_util::BodyExt;
use sf_kernels::{AlignedVec1024, DomainHypercubeLock, LockSpec, NoiseBitmask1024, DIM, MAX_TOPK};
use sf_proxy::{router, MockBatchEmbedder, MockUpstreamForwarder, ProxyAppState};
use sf_storage::MemoryRegistry;
use tokio::sync::Semaphore;
use tower::ServiceExt;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, default_value_t = 1000)]
    rps: u64,
    #[arg(long, default_value_t = 60)]
    duration_secs: u64,
    #[arg(long, default_value_t = 2.0)]
    assert_p99_ms: f64,
}

fn pass_lock() -> DomainHypercubeLock {
    let mut centroid = AlignedVec1024::zeros();
    centroid.data[0] = 1.0;
    let mut lo = AlignedVec1024::zeros();
    let mut hi = AlignedVec1024::zeros();
    lo.data.fill(-0.05);
    hi.data.fill(0.05);
    hi.data[0] = 1.2;
    DomainHypercubeLock::from_spec(LockSpec {
        domain_id: "python".to_string(),
        centroid,
        lo,
        hi,
        epsilon: 0.0,
        noise_mask: NoiseBitmask1024::new_bge_m3_default(),
        top_k_indices: [0u16; MAX_TOPK],
        top_k_len: 1,
        quorum_k: 103,
        l1_cosine_threshold: 0.2,
        l2_clause_threshold: 0.2,
        polarity: 0,
        domain_version: 1,
    })
}

fn prompt_m16() -> String {
    (0..16)
        .map(|idx| format!("clause {idx} stays inside the python lock"))
        .collect::<Vec<_>>()
        .join(". ")
}

fn rss_mb() -> f64 {
    let pid = std::process::id().to_string();
    let output = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &pid])
        .output()
        .expect("ps rss");
    let text = String::from_utf8_lossy(&output.stdout);
    let kb: f64 = text.trim().parse().unwrap_or(0.0);
    kb / 1024.0
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let centroid = {
        let lock = pass_lock();
        lock.centroid
    };
    let state = Arc::new(ProxyAppState {
        registry: MemoryRegistry::from_locks([pass_lock()]),
        embedder: Arc::new(MockBatchEmbedder {
            pass_vector: centroid,
            breach_vector: AlignedVec1024 { data: [0.0; DIM] },
        }),
        upstream: Arc::new(MockUpstreamForwarder),
        default_domain: "python".to_string(),
        max_telemetry_coords: 1,
    });
    let app = router(state);
    let body = Bytes::from(format!(
        r#"{{"messages":[{{"role":"user","content":"{}"}}]}}"#,
        prompt_m16()
    ));
    let total = args.rps.saturating_mul(args.duration_secs).max(1);
    let interval = Duration::from_nanos(1_000_000_000 / args.rps.max(1));
    let semaphore = Arc::new(Semaphore::new(128));
    let mut histogram = Histogram::<u64>::new(3).expect("histogram");
    let mut next = tokio::time::Instant::now();
    let mut inflight = tokio::task::JoinSet::new();
    for _ in 0..total {
        tokio::time::sleep_until(next).await;
        next += interval;
        let permit = semaphore.clone().acquire_owned().await.expect("permit");
        let app = app.clone();
        let body = body.clone();
        inflight.spawn(async move {
            let started = Instant::now();
            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/chat/completions")
                        .header("content-type", "application/json")
                        .header("x-semantic-domain", "python")
                        .body(Body::from(body))
                        .unwrap(),
                )
                .await
                .expect("response");
            let status = response.status();
            let _ = response.into_body().collect().await;
            drop(permit);
            (status, started.elapsed())
        });
        while let Some(joined) = inflight.try_join_next() {
            record(&mut histogram, joined);
        }
    }
    while let Some(joined) = inflight.join_next().await {
        record(&mut histogram, joined);
    }
    let p50_ms = histogram.value_at_quantile(0.50) as f64 / 1_000_000.0;
    let p99_ms = histogram.value_at_quantile(0.99) as f64 / 1_000_000.0;
    let rss = rss_mb();
    println!("requests={total} p50_ms={p50_ms:.4} p99_ms={p99_ms:.4} rss_mb={rss:.2}");
    let mut failed = false;
    if p99_ms >= args.assert_p99_ms.min(1.50) {
        eprintln!("p99 {p99_ms:.4} ms exceeds the SLA");
        failed = true;
    }
    if p50_ms >= 0.15 {
        eprintln!("p50 {p50_ms:.4} ms exceeds 0.15 ms");
        failed = true;
    }
    if rss >= 45.0 {
        eprintln!("rss {rss:.2} MB exceeds 45 MB");
        failed = true;
    }
    if failed {
        std::process::exit(1);
    }
}

fn record(
    histogram: &mut Histogram<u64>,
    joined: Result<(axum::http::StatusCode, Duration), tokio::task::JoinError>,
) {
    let (status, elapsed) = joined.expect("task");
    assert!(status.is_success(), "{status}");
    let nanos = u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX);
    histogram.record(nanos.max(1)).expect("record");
}
