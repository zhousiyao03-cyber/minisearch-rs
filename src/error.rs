//! Error types for the minisearch library.
//!
//! Following the 2026 Rust convention: libraries define typed errors via
//! `thiserror`, while applications (the CLI here) wrap them via `anyhow`.

use thiserror::Error;

/// Convenience alias: `Result<T, minisearch_rs::Error>`.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors produced by the search engine.
#[derive(Debug, Error)]
pub enum Error {
    /// I/O failed while loading or saving an index.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// The on-disk index could not be decoded.
    #[error("decode error: {0}")]
    Decode(String),

    /// The in-memory index could not be encoded for persistence.
    #[error("encode error: {0}")]
    Encode(String),

    /// A document with the given external id already exists.
    #[error("duplicate document id: {0}")]
    DuplicateDocId(String),
}
