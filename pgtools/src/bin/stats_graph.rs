use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use pgtools::compute_graph_stats_from_path;
use serde_json;

/// Graph topology statistics (N50, degrees, branching, etc.)
#[derive(Debug, Parser)]
#[command(name = "pgtools-stats-graph", version, about)]
struct Args {
    /// Input GFA or GFA.GZ file
    #[arg(value_name = "GFA")]
    input: PathBuf,

    /// Output JSON instead of human-readable text
    #[arg(long)]
    json: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let stats = compute_graph_stats_from_path(&args.input)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&stats)?);
        return Ok(());
    }

    println!("Graph stats for {}", args.input.display());
    println!("-----------------------------------------");
    println!("Segments (S)        : {}", stats.basic.node_count);
    println!("Edges (L)           : {}", stats.basic.edge_count);
    println!("Other records       : {}", stats.basic.other_records);
    println!();
    println!("Total bp            : {}", stats.basic.total_bp);
    println!("Segment N50         : {}", stats.n50);
    println!("Segment L50         : {}", stats.l50);
    println!("Mean segment length : {:.2}", stats.basic.mean_node_len());
    println!();
    println!("Branching nodes (deg>2): {}", stats.branching_nodes);
    println!("Degree histogram (deg -> count):");
    for (deg, count) in &stats.degree_histogram {
        println!("  {} -> {}", deg, count);
    }

    Ok(())
}
