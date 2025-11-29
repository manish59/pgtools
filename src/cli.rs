//! Command-line interface for pgtools

use crate::error::Result;
use crate::gfa::GfaGraph;
use crate::index::{GfaIndex, IndexType, IndexedReader};
use crate::stats::GfaStats;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::time::Instant;

/// pgtools - PanGenome Tools for reading and indexing GFA files
#[derive(Parser)]
#[command(name = "pgtools")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Commands,
}

/// Available commands
#[derive(Subcommand)]
pub enum Commands {
    /// Display statistics about a GFA file
    Stats {
        /// Path to the GFA file
        #[arg(short, long)]
        input: PathBuf,

        /// Output format (text or json)
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Build an index for a GFA file
    Index {
        /// Path to the GFA file
        #[arg(short, long)]
        input: PathBuf,

        /// Output index file path
        #[arg(short, long)]
        output: PathBuf,

        /// Type of index to build (segment, path, position, full)
        #[arg(short = 't', long, default_value = "full")]
        index_type: String,
    },

    /// Query an indexed GFA file
    Query {
        /// Path to the GFA file
        #[arg(short, long)]
        input: PathBuf,

        /// Path to the index file
        #[arg(short = 'x', long)]
        index: PathBuf,

        /// Query subcommand
        #[command(subcommand)]
        query: QueryCommands,
    },

    /// Show information about an index file
    IndexInfo {
        /// Path to the index file
        #[arg(short, long)]
        index: PathBuf,
    },

    /// Validate a GFA file
    Validate {
        /// Path to the GFA file
        #[arg(short, long)]
        input: PathBuf,

        /// Show detailed validation messages
        #[arg(short, long)]
        verbose: bool,
    },
}

/// Query subcommands
#[derive(Subcommand)]
pub enum QueryCommands {
    /// Get segment information
    Segment {
        /// Segment name
        #[arg(short, long)]
        name: String,
    },

    /// Get path information
    Path {
        /// Path name
        #[arg(short, long)]
        name: String,
    },

    /// Query by position
    Position {
        /// Path name
        #[arg(short, long)]
        path: String,

        /// Position in the path
        #[arg(long)]
        pos: u64,
    },

    /// List all segments
    ListSegments,

    /// List all paths
    ListPaths,
}

/// Run the CLI application
pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Stats {
            input,
            format,
            output,
        } => cmd_stats(&input, &format, output.as_deref()),
        Commands::Index {
            input,
            output,
            index_type,
        } => cmd_index(&input, &output, &index_type),
        Commands::Query {
            input,
            index,
            query,
        } => cmd_query(&input, &index, query),
        Commands::IndexInfo { index } => cmd_index_info(&index),
        Commands::Validate { input, verbose } => cmd_validate(&input, verbose),
    }
}

fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}

fn cmd_stats(input: &PathBuf, format: &str, output: Option<&std::path::Path>) -> Result<()> {
    let spinner = create_spinner("Reading GFA file...");
    let start = Instant::now();

    let graph = GfaGraph::from_file(input)?;
    spinner.set_message("Computing statistics...");

    let stats = GfaStats::from_graph(&graph);
    spinner.finish_with_message(format!("Done in {:.2?}", start.elapsed()));

    let output_text = match format.to_lowercase().as_str() {
        "json" => stats.to_json()?,
        _ => stats.format_summary(),
    };

    if let Some(output_path) = output {
        std::fs::write(output_path, &output_text)?;
        println!("Statistics written to: {}", output_path.display());
    } else {
        println!("{}", output_text);
    }

    Ok(())
}

fn cmd_index(input: &PathBuf, output: &PathBuf, index_type: &str) -> Result<()> {
    let idx_type: IndexType = index_type.parse()?;

    let spinner = create_spinner("Reading GFA file...");
    let start = Instant::now();

    let graph = GfaGraph::from_file(input)?;
    spinner.set_message(format!("Building {} index...", idx_type));

    let index = GfaIndex::build(&graph, &input.display().to_string(), idx_type);
    spinner.set_message("Saving index...");

    index.save(output)?;
    spinner.finish_with_message(format!("Index built and saved in {:.2?}", start.elapsed()));

    println!("\n{}", index.summary());
    println!("Index saved to: {}", output.display());

    Ok(())
}

fn cmd_query(input: &PathBuf, index_path: &PathBuf, query: QueryCommands) -> Result<()> {
    let reader = IndexedReader::new(input, index_path)?;

    match query {
        QueryCommands::Segment { name } => {
            if let Some(segment) = reader.get_segment(&name) {
                println!("Segment: {}", segment.name);
                println!("  Sequence length: {} bp", segment.sequence_length);
                println!("  File offset: {}", segment.file_offset);
            } else {
                println!("Segment '{}' not found in index", name);
            }
        }
        QueryCommands::Path { name } => {
            if let Some(path) = reader.get_path(&name) {
                println!("Path: {}", path.name);
                println!("  Steps: {}", path.step_count);
                println!("  Total length: {} bp", path.total_length);
            } else {
                println!("Path '{}' not found in index", name);
            }
        }
        QueryCommands::Position { path, pos } => {
            if let Some(entry) = reader.query_position(&path, pos) {
                println!("Position {} in path '{}':", pos, path);
                println!("  Segment: {}", entry.segment_name);
                println!("  Segment range: {} - {}", entry.start, entry.end);
                println!("  Step index: {}", entry.step_index);
            } else {
                println!("Position {} not found in path '{}'", pos, path);
            }
        }
        QueryCommands::ListSegments => {
            let segments = reader.list_segments();
            println!("Indexed segments ({}):", segments.len());
            for seg in segments {
                println!("  {}", seg);
            }
        }
        QueryCommands::ListPaths => {
            let paths = reader.list_paths();
            println!("Indexed paths ({}):", paths.len());
            for path in paths {
                println!("  {}", path);
            }
        }
    }

    Ok(())
}

fn cmd_index_info(index_path: &PathBuf) -> Result<()> {
    let index = GfaIndex::load(index_path)?;
    println!("{}", index.summary());
    Ok(())
}

fn cmd_validate(input: &PathBuf, verbose: bool) -> Result<()> {
    let spinner = create_spinner("Validating GFA file...");
    let start = Instant::now();

    let graph = GfaGraph::from_file(input)?;
    spinner.finish_with_message(format!("File parsed in {:.2?}", start.elapsed()));

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Check for segments referenced in links but not defined
    for link in &graph.links {
        if !graph.segments.contains_key(&link.from_segment) {
            errors.push(format!(
                "Link references undefined segment: {}",
                link.from_segment
            ));
        }
        if !graph.segments.contains_key(&link.to_segment) {
            errors.push(format!(
                "Link references undefined segment: {}",
                link.to_segment
            ));
        }
    }

    // Check for segments referenced in paths but not defined
    for path in &graph.paths {
        for step in &path.steps {
            if !graph.segments.contains_key(&step.segment) {
                errors.push(format!(
                    "Path '{}' references undefined segment: {}",
                    path.name, step.segment
                ));
            }
        }
    }

    // Check for empty sequences
    for (name, segment) in &graph.segments {
        if segment.sequence.is_empty() || segment.sequence == "*" {
            warnings.push(format!("Segment '{}' has empty/placeholder sequence", name));
        }
    }

    // Print results
    println!("\n=== Validation Results ===\n");
    println!("Segments: {}", graph.segment_count());
    println!("Links: {}", graph.link_count());
    println!("Paths: {}", graph.path_count());
    println!();

    if errors.is_empty() && warnings.is_empty() {
        println!("✓ No issues found");
    } else {
        if !errors.is_empty() {
            println!("Errors ({}):", errors.len());
            if verbose {
                for err in &errors {
                    println!("  ✗ {}", err);
                }
            } else {
                for err in errors.iter().take(5) {
                    println!("  ✗ {}", err);
                }
                if errors.len() > 5 {
                    println!("  ... and {} more errors", errors.len() - 5);
                }
            }
            println!();
        }

        if !warnings.is_empty() {
            println!("Warnings ({}):", warnings.len());
            if verbose {
                for warn in &warnings {
                    println!("  ⚠ {}", warn);
                }
            } else {
                for warn in warnings.iter().take(5) {
                    println!("  ⚠ {}", warn);
                }
                if warnings.len() > 5 {
                    println!("  ... and {} more warnings", warnings.len() - 5);
                }
            }
        }
    }

    if errors.is_empty() {
        println!("\n✓ Validation passed");
        Ok(())
    } else {
        println!("\n✗ Validation failed with {} errors", errors.len());
        Ok(()) // Return Ok to not exit with error code, just report issues
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_stats() {
        let cli = Cli::try_parse_from(["pgtools", "stats", "-i", "test.gfa"]).unwrap();
        match cli.command {
            Commands::Stats { input, .. } => {
                assert_eq!(input, PathBuf::from("test.gfa"));
            }
            _ => panic!("Expected Stats command"),
        }
    }

    #[test]
    fn test_cli_parse_index() {
        let cli = Cli::try_parse_from([
            "pgtools", "index", "-i", "test.gfa", "-o", "test.idx", "-t", "full",
        ])
        .unwrap();
        match cli.command {
            Commands::Index {
                input,
                output,
                index_type,
            } => {
                assert_eq!(input, PathBuf::from("test.gfa"));
                assert_eq!(output, PathBuf::from("test.idx"));
                assert_eq!(index_type, "full");
            }
            _ => panic!("Expected Index command"),
        }
    }
}
