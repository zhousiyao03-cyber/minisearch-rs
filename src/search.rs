//! BM25 ranking over the inverted index.
//!
//! ## Why BM25?
//!
//! BM25 is the de-facto baseline ranking function for full-text search and
//! the default in Lucene, Elasticsearch, and tantivy. For a document `D` and
//! query `Q = {q1, q2, ..., qn}` it scores:
//!
//! ```text
//! score(D, Q) = Σ IDF(qi) * (tf(qi, D) * (k1 + 1))
//!                          / (tf(qi, D) + k1 * (1 - b + b * |D| / avgdl))
//! ```
//!
//! - `tf(qi, D)` — how many times term `qi` appears in `D`
//! - `|D|` — `D`'s length in tokens
//! - `avgdl` — average document length in the corpus
//! - `IDF(qi) = ln((N - df + 0.5) / (df + 0.5) + 1)` — rare terms count more
//! - `k1` (default 1.2) — controls term-frequency saturation: small `k1` ≈
//!   binary "matched / didn't match", large `k1` ≈ raw term-frequency.
//! - `b` (default 0.75) — controls length normalization: `b=0` ignores
//!   document length entirely, `b=1` fully normalizes by average length.
//!
//! ## Engine
//!
//! [`Engine`] is the top-level façade that owns an [`Index`] and exposes
//! `add_document` / `search`. It hides the BM25 math from callers; you can
//! still tune `k1` / `b` via [`EngineConfig`] for experiments.

use std::collections::HashMap;

use crate::error::Result;
use crate::index::Index;
use crate::tokenizer::{TokenizerConfig, tokenize};

/// BM25 hyperparameters and the engine's tokenizer settings.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// `k1` — term frequency saturation. Default `1.2` (Lucene/ES default).
    pub k1: f32,
    /// `b` — length normalization. Default `0.75` (Lucene/ES default).
    pub b: f32,
    /// Tokenizer used for both indexing and querying. Must be the same on
    /// both sides — that's the point of carrying it on the engine.
    pub tokenizer: TokenizerConfig,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            k1: 1.2,
            b: 0.75,
            tokenizer: TokenizerConfig::default(),
        }
    }
}

/// One scored search result.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    /// User-supplied document id.
    pub doc_id: String,
    /// BM25 score; higher is better. Not normalized — only meaningful for
    /// ranking within a single query.
    pub score: f32,
}

/// Top-level search engine.
#[derive(Debug)]
pub struct Engine {
    config: EngineConfig,
    index: Index,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new(EngineConfig::default())
    }
}

impl Engine {
    /// Create a new engine with the given configuration and an empty index.
    #[must_use]
    pub fn new(config: EngineConfig) -> Self {
        Self {
            config,
            index: Index::new(),
        }
    }

    /// Build an engine from a pre-existing index (e.g. loaded from disk).
    #[must_use]
    pub fn from_index(index: Index, config: EngineConfig) -> Self {
        Self { config, index }
    }

    /// Borrow the underlying [`Index`] (read-only).
    #[must_use]
    pub fn index(&self) -> &Index {
        &self.index
    }

    /// Borrow the engine config.
    #[must_use]
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// Number of documents currently indexed.
    #[must_use]
    pub fn doc_count(&self) -> usize {
        self.index.doc_count()
    }

    /// Add a document.
    ///
    /// # Errors
    ///
    /// Propagates any error from [`Index::add_document`], including
    /// duplicate external ids.
    pub fn add_document(&mut self, external_id: impl Into<String>, text: &str) -> Result<()> {
        self.index
            .add_document(external_id, text, &self.config.tokenizer)?;
        Ok(())
    }

    /// Run a query and return up to `top_k` ranked hits.
    ///
    /// Returns an empty `Vec` if there are no matches.
    #[must_use]
    pub fn search(&self, query: &str, top_k: usize) -> Vec<SearchHit> {
        if top_k == 0 || self.index.doc_count() == 0 {
            return Vec::new();
        }

        let tokens = tokenize(query, &self.config.tokenizer);
        if tokens.is_empty() {
            return Vec::new();
        }

        // Deduplicate query terms — repeating "rust rust" should not
        // double-count the BM25 contribution. (Lucene's default BM25Similarity
        // does the same.)
        let mut query_terms: Vec<&str> = tokens.iter().map(|t| t.term.as_str()).collect();
        query_terms.sort_unstable();
        query_terms.dedup();

        let n_docs = self.index.doc_count();
        let avgdl = self.index.avg_doc_length();
        let k1 = self.config.k1;
        let b = self.config.b;

        // Accumulate scores per document.
        let mut scores: HashMap<u32, f32> = HashMap::new();

        for term in &query_terms {
            let postings = self.index.postings(term);
            if postings.is_empty() {
                continue;
            }
            let df = postings.len();
            let idf = idf_score(n_docs, df);

            for posting in postings {
                let Some(doc) = self.index.doc(posting.doc_id) else {
                    continue;
                };

                #[allow(clippy::cast_precision_loss)]
                let tf = posting.term_freq as f32;
                #[allow(clippy::cast_precision_loss)]
                let dl = doc.length as f32;
                let norm = 1.0 - b + b * (dl / avgdl);
                let term_score = idf * (tf * (k1 + 1.0)) / (tf + k1 * norm);
                *scores.entry(posting.doc_id).or_insert(0.0) += term_score;
            }
        }

        let mut hits: Vec<SearchHit> = scores
            .into_iter()
            .filter_map(|(doc_id, score)| {
                self.index.doc(doc_id).map(|meta| SearchHit {
                    doc_id: meta.external_id.clone(),
                    score,
                })
            })
            .collect();

        // Sort by score desc, breaking ties by external id for determinism.
        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.doc_id.cmp(&b.doc_id))
        });
        hits.truncate(top_k);
        hits
    }
}

/// BM25 inverse-document-frequency, smoothed (Lucene variant).
fn idf_score(n_docs: usize, df: usize) -> f32 {
    #[allow(clippy::cast_precision_loss)]
    let n = n_docs as f32;
    #[allow(clippy::cast_precision_loss)]
    let df_f = df as f32;
    ((n - df_f + 0.5) / (df_f + 0.5) + 1.0).ln()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Engine {
        Engine::default()
    }

    #[test]
    fn empty_engine_returns_no_hits() {
        let e = engine();
        assert!(e.search("anything", 10).is_empty());
    }

    #[test]
    fn unmatched_query_returns_empty() {
        let mut e = engine();
        e.add_document("a", "rust web search").unwrap();
        assert!(e.search("python", 10).is_empty());
    }

    #[test]
    fn ranks_more_relevant_doc_first() {
        let mut e = engine();
        // Doc a mentions "rust" twice; doc b once. Same length-ish.
        e.add_document("a", "rust rust web search engine").unwrap();
        e.add_document("b", "rust web search engine indeed")
            .unwrap();

        let hits = e.search("rust", 10);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].doc_id, "a");
        assert_eq!(hits[1].doc_id, "b");
        assert!(hits[0].score > hits[1].score);
    }

    #[test]
    fn rare_terms_outweigh_common_terms() {
        let mut e = engine();
        // "rust" appears in both docs; "wasm" only in b. Searching for both
        // should rank b higher because "wasm" has higher IDF.
        e.add_document("a", "rust rust rust rust").unwrap();
        e.add_document("b", "rust wasm").unwrap();

        let hits = e.search("rust wasm", 10);
        assert_eq!(hits[0].doc_id, "b");
    }

    #[test]
    fn longer_doc_with_same_tf_scores_lower() {
        // BM25's length normalization should prefer the shorter doc when the
        // term frequency is equal.
        let mut e = engine();
        e.add_document("short", "rust").unwrap();
        e.add_document(
            "long",
            "rust web search engine indexing tokenizer scoring browser",
        )
        .unwrap();

        let hits = e.search("rust", 10);
        assert_eq!(hits[0].doc_id, "short");
        assert_eq!(hits[1].doc_id, "long");
    }

    #[test]
    fn top_k_truncates_results() {
        let mut e = engine();
        for i in 0..5 {
            e.add_document(format!("d{i}"), "rust web").unwrap();
        }
        let hits = e.search("rust", 3);
        assert_eq!(hits.len(), 3);
    }

    #[test]
    fn duplicate_query_terms_do_not_double_score() {
        let mut e = engine();
        e.add_document("a", "rust web search").unwrap();
        let single = &e.search("rust", 1)[0];
        let doubled = &e.search("rust rust rust", 1)[0];
        assert!(
            (single.score - doubled.score).abs() < 1e-5,
            "expected query-term dedup, got {} vs {}",
            single.score,
            doubled.score
        );
    }

    #[test]
    fn deterministic_tie_break_by_doc_id() {
        // Two identical docs should tie on score and break by external id
        // alphabetically — making the snapshot below stable.
        let mut e = engine();
        e.add_document("zeta", "rust web").unwrap();
        e.add_document("alpha", "rust web").unwrap();
        let hits = e.search("rust", 10);
        assert_eq!(hits[0].doc_id, "alpha");
        assert_eq!(hits[1].doc_id, "zeta");
    }
}
