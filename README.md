# pgtools

A production-ready command-line tool for reading, analyzing, and indexing GFA (Graphical Fragment Assembly) files commonly used in pangenomics.

## Features

- **Statistics**: Compute comprehensive statistics about pangenome graphs
  - Node/edge/path counts
  - Sequence length statistics (total, average, min, max, N50)
  - GC content
  - Connected components analysis
  - Degree distributions

- **Index Building**: Create multiple index types for efficient random access
  - Segment index: Fast lookup by segment name
  - Path index: Fast lookup by path name
  - Position index: Query by genomic coordinates
  - Full index: All index types combined

- **Random Access**: Query indexed graphs efficiently
  - Get segment information
  - Get path information
  - Query by position in a path
  - List all segments and paths

- **Validation**: Validate GFA file integrity
  - Check for undefined segment references
  - Detect missing sequences
  - Report structural issues

- **Progress Indicators**: Visual feedback with progress bars and spinners

## Installation

### From source

```bash
# Clone the repository
git clone https://github.com/manish59/pgtools.git
cd pgtools

# Build with cargo
cargo build --release

# The binary will be at target/release/pgtools
```

## Usage

### Display Statistics

```bash
# Basic statistics
pgtools stats -i input.gfa

# Output as JSON
pgtools stats -i input.gfa -f json

# Save to file
pgtools stats -i input.gfa -o stats.txt
```

### Build Index

```bash
# Build full index (all types)
pgtools index -i input.gfa -o input.idx

# Build specific index type
pgtools index -i input.gfa -o input.idx -t segment
pgtools index -i input.gfa -o input.idx -t path
pgtools index -i input.gfa -o input.idx -t position
```

### Query Index

```bash
# Query segment information
pgtools query -i input.gfa -x input.idx segment -n segment_name

# Query path information
pgtools query -i input.gfa -x input.idx path -n path_name

# Query by position
pgtools query -i input.gfa -x input.idx position -p path_name --pos 1000

# List all segments
pgtools query -i input.gfa -x input.idx list-segments

# List all paths
pgtools query -i input.gfa -x input.idx list-paths
```

### Index Information

```bash
pgtools index-info -i input.idx
```

### Validate GFA File

```bash
# Basic validation
pgtools validate -i input.gfa

# Verbose validation
pgtools validate -i input.gfa -v
```

## GFA Format Support

pgtools supports GFA 1.0 format with the following record types:

- **H**: Header records
- **S**: Segment (node) records
- **L**: Link (edge) records
- **P**: Path records
- **W**: Walk records (GFA 2.0 style)

## Library Usage

pgtools can also be used as a Rust library:

```rust
use pgtools::gfa::GfaGraph;
use pgtools::stats::GfaStats;
use pgtools::index::{GfaIndex, IndexType};

// Parse a GFA file
let graph = GfaGraph::from_file("example.gfa")?;

// Compute statistics
let stats = GfaStats::from_graph(&graph);
println!("{}", stats.format_summary());

// Build and save an index
let index = GfaIndex::build(&graph, "example.gfa", IndexType::Full);
index.save("example.gfa.idx")?;

// Load and query an index
let loaded_index = GfaIndex::load("example.gfa.idx")?;
if let Some(entry) = loaded_index.query_position("path1", 1000) {
    println!("Position 1000 is in segment: {}", entry.segment_name);
}
```

## License

MIT License

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
