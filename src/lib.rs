//! pgtools - PanGenome Tools
//!
//! A library for reading, analyzing, and indexing GFA (Graphical Fragment Assembly) files
//! commonly used in pangenomics.
//!
//! # Features
//!
//! - Parse GFA format files (GFA 1.0 and partial GFA 2.0 support)
//! - Compute comprehensive statistics about pangenome graphs
//! - Build multiple index types for efficient random access
//! - Query indexed graphs by segment name, path name, or genomic position
//!
//! # Example
//!
//! ```no_run
//! use pgtools::gfa::GfaGraph;
//! use pgtools::stats::GfaStats;
//! use pgtools::index::{GfaIndex, IndexType};
//!
//! // Parse a GFA file
//! let graph = GfaGraph::from_file("example.gfa").unwrap();
//!
//! // Compute statistics
//! let stats = GfaStats::from_graph(&graph);
//! println!("{}", stats.format_summary());
//!
//! // Build an index
//! let index = GfaIndex::build(&graph, "example.gfa", IndexType::Full);
//! index.save("example.gfa.idx").unwrap();
//! ```

pub mod cli;
pub mod error;
pub mod gfa;
pub mod index;
pub mod stats;

pub use error::{PgToolsError, Result};
pub use gfa::GfaGraph;
pub use index::{GfaIndex, IndexType, IndexedReader};
pub use stats::GfaStats;
