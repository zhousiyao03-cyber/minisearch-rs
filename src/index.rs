//! Inverted index over indexed documents.
//!
//! ## Data model
//!
//! For each indexed document we keep:
//! - an **internal numeric id** ([`DocId`]) used inside posting lists for
//!   compact storage,
//! - the user-supplied **external id** (any string) so callers can look up
//!   their own records,
//! - the document's **token count**, used by BM25 length normalization.
//!
//! For each unique term we keep a **posting list**: one entry per document
//! that contains the term, with a term-frequency count.
//!
//! ## On disk
//!
//! The index is `Serialize` / `Deserialize` via `serde` and persisted with
//! `bincode 2`. Phase 2 (browser) will reuse the same format and store the
//! bytes in `IndexedDB`.
//!
//! ## Limitations
//!
//! - In-memory only at runtime; not designed for indexes that don't fit in
//!   RAM. For knosi-scale data (a few thousand notes, low MB) this is fine.
//! - Posting lists are flat `Vec`s, not delta-encoded — readability over
//!   density at this stage.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::tokenizer::{TokenizerConfig, tokenize};

/// Internal compact document identifier, assigned in insertion order.
pub type DocId = u32;

/// One entry in a posting list: which document, and how many times the term
/// occurs in it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Posting {
    /// The internal document id this posting refers to.
    pub doc_id: DocId,
    /// How often the term appears in that document.
    pub term_freq: u32,
}

/// Per-document metadata kept alongside the inverted index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocMeta {
    /// User-supplied identifier (any string).
    pub external_id: String,
    /// Token count after tokenization — needed for BM25 length normalization.
    pub length: u32,
}

/// Inverted index plus the document table needed to score it.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Index {
    /// `term -> postings` (postings sorted ascending by `doc_id`).
    postings: HashMap<String, Vec<Posting>>,
    /// `doc_id -> metadata`. Indexed by position so `doc_id == idx as u32`.
    docs: Vec<DocMeta>,
    /// `external_id -> doc_id` for duplicate detection and external lookup.
    external_to_internal: HashMap<String, DocId>,
}

impl Index {
    /// Create a new empty index.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of documents currently in the index.
    #[must_use]
    pub fn doc_count(&self) -> usize {
        self.docs.len()
    }

    /// Number of unique terms (vocabulary size).
    #[must_use]
    pub fn term_count(&self) -> usize {
        self.postings.len()
    }

    /// Average document length, used by BM25.
    ///
    /// Returns `0.0` if the index is empty.
    #[must_use]
    pub fn avg_doc_length(&self) -> f32 {
        if self.docs.is_empty() {
            return 0.0;
        }
        let total: u64 = self.docs.iter().map(|d| u64::from(d.length)).sum();
        // Precision loss is fine — BM25 only needs an approximation.
        #[allow(clippy::cast_precision_loss)]
        let total_f = total as f32;
        #[allow(clippy::cast_precision_loss)]
        let n = self.docs.len() as f32;
        total_f / n
    }

    /// Look up the metadata for an internal `doc_id`.
    #[must_use]
    pub fn doc(&self, doc_id: DocId) -> Option<&DocMeta> {
        self.docs.get(doc_id as usize)
    }

    /// Look up the posting list for a term. Returns an empty slice if the
    /// term is unknown.
    #[must_use]
    pub fn postings(&self, term: &str) -> &[Posting] {
        self.postings
            .get(term)
            .map_or(&[] as &[Posting], Vec::as_slice)
    }

    /// Add a document to the index.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DuplicateDocId`] if `external_id` is already
    /// indexed. Re-indexing the same document is not yet supported (Phase 1
    /// keeps things simple).
    ///
    /// # Panics
    ///
    /// Panics if more than `u32::MAX` (~4.3 billion) documents are added —
    /// well beyond what this engine is designed for. For knosi-scale data
    /// (thousands of notes) this is unreachable.
    pub fn add_document(
        &mut self,
        external_id: impl Into<String>,
        text: &str,
        config: &TokenizerConfig,
    ) -> Result<DocId> {
        let external_id = external_id.into();
        if self.external_to_internal.contains_key(&external_id) {
            return Err(Error::DuplicateDocId(external_id));
        }

        let tokens = tokenize(text, config);
        // Aggregate term frequencies inside this document.
        let mut tf: HashMap<String, u32> = HashMap::new();
        for tok in &tokens {
            *tf.entry(tok.term.clone()).or_default() += 1;
        }

        let doc_id: DocId = u32::try_from(self.docs.len()).expect("doc id overflow");
        let length = u32::try_from(tokens.len()).unwrap_or(u32::MAX);
        self.docs.push(DocMeta {
            external_id: external_id.clone(),
            length,
        });
        self.external_to_internal.insert(external_id, doc_id);

        // Append to each term's posting list. We maintain ascending doc_id
        // order trivially because we only ever append.
        for (term, freq) in tf {
            self.postings.entry(term).or_default().push(Posting {
                doc_id,
                term_freq: freq,
            });
        }

        Ok(doc_id)
    }

    /// Encode the index to bytes using `bincode 2`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Encode`] if serialization fails.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let config = bincode::config::standard();
        bincode::serde::encode_to_vec(self, config).map_err(|e| Error::Encode(e.to_string()))
    }

    /// Decode an index from bytes produced by [`Index::to_bytes`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::Decode`] if the bytes are malformed.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let config = bincode::config::standard();
        let (index, _read) = bincode::serde::decode_from_slice::<Self, _>(bytes, config)
            .map_err(|e| Error::Decode(e.to_string()))?;
        Ok(index)
    }

    /// Save the index to a file. Convenience around [`Index::to_bytes`].
    ///
    /// # Errors
    ///
    /// Returns any I/O or encoding error.
    pub fn save_to(&self, path: impl AsRef<Path>) -> Result<()> {
        let bytes = self.to_bytes()?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// Load an index from a file. Convenience around [`Index::from_bytes`].
    ///
    /// # Errors
    ///
    /// Returns any I/O or decoding error.
    pub fn load_from(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        Self::from_bytes(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> TokenizerConfig {
        TokenizerConfig::default()
    }

    #[test]
    fn empty_index_has_zero_counts() {
        let idx = Index::new();
        assert_eq!(idx.doc_count(), 0);
        assert_eq!(idx.term_count(), 0);
        assert!((idx.avg_doc_length() - 0.0).abs() < f32::EPSILON);
        assert!(idx.postings("anything").is_empty());
    }

    #[test]
    fn adds_documents_and_assigns_sequential_ids() {
        let mut idx = Index::new();
        let id0 = idx.add_document("a", "rust web", &cfg()).unwrap();
        let id1 = idx.add_document("b", "rust browser", &cfg()).unwrap();
        assert_eq!(id0, 0);
        assert_eq!(id1, 1);
        assert_eq!(idx.doc_count(), 2);
        assert_eq!(idx.doc(0).unwrap().external_id, "a");
        assert_eq!(idx.doc(1).unwrap().external_id, "b");
    }

    #[test]
    fn duplicate_external_id_is_rejected() {
        let mut idx = Index::new();
        idx.add_document("a", "rust", &cfg()).unwrap();
        let err = idx.add_document("a", "again", &cfg()).unwrap_err();
        assert!(matches!(err, Error::DuplicateDocId(ref s) if s == "a"));
    }

    #[test]
    fn posting_lists_are_built_correctly() {
        let mut idx = Index::new();
        idx.add_document("a", "rust rust web", &cfg()).unwrap();
        idx.add_document("b", "rust browser", &cfg()).unwrap();

        let rust = idx.postings("rust");
        assert_eq!(rust.len(), 2);
        let p0 = rust.iter().find(|p| p.doc_id == 0).unwrap();
        let p1 = rust.iter().find(|p| p.doc_id == 1).unwrap();
        assert_eq!(p0.term_freq, 2); // "rust" appears twice in doc a
        assert_eq!(p1.term_freq, 1);

        assert_eq!(idx.postings("web").len(), 1);
        assert_eq!(idx.postings("browser").len(), 1);
        assert!(idx.postings("missing").is_empty());
    }

    #[test]
    fn avg_doc_length_is_correct() {
        let mut idx = Index::new();
        // Stopword filtering means content words only.
        idx.add_document("a", "rust web", &cfg()).unwrap(); // length 2
        idx.add_document("b", "rust browser webassembly", &cfg())
            .unwrap(); // length 3
        let avg = idx.avg_doc_length();
        assert!((avg - 2.5).abs() < 1e-5);
    }

    #[test]
    fn round_trip_via_bytes() {
        let mut idx = Index::new();
        idx.add_document("a", "Rust is a memory safe systems language.", &cfg())
            .unwrap();
        idx.add_document("b", "WebAssembly lets Rust run in the browser.", &cfg())
            .unwrap();

        let bytes = idx.to_bytes().unwrap();
        let loaded = Index::from_bytes(&bytes).unwrap();

        assert_eq!(loaded.doc_count(), idx.doc_count());
        assert_eq!(loaded.term_count(), idx.term_count());
        assert_eq!(loaded.postings("rust").len(), idx.postings("rust").len());
        assert_eq!(loaded.doc(0), idx.doc(0));
    }

    #[test]
    fn round_trip_via_disk() {
        let mut idx = Index::new();
        idx.add_document("a", "rust web search", &cfg()).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.bin");
        idx.save_to(&path).unwrap();

        let loaded = Index::load_from(&path).unwrap();
        assert_eq!(loaded.doc_count(), 1);
        assert_eq!(loaded.postings("rust").len(), 1);
    }
}
