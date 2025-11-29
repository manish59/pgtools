use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use pgtools::{compute_basic_stats_from_path, compute_basic_stats_from_path_with_progress};
use serde_json;

/// Compute basic streaming statistics for a GFA or GFA.GZ file.
#[derive(Debug, Parser)]
#[command(name = "pgtools-stats-basic", version, about)]
struct Args {
    /// Input GFA or GFA.GZ file
    #[arg(value_name = "GFA_PATH")]
    input: PathBuf,

    /// Disable progress bar
    #[arg(long)]
    no_progress: bool,

    /// Output JSON instead of human-readable text
    #[arg(long)]
    json: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let stats = if args.no_progress {
        compute_basic_stats_from_path(&args.input)?
    } else {
        compute_basic_stats_from_path_with_progress(&args.input)?
    };

    if args.json {
        let json = serde_json::json!({
            "total_lines": stats.total_lines,
            "nodes": {
                "count": stats.node_count,
                "total_bp": stats.total_bp,
                "min_length": stats.min_node_len,
                "max_length": stats.max_node_len,
                "mean_length": stats.mean_node_len(),
            },
            "edges": { "count": stats.edge_count },
            "paths": { "count": stats.path_count },
            "bases": {
                "gc": stats.gc_bases,
                "n": stats.n_bases
            },
            "other_records": stats.other_records,
            "comment_lines": stats.comment_lines
        });

        println!("{}", serde_json::to_string_pretty(&json)?);
        return Ok(());
    }

    println!("Basic stats for {}", args.input.display());
    println!("-----------------------------------------");
    println!("Total lines        : {}", stats.total_lines);
    println!("Nodes (S)          : {}", stats.node_count);
    println!("Edges (L)          : {}", stats.edge_count);
    println!("Paths (P)          : {}", stats.path_count);
    println!("Other records      : {}", stats.other_records);
    println!("Comment lines (#)  : {}", stats.comment_lines);
    println!();
    println!("Total bp           : {}", stats.total_bp);
    println!("Min node length    : {}", stats.min_node_len);
    println!("Max node length    : {}", stats.max_node_len);
    println!("Mean node length   : {:.2}", stats.mean_node_len());
    println!();
    println!("GC bases           : {}", stats.gc_bases);
    println!("N bases            : {}", stats.n_bases);

    Ok(())
}
