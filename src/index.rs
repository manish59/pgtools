//! Index building and management for GFA graphs
//!
//! This module provides different types of indexes for efficient random access
//! to GFA graph data.

use crate::error::{PgToolsError, Result};
use crate::gfa::GfaGraph;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

/// Magic number for index files
const INDEX_MAGIC: u64 = 0x5047544F4F4C5349; // "PGTOOLSI" in hex

/// Index version
const INDEX_VERSION: u32 = 1;

/// Type of index
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexType {
    /// Segment index for fast segment lookup by name
    Segment,
    /// Path index for fast path lookup
    Path,
    /// Position index for genomic coordinate queries
    Position,
    /// Full index containing all index types
    Full,
}

impl std::fmt::Display for IndexType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexType::Segment => write!(f, "segment"),
            IndexType::Path => write!(f, "path"),
            IndexType::Position => write!(f, "position"),
            IndexType::Full => write!(f, "full"),
        }
    }
}

impl std::str::FromStr for IndexType {
    type Err = PgToolsError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "segment" | "seg" | "s" => Ok(IndexType::Segment),
            "path" | "p" => Ok(IndexType::Path),
            "position" | "pos" => Ok(IndexType::Position),
            "full" | "all" | "f" => Ok(IndexType::Full),
            _ => Err(PgToolsError::InvalidInput(format!(
                "Unknown index type: {}. Valid types: segment, path, position, full",
                s
            ))),
        }
    }
}

/// Entry in the segment index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentIndexEntry {
    /// Segment name
    pub name: String,
    /// Sequence length
    pub sequence_length: usize,
    /// Offset in the original GFA file (for random access)
    pub file_offset: u64,
}

/// Entry in the path index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathIndexEntry {
    /// Path name
    pub name: String,
    /// Number of steps
    pub step_count: usize,
    /// Total sequence length
    pub total_length: u64,
    /// File offset
    pub file_offset: u64,
}

/// Entry in the position index for coordinate-based queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionIndexEntry {
    /// Path name
    pub path_name: String,
    /// Start position in the path
    pub start: u64,
    /// End position
    pub end: u64,
    /// Segment name
    pub segment_name: String,
    /// Step index in the path
    pub step_index: usize,
}

/// Segment index for fast segment lookup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentIndex {
    /// Map from segment name to index entry
    pub entries: HashMap<String, SegmentIndexEntry>,
}

/// Path index for fast path lookup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathIndex {
    /// Map from path name to index entry
    pub entries: HashMap<String, PathIndexEntry>,
    /// List of all path names for iteration
    pub path_names: Vec<String>,
}

/// Position index for genomic coordinate queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionIndex {
    /// Map from path name to position entries
    pub entries: HashMap<String, Vec<PositionIndexEntry>>,
}

/// Complete index containing all index types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GfaIndex {
    /// Source GFA file path
    pub source_file: String,
    /// Index version
    pub version: u32,
    /// Segment index
    pub segment_index: Option<SegmentIndex>,
    /// Path index
    pub path_index: Option<PathIndex>,
    /// Position index
    pub position_index: Option<PositionIndex>,
}

impl GfaIndex {
    /// Create a new empty index
    pub fn new(source_file: &str) -> Self {
        Self {
            source_file: source_file.to_string(),
            version: INDEX_VERSION,
            segment_index: None,
            path_index: None,
            position_index: None,
        }
    }

    /// Build index from a GFA graph
    pub fn build(graph: &GfaGraph, source_file: &str, index_type: IndexType) -> Self {
        let mut index = GfaIndex::new(source_file);

        match index_type {
            IndexType::Segment => {
                index.segment_index = Some(Self::build_segment_index(graph));
            }
            IndexType::Path => {
                index.path_index = Some(Self::build_path_index(graph));
            }
            IndexType::Position => {
                index.position_index = Some(Self::build_position_index(graph));
            }
            IndexType::Full => {
                index.segment_index = Some(Self::build_segment_index(graph));
                index.path_index = Some(Self::build_path_index(graph));
                index.position_index = Some(Self::build_position_index(graph));
            }
        }

        index
    }

    fn build_segment_index(graph: &GfaGraph) -> SegmentIndex {
        let entries: HashMap<String, SegmentIndexEntry> = graph
            .segments
            .iter()
            .enumerate()
            .map(|(i, (name, segment))| {
                (
                    name.clone(),
                    SegmentIndexEntry {
                        name: name.clone(),
                        sequence_length: segment.sequence.len(),
                        file_offset: i as u64, // Placeholder, would be actual file offset in production
                    },
                )
            })
            .collect();

        SegmentIndex { entries }
    }

    fn build_path_index(graph: &GfaGraph) -> PathIndex {
        let mut entries = HashMap::new();
        let mut path_names = Vec::new();

        for (i, path) in graph.paths.iter().enumerate() {
            let total_length: u64 = path
                .steps
                .iter()
                .filter_map(|step| graph.segments.get(&step.segment))
                .map(|seg| seg.sequence.len() as u64)
                .sum();

            entries.insert(
                path.name.clone(),
                PathIndexEntry {
                    name: path.name.clone(),
                    step_count: path.steps.len(),
                    total_length,
                    file_offset: i as u64,
                },
            );
            path_names.push(path.name.clone());
        }

        PathIndex {
            entries,
            path_names,
        }
    }

    fn build_position_index(graph: &GfaGraph) -> PositionIndex {
        let mut entries: HashMap<String, Vec<PositionIndexEntry>> = HashMap::new();

        for path in &graph.paths {
            let mut position: u64 = 0;
            let mut path_entries = Vec::new();

            for (step_index, step) in path.steps.iter().enumerate() {
                let seg_len = graph
                    .segments
                    .get(&step.segment)
                    .map(|s| s.sequence.len() as u64)
                    .unwrap_or(0);

                path_entries.push(PositionIndexEntry {
                    path_name: path.name.clone(),
                    start: position,
                    end: position + seg_len,
                    segment_name: step.segment.clone(),
                    step_index,
                });

                position += seg_len;
            }

            entries.insert(path.name.clone(), path_entries);
        }

        PositionIndex { entries }
    }

    /// Save index to a file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // Write magic number and version
        writer.write_u64::<LittleEndian>(INDEX_MAGIC)?;
        writer.write_u32::<LittleEndian>(self.version)?;

        // Serialize the rest with bincode
        let data = bincode::serialize(self)?;
        writer.write_u64::<LittleEndian>(data.len() as u64)?;
        writer.write_all(&data)?;

        Ok(())
    }

    /// Load index from a file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(PgToolsError::FileNotFound(path.display().to_string()));
        }

        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Read and verify magic number
        let magic = reader.read_u64::<LittleEndian>()?;
        if magic != INDEX_MAGIC {
            return Err(PgToolsError::Index("Invalid index file format".to_string()));
        }

        // Read version
        let version = reader.read_u32::<LittleEndian>()?;
        if version > INDEX_VERSION {
            return Err(PgToolsError::Index(format!(
                "Index version {} not supported (max: {})",
                version, INDEX_VERSION
            )));
        }

        // Read data size and deserialize
        let data_len = reader.read_u64::<LittleEndian>()? as usize;
        let mut data = vec![0u8; data_len];
        reader.read_exact(&mut data)?;

        let index: GfaIndex = bincode::deserialize(&data)?;
        Ok(index)
    }

    /// Get segment by name using the index
    pub fn get_segment_info(&self, name: &str) -> Option<&SegmentIndexEntry> {
        self.segment_index.as_ref()?.entries.get(name)
    }

    /// Get path by name using the index
    pub fn get_path_info(&self, name: &str) -> Option<&PathIndexEntry> {
        self.path_index.as_ref()?.entries.get(name)
    }

    /// Query position index to find segment at a given position in a path
    pub fn query_position(&self, path_name: &str, position: u64) -> Option<&PositionIndexEntry> {
        let path_entries = self.position_index.as_ref()?.entries.get(path_name)?;

        // Binary search for the position
        path_entries
            .iter()
            .find(|entry| position >= entry.start && position < entry.end)
    }

    /// List all indexed segments
    pub fn list_segments(&self) -> Vec<&str> {
        self.segment_index
            .as_ref()
            .map(|idx| idx.entries.keys().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// List all indexed paths
    pub fn list_paths(&self) -> Vec<&str> {
        self.path_index
            .as_ref()
            .map(|idx| idx.path_names.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Get index summary
    pub fn summary(&self) -> String {
        let mut output = String::new();
        output.push_str("=== Index Summary ===\n\n");
        output.push_str(&format!("Source file: {}\n", self.source_file));
        output.push_str(&format!("Version: {}\n\n", self.version));

        if let Some(ref seg_idx) = self.segment_index {
            output.push_str(&format!(
                "Segment index: {} entries\n",
                seg_idx.entries.len()
            ));
        } else {
            output.push_str("Segment index: not built\n");
        }

        if let Some(ref path_idx) = self.path_index {
            output.push_str(&format!("Path index: {} entries\n", path_idx.entries.len()));
        } else {
            output.push_str("Path index: not built\n");
        }

        if let Some(ref pos_idx) = self.position_index {
            let total_entries: usize = pos_idx.entries.values().map(|v| v.len()).sum();
            output.push_str(&format!(
                "Position index: {} entries across {} paths\n",
                total_entries,
                pos_idx.entries.len()
            ));
        } else {
            output.push_str("Position index: not built\n");
        }

        output
    }
}

/// Random access reader that uses an index
pub struct IndexedReader {
    /// The index
    pub index: GfaIndex,
    /// Source GFA file path
    #[allow(dead_code)]
    source_path: std::path::PathBuf,
}

impl IndexedReader {
    /// Create a new indexed reader
    pub fn new<P: AsRef<Path>>(gfa_path: P, index_path: P) -> Result<Self> {
        let gfa_path = gfa_path.as_ref();
        let index = GfaIndex::load(index_path)?;

        if !gfa_path.exists() {
            return Err(PgToolsError::FileNotFound(gfa_path.display().to_string()));
        }

        Ok(Self {
            index,
            source_path: gfa_path.to_path_buf(),
        })
    }

    /// Get segment info by name
    pub fn get_segment(&self, name: &str) -> Option<&SegmentIndexEntry> {
        self.index.get_segment_info(name)
    }

    /// Get path info by name
    pub fn get_path(&self, name: &str) -> Option<&PathIndexEntry> {
        self.index.get_path_info(name)
    }

    /// Query for segment at position
    pub fn query_position(&self, path: &str, position: u64) -> Option<&PositionIndexEntry> {
        self.index.query_position(path, position)
    }

    /// List all segments
    pub fn list_segments(&self) -> Vec<&str> {
        self.index.list_segments()
    }

    /// List all paths
    pub fn list_paths(&self) -> Vec<&str> {
        self.index.list_paths()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tempfile::tempdir;

    fn create_test_graph() -> GfaGraph {
        let gfa_content = "H\tVN:Z:1.0\n\
                          S\ts1\tACGTACGT\n\
                          S\ts2\tGGGGGGGG\n\
                          S\ts3\tTTTTTTTT\n\
                          L\ts1\t+\ts2\t+\t0M\n\
                          L\ts2\t+\ts3\t+\t0M\n\
                          P\tpath1\ts1+,s2+,s3+\t*\n";
        let cursor = Cursor::new(gfa_content);
        GfaGraph::parse(cursor).unwrap()
    }

    #[test]
    fn test_build_segment_index() {
        let graph = create_test_graph();
        let index = GfaIndex::build(&graph, "test.gfa", IndexType::Segment);

        assert!(index.segment_index.is_some());
        let seg_idx = index.segment_index.unwrap();
        assert_eq!(seg_idx.entries.len(), 3);
        assert!(seg_idx.entries.contains_key("s1"));
    }

    #[test]
    fn test_build_path_index() {
        let graph = create_test_graph();
        let index = GfaIndex::build(&graph, "test.gfa", IndexType::Path);

        assert!(index.path_index.is_some());
        let path_idx = index.path_index.unwrap();
        assert_eq!(path_idx.entries.len(), 1);
        assert!(path_idx.entries.contains_key("path1"));
        assert_eq!(path_idx.entries["path1"].total_length, 24);
    }

    #[test]
    fn test_build_position_index() {
        let graph = create_test_graph();
        let index = GfaIndex::build(&graph, "test.gfa", IndexType::Position);

        assert!(index.position_index.is_some());
        let pos_idx = index.position_index.unwrap();

        let path1_entries = &pos_idx.entries["path1"];
        assert_eq!(path1_entries.len(), 3);

        // Check positions
        assert_eq!(path1_entries[0].start, 0);
        assert_eq!(path1_entries[0].end, 8);
        assert_eq!(path1_entries[1].start, 8);
        assert_eq!(path1_entries[1].end, 16);
    }

    #[test]
    fn test_query_position() {
        let graph = create_test_graph();
        let index = GfaIndex::build(&graph, "test.gfa", IndexType::Full);

        // Query position 5 - should be in s1
        let entry = index.query_position("path1", 5).unwrap();
        assert_eq!(entry.segment_name, "s1");

        // Query position 10 - should be in s2
        let entry = index.query_position("path1", 10).unwrap();
        assert_eq!(entry.segment_name, "s2");

        // Query position 20 - should be in s3
        let entry = index.query_position("path1", 20).unwrap();
        assert_eq!(entry.segment_name, "s3");
    }

    #[test]
    fn test_save_load_index() {
        let graph = create_test_graph();
        let index = GfaIndex::build(&graph, "test.gfa", IndexType::Full);

        let dir = tempdir().unwrap();
        let index_path = dir.path().join("test.idx");

        // Save index
        index.save(&index_path).unwrap();

        // Load index
        let loaded = GfaIndex::load(&index_path).unwrap();

        assert_eq!(loaded.source_file, "test.gfa");
        assert!(loaded.segment_index.is_some());
        assert!(loaded.path_index.is_some());
        assert!(loaded.position_index.is_some());
    }

    #[test]
    fn test_index_type_parsing() {
        assert_eq!("segment".parse::<IndexType>().unwrap(), IndexType::Segment);
        assert_eq!("path".parse::<IndexType>().unwrap(), IndexType::Path);
        assert_eq!(
            "position".parse::<IndexType>().unwrap(),
            IndexType::Position
        );
        assert_eq!("full".parse::<IndexType>().unwrap(), IndexType::Full);
    }
}
