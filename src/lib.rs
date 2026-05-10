//! # minisearch-rs
//!
//! A tiny, embeddable BM25 full-text search engine. Designed for "thousands
//! of documents, low MB" workloads — note-taking apps, doc search, in-page
//! filters — where shipping `tantivy` or running Meilisearch is overkill.
//!
//! The same API runs natively (CLI, server) and in the browser via
//! `wasm-bindgen`.
//!
//! ## Modules
//!
//! - [`tokenizer`] — Unicode-aware tokenization with stopword filtering.
//! - [`index`] — Inverted index with serde-based persistence.
//! - [`search`] — BM25 ranking over the inverted index.
//! - [`snippet`] — Context window extraction around query matches.
//!
//! ## Example
//!
//! ```ignore
//! use minisearch_rs::{Engine, EngineConfig};
//!
//! let mut engine = Engine::new(EngineConfig::default());
//! engine.add_document("doc-1", "Rust is a memory-safe systems language.");
//! engine.add_document("doc-2", "WebAssembly lets Rust run in the browser.");
//!
//! let hits = engine.search("rust browser", 10);
//! assert!(!hits.is_empty());
//! ```

#![doc(html_root_url = "https://docs.rs/minisearch-rs/0.1.0")]

pub mod error;
pub mod index;
pub mod search;
pub mod snippet;
pub mod tokenizer;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub use error::{Error, Result};
pub use index::Index;
pub use search::{Engine, EngineConfig, SearchHit};
pub use snippet::{Snippet, SnippetConfig, extract as extract_snippet};
