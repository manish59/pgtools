use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Debug, Default, Clone)]
pub struct BasicStats {
    pub total_lines: u64,
    pub node_count: u64,
    pub edge_count: u64,
    pub path_count: u64,
    pub other_records: u64,
    pub comment_lines: u64,
    pub total_bp: u64,
    pub min_node_len: u64,
    pub max_node_len: u64,
    pub gc_bases: u64,
    pub n_bases: u64,
}

impl BasicStats {
    pub fn new() -> Self {
        Self {
            min_node_len: u64::MAX,
            ..Default::default()
        }
    }

    pub fn mean_node_len(&self) -> f64 {
        if self.node_count == 0 {
            0.0
        } else {
            self.total_bp as f64 / self.node_count as f64
        }
    }

    fn process_line(&mut self, line: &str) {
        self.total_lines += 1;

        if line.is_empty() {
            return;
        }

        let first_char = line.as_bytes()[0];

        match first_char {
            b'#' => {
                self.comment_lines += 1;
            }
            b'S' => {
                self.node_count += 1;
                if let Some(seq) = extract_segment_sequence(line) {
                    let len = seq.len() as u64;
                    self.total_bp += len;
                    self.min_node_len = self.min_node_len.min(len);
                    self.max_node_len = self.max_node_len.max(len);

                    // Count GC and N bases
                    for &byte in seq.as_bytes() {
                        match byte {
                            b'G' | b'C' | b'g' | b'c' => self.gc_bases += 1,
                            b'N' | b'n' => self.n_bases += 1,
                            _ => {}
                        }
                    }
                }
            }
            b'L' => {
                self.edge_count += 1;
            }
            b'P' => {
                self.path_count += 1;
            }
            _ => {
                self.other_records += 1;
            }
        }
    }
}

fn extract_segment_sequence(line: &str) -> Option<&str> {
    // GFA format: S <name> <sequence> [<optional_fields>...]
    let mut fields = line.split('\t');
    fields.next()?; // Skip 'S'
    fields.next()?; // Skip name
    fields.next() // Return sequence
}

pub fn compute_basic_stats_from_path<P: AsRef<Path>>(path: P) -> Result<BasicStats> {
    let path = path.as_ref();
    let file = File::open(path)
        .with_context(|| format!("Failed to open file: {}", path.display()))?;

    let reader: Box<dyn Read> = if path.extension().and_then(|s| s.to_str()) == Some("gz") {
        Box::new(GzDecoder::new(file))
    } else {
        Box::new(file)
    };

    let mut stats = BasicStats::new();
    let buf_reader = BufReader::new(reader);

    for line in buf_reader.lines() {
        let line = line.context("Failed to read line")?;
        stats.process_line(&line);
    }

    // Fix min_node_len if no nodes were found
    if stats.node_count == 0 {
        stats.min_node_len = 0;
    }

    Ok(stats)
}

pub fn compute_basic_stats_from_path_with_progress<P: AsRef<Path>>(
    path: P,
) -> Result<BasicStats> {
    let path = path.as_ref();
    let file = File::open(path)
        .with_context(|| format!("Failed to open file: {}", path.display()))?;

    let file_size = file.metadata()?.len();

    let reader: Box<dyn Read> = if path.extension().and_then(|s| s.to_str()) == Some("gz") {
        Box::new(GzDecoder::new(file))
    } else {
        Box::new(file)
    };

    let pb = ProgressBar::new(file_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
            .progress_chars("#>-"),
    );

    let mut stats = BasicStats::new();
    let buf_reader = BufReader::new(reader);

    for line in buf_reader.lines() {
        let line = line.context("Failed to read line")?;
        stats.process_line(&line);
        pb.inc(line.len() as u64 + 1); // +1 for newline
    }

    pb.finish_with_message("Done");

    // Fix min_node_len if no nodes were found
    if stats.node_count == 0 {
        stats.min_node_len = 0;
    }

    Ok(stats)
}