//! WebAssembly bindings.
//!
//! Compiled only for `wasm32` targets. The bindings are a thin facade over
//! [`crate::Engine`]: documents in, ranked hits + snippets out. Index bytes
//! are exposed as `Uint8Array` so callers can persist them to `IndexedDB`,
//! `localStorage`, or the network.

#![cfg(target_arch = "wasm32")]
#![allow(missing_docs)]

use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::tokenizer::TokenizerConfig;
use crate::{Engine, EngineConfig, Index, SnippetConfig, extract_snippet};

/// Install a one-shot panic hook that forwards Rust panics to the JS
/// console. Idempotent — safe to call from every entry point.
fn install_panic_hook() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        console_error_panic_hook::set_once();
    });
}

/// JS-facing search engine. Owns an [`Engine`] internally.
#[wasm_bindgen]
pub struct JsEngine {
    inner: Engine,
}

/// Search hit shaped for JS consumers (snippet text + highlight ranges
/// included).
#[derive(Serialize)]
struct JsHit<'a> {
    doc_id: &'a str,
    score: f32,
    snippet: Option<JsSnippet>,
}

#[derive(Serialize)]
struct JsSnippet {
    text: String,
    highlights: Vec<(usize, usize)>,
}

#[wasm_bindgen]
impl JsEngine {
    /// Create an empty engine with default BM25 config.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        install_panic_hook();
        Self {
            inner: Engine::new(EngineConfig::default()),
        }
    }

    /// Restore an engine from a previously-saved index byte blob.
    ///
    /// Returns a `JsValue` error if decoding fails.
    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, JsValue> {
        install_panic_hook();
        let index = Index::from_bytes(bytes).map_err(to_js_err)?;
        Ok(Self {
            inner: Engine::from_index(index, EngineConfig::default()),
        })
    }

    /// Number of indexed documents.
    #[wasm_bindgen(js_name = docCount)]
    #[must_use]
    pub fn doc_count(&self) -> usize {
        self.inner.doc_count()
    }

    /// Number of unique terms (vocabulary size).
    #[wasm_bindgen(js_name = termCount)]
    #[must_use]
    pub fn term_count(&self) -> usize {
        self.inner.index().term_count()
    }

    /// Add a document. Returns a JS error if `id` is a duplicate.
    #[wasm_bindgen(js_name = addDocument)]
    pub fn add_document(&mut self, id: &str, text: &str) -> Result<(), JsValue> {
        self.inner.add_document(id, text).map_err(to_js_err)?;
        Ok(())
    }

    /// Run a query, return up to `top_k` ranked hits as a JS array of
    /// `{ doc_id, score, snippet?: { text, highlights: [[s,e], …] } }`.
    ///
    /// `corpus` is an optional `id -> original text` map (`Map<string, string>`
    /// from JS) used to build snippets. Pass `null`/`undefined` to skip
    /// snippet generation.
    pub fn search(&self, query: &str, top_k: usize, corpus: JsValue) -> Result<JsValue, JsValue> {
        let hits = self.inner.search(query, top_k);
        let query_terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        let term_refs: Vec<&str> = query_terms.iter().map(String::as_str).collect();

        // Decode the optional corpus map. We accept either `undefined`/`null`
        // (no snippets) or a `Record<string, string>` we can serde-decode.
        let corpus_map: Option<std::collections::HashMap<String, String>> =
            if corpus.is_undefined() || corpus.is_null() {
                None
            } else {
                Some(serde_wasm_bindgen::from_value(corpus).map_err(to_js_err_dyn)?)
            };

        let tcfg = TokenizerConfig::default();
        let scfg = SnippetConfig::default();

        let payload: Vec<JsHit<'_>> = hits
            .iter()
            .map(|h| {
                let snippet = corpus_map
                    .as_ref()
                    .and_then(|m| m.get(&h.doc_id))
                    .and_then(|text| extract_snippet(text, &term_refs, &tcfg, &scfg))
                    .map(|s| {
                        // Convert UTF-8 byte offsets into UTF-16 code-unit
                        // offsets so JS can index the string directly with
                        // `.slice()`. JS strings are UTF-16; supplementary
                        // characters count as two code units.
                        let highlights = s
                            .highlights
                            .iter()
                            .map(|&(start_b, end_b)| {
                                (
                                    byte_to_utf16_offset(&s.text, start_b),
                                    byte_to_utf16_offset(&s.text, end_b),
                                )
                            })
                            .collect();
                        JsSnippet {
                            text: s.text,
                            highlights,
                        }
                    });
                JsHit {
                    doc_id: &h.doc_id,
                    score: h.score,
                    snippet,
                }
            })
            .collect();

        serde_wasm_bindgen::to_value(&payload).map_err(to_js_err_dyn)
    }

    /// Serialize the index to a `Uint8Array` so callers can persist it.
    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Result<Vec<u8>, JsValue> {
        self.inner.index().to_bytes().map_err(to_js_err)
    }
}

impl Default for JsEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn to_js_err(e: crate::Error) -> JsValue {
    JsValue::from_str(&e.to_string())
}

fn to_js_err_dyn(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

/// Translate a UTF-8 byte offset within `s` into a UTF-16 code-unit
/// offset so that JavaScript can use it directly with `String.prototype.slice`.
///
/// Saturates at the string's length if the byte offset is past the end.
fn byte_to_utf16_offset(s: &str, byte: usize) -> usize {
    if byte >= s.len() {
        return s.encode_utf16().count();
    }
    let prefix = &s[..byte];
    prefix.encode_utf16().count()
}
