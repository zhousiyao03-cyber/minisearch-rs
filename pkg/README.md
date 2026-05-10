# minisearch-rs

A tiny, embeddable BM25 full-text search engine for Rust and the browser.

- **Compact** — ~1k LOC of pure Rust, no `unsafe`, designed to compile to a small wasm bundle.
- **Self-contained** — single index file (`bincode 2`), no server, no daemon, no schema files.
- **Drop-in for small corpora** — built for "thousands of documents, low MB" workloads where shipping `tantivy` or running Meilisearch is overkill.
- **Same API on native and wasm** — score notes, docs, or messages from a CLI today and from a browser tomorrow.

## What it does

| Capability | Details |
|---|---|
| Tokenization | Unicode-aware (`unicode-segmentation`), lowercase, configurable English stopwords |
| Indexing | In-memory inverted index, persisted via `serde` + `bincode 2` |
| Ranking | Lucene-flavor BM25 with tunable `k1` / `b` |
| Snippets | Best-window extraction with byte-offset highlights and whole-word boundaries |
| Storage | One file (`*.bin`) — works on disk, in `IndexedDB`, or in-memory bytes |

## Quick start (Rust)

```rust
use minisearch_rs::{Engine, EngineConfig};

let mut engine = Engine::new(EngineConfig::default());
engine.add_document("doc-1", "Rust is a memory-safe systems language.")?;
engine.add_document("doc-2", "WebAssembly lets Rust run in the browser.")?;

let hits = engine.search("rust browser", 10);
assert_eq!(hits[0].doc_id, "doc-2");
```

## CLI

```bash
cargo run --example cli --release -- index  ./docs
cargo run --example cli --release -- search "rust browser"
cargo run --example cli --release -- info
```

The CLI walks a directory for `*.md` files, builds an index at `./minisearch.bin`, and prints ranked hits with snippets.

## Roadmap

- [x] **Phase 1** — Native library + CLI (BM25, snippets, on-disk index)
- [ ] **Phase 2** — `wasm-bindgen` build + browser demo
- [ ] **Phase 3** — Optional [knosi](https://www.knosi.xyz) integration

## Configuration

```rust
EngineConfig {
    k1: 1.2,                   // term-frequency saturation (Lucene default)
    b:  0.75,                  // length normalization     (Lucene default)
    tokenizer: TokenizerConfig {
        drop_punct:     true,
        drop_stopwords: true,
        min_len:        1,
    },
}
```

## Stack

- Rust 2024 edition (rustc ≥ 1.85)
- `unicode-segmentation` · `serde` · `bincode 2` · `thiserror`
- Tests: built-in `#[test]` · `insta` · `proptest`
- CI: `cargo fmt --check` · `cargo clippy -- -D warnings` (with `clippy::pedantic`) · `cargo test`

## License

MIT
