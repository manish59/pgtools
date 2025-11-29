// ================== Imports ==================

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use thiserror::Error;

// ================== BasicStats struct ==================

#[derive(Debug, Clone, Serialize)]
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

impl Default for BasicStats {
    fn default() -> Self {
        Self {
            total_lines: 0,
            node_count: 0,
            edge_count: 0,
            path_count: 0,
            other_records: 0,
            comment_lines: 0,
            total_bp: 0,
            min_node_len: u64::MAX,
            max_node_len: 0,
            gc_bases: 0,
            n_bases: 0,
        }
    }
}

impl BasicStats {
    pub fn mean_node_len(&self) -> f64 {
        if self.node_count == 0 {
            0.0
        } else {
            self.total_bp as f64 / self.node_count as f64
        }
    }

    pub fn normalized(self) -> Self {
        if self.node_count == 0 {
            Self {
                min_node_len: 0,
                ..self
            }
        } else {
            self
        }
    }
}

// ================== Error type ==================

#[derive(Error, Debug)]
pub enum GfaError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Malformed GFA line: {0}")]
    MalformedLine(String),
}

// ================== Reader helper (GFA / GFA.GZ) ==================

pub fn open_gfa_reader<P: AsRef<Path>>(path: P) -> Result<Box<dyn BufRead>, GfaError> {
    let path_ref = path.as_ref();
    let file = File::open(path_ref)?;

    // Detect .gz filename
    if let Some(ext) = path_ref.extension() {
        if ext == "gz" {
            let decoder = GzDecoder::new(file);
            return Ok(Box::new(BufReader::new(decoder)));
        }
    }

    Ok(Box::new(BufReader::new(file)))
}

// ================== Core compute functions ==================

pub fn compute_basic_stats<R: BufRead>(reader: R) -> Result<BasicStats, GfaError> {
    let mut stats = BasicStats::default();

    for line_result in reader.lines() {
        let line = line_result?;
        process_line(&mut stats, &line)?;
    }

    Ok(stats.normalized())
}

pub fn compute_basic_stats_from_path<P: AsRef<Path>>(path: P) -> Result<BasicStats, GfaError> {
    let reader = open_gfa_reader(&path)?;
    compute_basic_stats(reader)
}

pub fn compute_basic_stats_from_path_with_progress<P: AsRef<Path>>(
    path: P,
) -> Result<BasicStats, GfaError> {
    let path_ref = path.as_ref();

    let is_gz = path_ref.extension().map_or(false, |e| e == "gz");

    if is_gz {
        eprintln!("Note: .gz file detected — disabling progress bar.");
        return compute_basic_stats_from_path(path);
    }

    let file = File::open(path_ref)?;
    let metadata = file.metadata()?;
    let total_bytes = metadata.len();

    let pb = ProgressBar::new(total_bytes);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] \
             {bytes}/{total_bytes} ({eta})",
        )
        .unwrap()
        .progress_chars("█▉▊▋▌▍▎▏ "),
    );

    let mut reader = BufReader::new(file);
    let mut buf = String::new();
    let mut stats = BasicStats::default();

    loop {
        buf.clear();
        let bytes_read = reader.read_line(&mut buf)?;
        if bytes_read == 0 {
            break;
        }

        pb.inc(bytes_read as u64);
        let line = buf.trim_end_matches('\n');
        process_line(&mut stats, line)?;
    }

    pb.finish_with_message("Done");
    Ok(stats.normalized())
}

// ================== Line parsing ==================

fn process_line(stats: &mut BasicStats, line: &str) -> Result<(), GfaError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    stats.total_lines += 1;

    if trimmed.starts_with('#') {
        stats.comment_lines += 1;
        return Ok(());
    }

    let record_type = trimmed
        .chars()
        .next()
        .ok_or_else(|| GfaError::MalformedLine(line.to_string()))?;

    match record_type {
        'S' => {
            stats.node_count += 1;
            handle_segment_line(stats, trimmed, line)?;
        }
        'L' => {
            stats.edge_count += 1;
        }
        'P' => {
            stats.path_count += 1;
        }
        _ => {
            stats.other_records += 1;
        }
    }

    Ok(())
}

fn handle_segment_line(
    stats: &mut BasicStats,
    trimmed: &str,
    original_line: &str,
) -> Result<(), GfaError> {
    let mut fields = trimmed.split('\t');
    let _s = fields.next();
    let _sid = fields.next();
    let seq = fields
        .next()
        .ok_or_else(|| GfaError::MalformedLine(original_line.to_string()))?;

    if seq != "*" {
        let len = seq.len() as u64;
        stats.total_bp += len;

        if len < stats.min_node_len {
            stats.min_node_len = len;
        }
        if len > stats.max_node_len {
            stats.max_node_len = len;
        }

        for b in seq.as_bytes() {
            match b {
                b'G' | b'g' | b'C' | b'c' => stats.gc_bases += 1,
                b'N' | b'n' => stats.n_bases += 1,
                _ => {}
            }
        }
    }

    Ok(())
}
