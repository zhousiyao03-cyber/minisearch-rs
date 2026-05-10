//! End-to-end integration test: build → save → load → search.

use minisearch_rs::{Engine, EngineConfig, Index};

#[test]
fn build_save_load_search_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("idx.bin");

    {
        let mut engine = Engine::new(EngineConfig::default());
        engine
            .add_document(
                "rust-intro.md",
                "Rust is a memory-safe systems programming language with strong tooling.",
            )
            .unwrap();
        engine
            .add_document(
                "wasm-intro.md",
                "WebAssembly lets Rust ship to the browser as a fast portable binary.",
            )
            .unwrap();
        engine
            .add_document(
                "search-intro.md",
                "BM25 is the de-facto baseline ranking algorithm for full-text search.",
            )
            .unwrap();
        engine.index().save_to(&path).unwrap();
    }

    let loaded = Index::load_from(&path).unwrap();
    let engine = Engine::from_index(loaded, EngineConfig::default());

    // Term "rust" appears in two docs; rust-intro should win because the
    // doc is shorter and rust appears first.
    let hits = engine.search("rust", 10);
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].doc_id, "rust-intro.md");

    // "BM25" is rare, so search-intro should rank highest for it.
    let hits = engine.search("bm25", 10);
    assert_eq!(hits[0].doc_id, "search-intro.md");

    // Multi-term query.
    let hits = engine.search("rust browser", 10);
    assert_eq!(hits[0].doc_id, "wasm-intro.md");
}
