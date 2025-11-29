// Core statistics module
mod stats;

// Re-export public API
pub use stats::{compute_basic_stats_from_path, compute_basic_stats_from_path_with_progress, BasicStats};

// ================== Python bindings ==================

use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};

#[pyfunction]
fn stats_basic<'py>(py: Python<'py>, path: &str) -> PyResult<Bound<'py, PyDict>> {
    let stats = compute_basic_stats_from_path(path)
        .map_err(|e| PyIOError::new_err(e.to_string()))?;

    let root = PyDict::new_bound(py);

    let nodes = PyDict::new_bound(py);
    nodes.set_item("count", stats.node_count)?;
    nodes.set_item("total_bp", stats.total_bp)?;
    nodes.set_item("min_length", stats.min_node_len)?;
    nodes.set_item("max_length", stats.max_node_len)?;
    nodes.set_item("mean_length", stats.mean_node_len())?;
    root.set_item("nodes", nodes)?;

    let edges = PyDict::new_bound(py);
    edges.set_item("count", stats.edge_count)?;
    root.set_item("edges", edges)?;

    let paths = PyDict::new_bound(py);
    paths.set_item("count", stats.path_count)?;
    root.set_item("paths", paths)?;

    let bases = PyDict::new_bound(py);
    bases.set_item("gc", stats.gc_bases)?;
    bases.set_item("n", stats.n_bases)?;
    root.set_item("bases", bases)?;

    root.set_item("total_lines", stats.total_lines)?;
    root.set_item("other_records", stats.other_records)?;
    root.set_item("comment_lines", stats.comment_lines)?;

    Ok(root)
}

#[pymodule]
fn pgtools(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(stats_basic, m)?)?;
    Ok(())
}