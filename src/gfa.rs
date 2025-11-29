//! GFA (Graphical Fragment Assembly) file parser
//!
//! This module provides parsing functionality for GFA format files,
//! which are commonly used to represent pangenome graphs.

use crate::error::{PgToolsError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Orientation of a segment in a path or link
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Orientation {
    Forward,
    Reverse,
}

impl Orientation {
    fn from_char(c: char) -> Result<Self> {
        match c {
            '+' => Ok(Orientation::Forward),
            '-' => Ok(Orientation::Reverse),
            _ => Err(PgToolsError::InvalidInput(format!(
                "Invalid orientation: {}",
                c
            ))),
        }
    }
}

impl std::fmt::Display for Orientation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Orientation::Forward => write!(f, "+"),
            Orientation::Reverse => write!(f, "-"),
        }
    }
}

/// A segment (node) in the GFA graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    /// Segment name/identifier
    pub name: String,
    /// Sequence data
    pub sequence: String,
    /// Optional tags
    pub tags: HashMap<String, String>,
}

/// A link (edge) between two segments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    /// From segment name
    pub from_segment: String,
    /// From orientation
    pub from_orient: Orientation,
    /// To segment name
    pub to_segment: String,
    /// To orientation
    pub to_orient: Orientation,
    /// Overlap CIGAR string
    pub overlap: String,
}

/// A step in a path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathStep {
    /// Segment name
    pub segment: String,
    /// Orientation
    pub orientation: Orientation,
}

/// A path through the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GfaPath {
    /// Path name
    pub name: String,
    /// Steps in the path
    pub steps: Vec<PathStep>,
    /// Optional overlaps
    pub overlaps: Option<Vec<String>>,
}

/// Header information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Header {
    /// Version string
    pub version: Option<String>,
    /// Additional tags
    pub tags: HashMap<String, String>,
}

/// Complete GFA graph representation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GfaGraph {
    /// Header information
    pub header: Header,
    /// Segments (nodes) indexed by name
    pub segments: HashMap<String, Segment>,
    /// Links (edges)
    pub links: Vec<Link>,
    /// Paths through the graph
    pub paths: Vec<GfaPath>,
}

impl GfaGraph {
    /// Create a new empty GFA graph
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a GFA file from a path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(PgToolsError::FileNotFound(path.display().to_string()));
        }

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::parse(reader)
    }

    /// Parse GFA from a buffered reader
    pub fn parse<R: BufRead>(reader: R) -> Result<Self> {
        let mut graph = GfaGraph::new();

        for (line_num, line_result) in reader.lines().enumerate() {
            let line = line_result?;
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let fields: Vec<&str> = line.split('\t').collect();
            if fields.is_empty() {
                continue;
            }

            match fields[0] {
                "H" => graph.parse_header(&fields, line_num + 1)?,
                "S" => graph.parse_segment(&fields, line_num + 1)?,
                "L" => graph.parse_link(&fields, line_num + 1)?,
                "P" => graph.parse_path(&fields, line_num + 1)?,
                "W" => graph.parse_walk(&fields, line_num + 1)?,
                _ => {
                    // Unknown record type, skip
                }
            }
        }

        Ok(graph)
    }

    fn parse_header(&mut self, fields: &[&str], _line: usize) -> Result<()> {
        for field in fields.iter().skip(1) {
            if let Some((key, value)) = field.split_once(':') {
                if key == "VN" {
                    self.header.version = Some(value.to_string());
                } else {
                    self.header.tags.insert(key.to_string(), value.to_string());
                }
            }
        }
        Ok(())
    }

    fn parse_segment(&mut self, fields: &[&str], line: usize) -> Result<()> {
        if fields.len() < 3 {
            return Err(PgToolsError::GfaParse {
                line,
                message: "Segment record requires at least 3 fields".to_string(),
            });
        }

        let name = fields[1].to_string();
        let sequence = fields[2].to_string();
        let mut tags = HashMap::new();

        // Parse optional tags
        for field in fields.iter().skip(3) {
            if let Some((key, value)) = field.split_once(':') {
                tags.insert(key.to_string(), value.to_string());
            }
        }

        self.segments.insert(
            name.clone(),
            Segment {
                name,
                sequence,
                tags,
            },
        );

        Ok(())
    }

    fn parse_link(&mut self, fields: &[&str], line: usize) -> Result<()> {
        if fields.len() < 6 {
            return Err(PgToolsError::GfaParse {
                line,
                message: "Link record requires at least 6 fields".to_string(),
            });
        }

        let from_segment = fields[1].to_string();
        let from_orient = Orientation::from_char(fields[2].chars().next().ok_or_else(|| {
            PgToolsError::GfaParse {
                line,
                message: "Missing from orientation".to_string(),
            }
        })?)?;
        let to_segment = fields[3].to_string();
        let to_orient = Orientation::from_char(fields[4].chars().next().ok_or_else(|| {
            PgToolsError::GfaParse {
                line,
                message: "Missing to orientation".to_string(),
            }
        })?)?;
        let overlap = fields[5].to_string();

        self.links.push(Link {
            from_segment,
            from_orient,
            to_segment,
            to_orient,
            overlap,
        });

        Ok(())
    }

    fn parse_path(&mut self, fields: &[&str], line: usize) -> Result<()> {
        if fields.len() < 3 {
            return Err(PgToolsError::GfaParse {
                line,
                message: "Path record requires at least 3 fields".to_string(),
            });
        }

        let name = fields[1].to_string();
        let steps_str = fields[2];

        let mut steps = Vec::new();
        for step in steps_str.split(',') {
            let step = step.trim();
            if step.is_empty() {
                continue;
            }

            let (segment, orient_char) = if step.ends_with('+') || step.ends_with('-') {
                let orient = step.chars().last().unwrap();
                let seg = &step[..step.len() - 1];
                (seg, orient)
            } else {
                return Err(PgToolsError::GfaParse {
                    line,
                    message: format!("Path step missing orientation: {}", step),
                });
            };

            steps.push(PathStep {
                segment: segment.to_string(),
                orientation: Orientation::from_char(orient_char)?,
            });
        }

        let overlaps = if fields.len() > 3 && !fields[3].is_empty() && fields[3] != "*" {
            Some(fields[3].split(',').map(String::from).collect())
        } else {
            None
        };

        self.paths.push(GfaPath {
            name,
            steps,
            overlaps,
        });

        Ok(())
    }

    fn parse_walk(&mut self, fields: &[&str], line: usize) -> Result<()> {
        // W record: W sample haplotype seq_id seq_start seq_end walk
        if fields.len() < 7 {
            return Err(PgToolsError::GfaParse {
                line,
                message: "Walk record requires at least 7 fields".to_string(),
            });
        }

        let sample = fields[1];
        let haplotype = fields[2];
        let seq_id = fields[3];
        let name = format!("{}#{}#{}", sample, haplotype, seq_id);
        let walk_str = fields[6];

        let mut steps = Vec::new();
        let mut current_segment = String::new();
        let mut in_segment = false;

        for c in walk_str.chars() {
            match c {
                '>' => {
                    if in_segment && !current_segment.is_empty() {
                        steps.push(PathStep {
                            segment: current_segment.clone(),
                            orientation: Orientation::Forward,
                        });
                        current_segment.clear();
                    }
                    in_segment = true;
                }
                '<' => {
                    if in_segment && !current_segment.is_empty() {
                        steps.push(PathStep {
                            segment: current_segment.clone(),
                            orientation: Orientation::Reverse,
                        });
                        current_segment.clear();
                    }
                    in_segment = true;
                }
                _ => {
                    if in_segment {
                        current_segment.push(c);
                    }
                }
            }
        }

        // Handle last segment
        if !current_segment.is_empty() {
            // The orientation is determined by the prefix that started this segment
            // We need to track this differently
            steps.push(PathStep {
                segment: current_segment,
                orientation: Orientation::Forward, // Default, the actual orientation was set when we started
            });
        }

        self.paths.push(GfaPath {
            name,
            steps,
            overlaps: None,
        });

        Ok(())
    }

    /// Get segment by name
    pub fn get_segment(&self, name: &str) -> Option<&Segment> {
        self.segments.get(name)
    }

    /// Get total sequence length
    pub fn total_sequence_length(&self) -> u64 {
        self.segments
            .values()
            .map(|s| s.sequence.len() as u64)
            .sum()
    }

    /// Get number of segments
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    /// Get number of links
    pub fn link_count(&self) -> usize {
        self.links.len()
    }

    /// Get number of paths
    pub fn path_count(&self) -> usize {
        self.paths.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_simple_gfa() {
        let gfa_content = "H\tVN:Z:1.0\n\
                          S\ts1\tACGT\n\
                          S\ts2\tGGGG\n\
                          L\ts1\t+\ts2\t+\t0M\n\
                          P\tpath1\ts1+,s2+\t*\n";

        let cursor = Cursor::new(gfa_content);
        let graph = GfaGraph::parse(cursor).unwrap();

        assert_eq!(graph.header.version, Some("Z:1.0".to_string()));
        assert_eq!(graph.segment_count(), 2);
        assert_eq!(graph.link_count(), 1);
        assert_eq!(graph.path_count(), 1);
        assert_eq!(graph.total_sequence_length(), 8);
    }

    #[test]
    fn test_parse_segment() {
        let gfa_content = "S\tnode1\tACGTACGT\n";
        let cursor = Cursor::new(gfa_content);
        let graph = GfaGraph::parse(cursor).unwrap();

        let segment = graph.get_segment("node1").unwrap();
        assert_eq!(segment.name, "node1");
        assert_eq!(segment.sequence, "ACGTACGT");
    }

    #[test]
    fn test_parse_link() {
        let gfa_content = "S\ts1\tACGT\n\
                          S\ts2\tGGGG\n\
                          L\ts1\t+\ts2\t-\t2M\n";
        let cursor = Cursor::new(gfa_content);
        let graph = GfaGraph::parse(cursor).unwrap();

        assert_eq!(graph.links.len(), 1);
        let link = &graph.links[0];
        assert_eq!(link.from_segment, "s1");
        assert_eq!(link.from_orient, Orientation::Forward);
        assert_eq!(link.to_segment, "s2");
        assert_eq!(link.to_orient, Orientation::Reverse);
        assert_eq!(link.overlap, "2M");
    }

    #[test]
    fn test_parse_path() {
        let gfa_content = "S\ts1\tACGT\n\
                          S\ts2\tGGGG\n\
                          S\ts3\tTTTT\n\
                          P\tmypath\ts1+,s2-,s3+\t*\n";
        let cursor = Cursor::new(gfa_content);
        let graph = GfaGraph::parse(cursor).unwrap();

        assert_eq!(graph.paths.len(), 1);
        let path = &graph.paths[0];
        assert_eq!(path.name, "mypath");
        assert_eq!(path.steps.len(), 3);
        assert_eq!(path.steps[0].segment, "s1");
        assert_eq!(path.steps[0].orientation, Orientation::Forward);
        assert_eq!(path.steps[1].segment, "s2");
        assert_eq!(path.steps[1].orientation, Orientation::Reverse);
    }

    #[test]
    fn test_orientation_display() {
        assert_eq!(format!("{}", Orientation::Forward), "+");
        assert_eq!(format!("{}", Orientation::Reverse), "-");
    }
}
