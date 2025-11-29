//! Statistics computation for GFA graphs

use crate::gfa::GfaGraph;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Statistics about a GFA graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GfaStats {
    /// Total number of segments (nodes)
    pub segment_count: usize,
    /// Total number of links (edges)
    pub link_count: usize,
    /// Total number of paths
    pub path_count: usize,
    /// Total sequence length across all segments
    pub total_sequence_length: u64,
    /// Average segment length
    pub average_segment_length: f64,
    /// Minimum segment length
    pub min_segment_length: usize,
    /// Maximum segment length
    pub max_segment_length: usize,
    /// N50 of segment lengths
    pub n50: usize,
    /// GC content percentage
    pub gc_content: f64,
    /// Number of connected components
    pub connected_components: usize,
    /// Average path length (in segments)
    pub average_path_length: f64,
    /// Total path length (sum of sequence lengths)
    pub total_path_sequence_length: u64,
    /// Segment length histogram (binned)
    pub segment_length_histogram: Vec<(String, usize)>,
    /// In-degree distribution
    pub in_degree_distribution: HashMap<usize, usize>,
    /// Out-degree distribution
    pub out_degree_distribution: HashMap<usize, usize>,
}

impl GfaStats {
    /// Compute statistics from a GFA graph
    pub fn from_graph(graph: &GfaGraph) -> Self {
        let segment_count = graph.segment_count();
        let link_count = graph.link_count();
        let path_count = graph.path_count();
        let total_sequence_length = graph.total_sequence_length();

        // Compute segment length statistics
        let segment_lengths: Vec<usize> =
            graph.segments.values().map(|s| s.sequence.len()).collect();

        let (min_segment_length, max_segment_length, average_segment_length) = if segment_lengths
            .is_empty()
        {
            (0, 0, 0.0)
        } else {
            let min = *segment_lengths.iter().min().unwrap();
            let max = *segment_lengths.iter().max().unwrap();
            let avg = segment_lengths.iter().sum::<usize>() as f64 / segment_lengths.len() as f64;
            (min, max, avg)
        };

        // Compute N50
        let n50 = compute_n50(&segment_lengths);

        // Compute GC content
        let gc_content = compute_gc_content(graph);

        // Compute connected components
        let connected_components = compute_connected_components(graph);

        // Compute path statistics
        let (average_path_length, total_path_sequence_length) = compute_path_stats(graph);

        // Compute segment length histogram
        let segment_length_histogram = compute_length_histogram(&segment_lengths);

        // Compute degree distributions
        let (in_degree_distribution, out_degree_distribution) = compute_degree_distributions(graph);

        GfaStats {
            segment_count,
            link_count,
            path_count,
            total_sequence_length,
            average_segment_length,
            min_segment_length,
            max_segment_length,
            n50,
            gc_content,
            connected_components,
            average_path_length,
            total_path_sequence_length,
            segment_length_histogram,
            in_degree_distribution,
            out_degree_distribution,
        }
    }

    /// Format statistics as a human-readable string
    pub fn format_summary(&self) -> String {
        let mut output = String::new();
        output.push_str("=== GFA Graph Statistics ===\n\n");

        output.push_str(&format!(
            "Segments (nodes):        {:>12}\n",
            self.segment_count
        ));
        output.push_str(&format!(
            "Links (edges):           {:>12}\n",
            self.link_count
        ));
        output.push_str(&format!(
            "Paths:                   {:>12}\n",
            self.path_count
        ));
        output.push_str(&format!(
            "Connected components:    {:>12}\n",
            self.connected_components
        ));
        output.push('\n');

        output.push_str("--- Sequence Statistics ---\n");
        output.push_str(&format!(
            "Total sequence length:   {:>12} bp\n",
            self.total_sequence_length
        ));
        output.push_str(&format!(
            "Average segment length:  {:>12.2} bp\n",
            self.average_segment_length
        ));
        output.push_str(&format!(
            "Min segment length:      {:>12} bp\n",
            self.min_segment_length
        ));
        output.push_str(&format!(
            "Max segment length:      {:>12} bp\n",
            self.max_segment_length
        ));
        output.push_str(&format!("N50:                     {:>12} bp\n", self.n50));
        output.push_str(&format!(
            "GC content:              {:>12.2}%\n",
            self.gc_content
        ));
        output.push('\n');

        if self.path_count > 0 {
            output.push_str("--- Path Statistics ---\n");
            output.push_str(&format!(
                "Average path length:     {:>12.2} segments\n",
                self.average_path_length
            ));
            output.push_str(&format!(
                "Total path seq length:   {:>12} bp\n",
                self.total_path_sequence_length
            ));
            output.push('\n');
        }

        output.push_str("--- Segment Length Distribution ---\n");
        for (bin, count) in &self.segment_length_histogram {
            if *count > 0 {
                output.push_str(&format!("{:>15}: {:>8}\n", bin, count));
            }
        }

        output
    }

    /// Export statistics as JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

fn compute_n50(lengths: &[usize]) -> usize {
    if lengths.is_empty() {
        return 0;
    }

    let mut sorted: Vec<usize> = lengths.to_vec();
    sorted.sort_unstable_by(|a, b| b.cmp(a)); // Sort descending

    let total: usize = sorted.iter().sum();
    let half = total / 2;

    let mut cumsum = 0;
    for len in sorted {
        cumsum += len;
        if cumsum >= half {
            return len;
        }
    }

    0
}

fn compute_gc_content(graph: &GfaGraph) -> f64 {
    let mut gc_count: u64 = 0;
    let mut total_count: u64 = 0;

    for segment in graph.segments.values() {
        for c in segment.sequence.chars() {
            match c.to_ascii_uppercase() {
                'G' | 'C' => {
                    gc_count += 1;
                    total_count += 1;
                }
                'A' | 'T' => {
                    total_count += 1;
                }
                _ => {} // Skip N and other characters
            }
        }
    }

    if total_count == 0 {
        0.0
    } else {
        (gc_count as f64 / total_count as f64) * 100.0
    }
}

fn compute_connected_components(graph: &GfaGraph) -> usize {
    if graph.segments.is_empty() {
        return 0;
    }

    // Build adjacency list
    let mut adjacency: HashMap<&str, HashSet<&str>> = HashMap::new();
    for segment in graph.segments.keys() {
        adjacency.insert(segment.as_str(), HashSet::new());
    }

    for link in &graph.links {
        if let Some(neighbors) = adjacency.get_mut(link.from_segment.as_str()) {
            neighbors.insert(link.to_segment.as_str());
        }
        if let Some(neighbors) = adjacency.get_mut(link.to_segment.as_str()) {
            neighbors.insert(link.from_segment.as_str());
        }
    }

    // Count components using DFS
    let mut visited: HashSet<&str> = HashSet::new();
    let mut components = 0;

    for segment in graph.segments.keys() {
        if !visited.contains(segment.as_str()) {
            dfs(segment.as_str(), &adjacency, &mut visited);
            components += 1;
        }
    }

    components
}

fn dfs<'a>(
    node: &'a str,
    adjacency: &HashMap<&'a str, HashSet<&'a str>>,
    visited: &mut HashSet<&'a str>,
) {
    visited.insert(node);
    if let Some(neighbors) = adjacency.get(node) {
        for neighbor in neighbors {
            if !visited.contains(neighbor) {
                dfs(neighbor, adjacency, visited);
            }
        }
    }
}

fn compute_path_stats(graph: &GfaGraph) -> (f64, u64) {
    if graph.paths.is_empty() {
        return (0.0, 0);
    }

    let path_lengths: Vec<usize> = graph.paths.iter().map(|p| p.steps.len()).collect();
    let average = path_lengths.iter().sum::<usize>() as f64 / path_lengths.len() as f64;

    let mut total_seq_len: u64 = 0;
    for path in &graph.paths {
        for step in &path.steps {
            if let Some(segment) = graph.segments.get(&step.segment) {
                total_seq_len += segment.sequence.len() as u64;
            }
        }
    }

    (average, total_seq_len)
}

fn compute_length_histogram(lengths: &[usize]) -> Vec<(String, usize)> {
    let bins = [
        (0, 100, "0-100"),
        (100, 500, "100-500"),
        (500, 1000, "500-1K"),
        (1000, 5000, "1K-5K"),
        (5000, 10000, "5K-10K"),
        (10000, 50000, "10K-50K"),
        (50000, 100000, "50K-100K"),
        (100000, 500000, "100K-500K"),
        (500000, 1000000, "500K-1M"),
        (1000000, usize::MAX, ">1M"),
    ];

    let mut histogram: Vec<(String, usize)> = bins
        .iter()
        .map(|(_, _, label)| (label.to_string(), 0))
        .collect();

    for &len in lengths {
        for (i, (min, max, _)) in bins.iter().enumerate() {
            if len >= *min && len < *max {
                histogram[i].1 += 1;
                break;
            }
        }
    }

    histogram
}

fn compute_degree_distributions(
    graph: &GfaGraph,
) -> (HashMap<usize, usize>, HashMap<usize, usize>) {
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut out_degree: HashMap<String, usize> = HashMap::new();

    // Initialize all segments with 0 degree
    for name in graph.segments.keys() {
        in_degree.insert(name.clone(), 0);
        out_degree.insert(name.clone(), 0);
    }

    // Count degrees from links
    for link in &graph.links {
        // Outgoing from from_segment
        *out_degree.entry(link.from_segment.clone()).or_insert(0) += 1;
        // Incoming to to_segment
        *in_degree.entry(link.to_segment.clone()).or_insert(0) += 1;
    }

    // Convert to distribution
    let mut in_dist: HashMap<usize, usize> = HashMap::new();
    let mut out_dist: HashMap<usize, usize> = HashMap::new();

    for (_, degree) in in_degree {
        *in_dist.entry(degree).or_insert(0) += 1;
    }
    for (_, degree) in out_degree {
        *out_dist.entry(degree).or_insert(0) += 1;
    }

    (in_dist, out_dist)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

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
    fn test_basic_stats() {
        let graph = create_test_graph();
        let stats = GfaStats::from_graph(&graph);

        assert_eq!(stats.segment_count, 3);
        assert_eq!(stats.link_count, 2);
        assert_eq!(stats.path_count, 1);
        assert_eq!(stats.total_sequence_length, 24);
    }

    #[test]
    fn test_gc_content() {
        let graph = create_test_graph();
        let stats = GfaStats::from_graph(&graph);

        // s1: ACGTACGT -> 4 GC out of 8
        // s2: GGGGGGGG -> 8 GC out of 8
        // s3: TTTTTTTT -> 0 GC out of 8
        // Total: 12 GC out of 24 = 50%
        assert!((stats.gc_content - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_n50() {
        let lengths = vec![10, 20, 30, 40, 50];
        let n50 = compute_n50(&lengths);
        // Total = 150, half = 75
        // Sorted desc: 50, 40, 30, 20, 10
        // cumsum: 50, 90 >= 75 -> N50 = 40
        assert_eq!(n50, 40);
    }

    #[test]
    fn test_connected_components() {
        let graph = create_test_graph();
        let stats = GfaStats::from_graph(&graph);
        assert_eq!(stats.connected_components, 1);
    }
}
