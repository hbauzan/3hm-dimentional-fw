use std::path::PathBuf;

use clap::Parser;
use sf_storage::{calibrate_matrix, read_f32_matrix, write_calibration, IngestSpec};

#[derive(Debug, Parser)]
#[command(name = "sf-ingest")]
struct Args {
    #[arg(long)]
    domain_id: String,
    #[arg(long, default_value_t = 0)]
    polarity: u8,
    #[arg(long, default_value_t = 0.0)]
    epsilon: f32,
    #[arg(long, default_value_t = 0.5)]
    l1_threshold: f32,
    #[arg(long, default_value_t = 0.5)]
    l2_threshold: f32,
    #[arg(long, default_value_t = 103)]
    quorum_k: u16,
    #[arg(long, default_value_t = 0)]
    extra_pruned_dims: usize,
    /// Little-endian file: `u64` row count, then `rows * 1024` binary32 values.
    #[arg(long)]
    input_f32: PathBuf,
    #[arg(long)]
    lancedb_uri: String,
    #[arg(long, default_value_t = 1)]
    version: u64,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    if let Err(err) = run(args).await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

async fn run(args: Args) -> Result<(), String> {
    let bytes = std::fs::read(&args.input_f32).map_err(|err| err.to_string())?;
    let matrix = read_f32_matrix(&bytes).map_err(|err| err.to_string())?;
    let calibration =
        calibrate_matrix(&matrix, args.extra_pruned_dims).map_err(|err| err.to_string())?;
    write_calibration(IngestSpec {
        uri: &args.lancedb_uri,
        domain_id: &args.domain_id,
        version: args.version,
        polarity: args.polarity,
        l1: args.l1_threshold,
        l2: args.l2_threshold,
        epsilon: args.epsilon,
        quorum_k: args.quorum_k,
        calibration: &calibration,
    })
    .await
    .map_err(|err| err.to_string())?;
    println!(
        "ingested {} rows into {} active_dims={}",
        calibration.rows,
        args.domain_id,
        calibration.noise_mask.active_count()
    );
    Ok(())
}
