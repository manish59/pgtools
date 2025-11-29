//! Error types for pgtools

use thiserror::Error;

/// Result type alias for pgtools operations
pub type Result<T> = std::result::Result<T, PgToolsError>;

/// Main error type for pgtools
#[derive(Error, Debug)]
pub enum PgToolsError {
    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// GFA parsing errors
    #[error("GFA parse error at line {line}: {message}")]
    GfaParse { line: usize, message: String },

    /// Index errors
    #[error("Index error: {0}")]
    Index(String),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Invalid input errors
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// File not found errors
    #[error("File not found: {0}")]
    FileNotFound(String),
}

impl From<bincode::Error> for PgToolsError {
    fn from(err: bincode::Error) -> Self {
        PgToolsError::Serialization(err.to_string())
    }
}

impl From<serde_json::Error> for PgToolsError {
    fn from(err: serde_json::Error) -> Self {
        PgToolsError::Serialization(err.to_string())
    }
}
