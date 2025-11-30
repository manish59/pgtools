use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{anyhow, Result};
use clap::Parser;
use serde::Serialize;
use serde_json;

#[derive(Debug, Serialize)]
struct SampleSummary {
    sample: String,
    path_count: u64,
}

#[derive(Debug, Serialize)]
struct PathsStats {
    total_paths: u64,
    samples: Vec<SampleSummary>,
}

/// Extract path & population stats from a VG XG index (.xg) using `vg paths`.
#[derive(Debug, Parser)]
#[command(name = "pgtools-stats-paths", version, about)]
struct Args {
    /// Input XG file
    #[arg(value_name = "XG")]
    xg_path: PathBuf,

    /// Output JSON instead of text
    #[arg(long)]
    json: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let stats = compute_paths_stats(&args.xg_path)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&stats)?);
        return Ok(());
    }

    println!("Path stats for {}", args.xg_path.display());
    println!("-----------------------------------------");
    println!("Total paths (haplotypes): {}", stats.total_paths);
    println!("Samples:");
    for s in &stats.samples {
        println!("  {} -> {} paths", s.sample, s.path_count);
    }

    Ok(())
}

fn compute_paths_stats(xg_path: &PathBuf) -> Result<PathsStats> {
    // Requires `vg` in PATH
    let output = Command::new("vg")
        .args(["paths", "-L", "-x"])
        .arg(xg_path)
        .output()
        .map_err(|e| anyhow!("failed to invoke vg: {e}"))?;

    if !output.status.success() {
        return Err(anyhow!(
            "vg paths failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8(output.stdout)?;
    let mut sample_map: HashMap<String, u64> = HashMap::new();
    let mut total_paths: u64 = 0;

    for line in stdout.lines() {
        let name = line.trim();
        if name.is_empty() {
            continue;
        }
        total_paths += 1;

        // HPRC naming convention: SAMPLE#hap#chr
        let sample = name.split('#').next().unwrap_or(name).to_string();
        *sample_map.entry(sample).or_insert(0) += 1;
    }

    let mut samples: Vec<SampleSummary> = sample_map
        .into_iter()
        .map(|(sample, path_count)| SampleSummary { sample, path_count })
        .collect();

    samples.sort_by(|a, b| a.sample.cmp(&b.sample));

    Ok(PathsStats {
        total_paths,
        samples,
    })
}
