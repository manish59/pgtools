//! pgtools - PanGenome Tools CLI
//!
//! A command-line tool for reading, analyzing, and indexing GFA files.

use pgtools::cli;

fn main() {
    if let Err(e) = cli::run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
