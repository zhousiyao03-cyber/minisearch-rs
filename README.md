# minisearch-rs

A tiny BM25 full-text search engine, built from scratch in Rust.

**Status: 🚧 Phase 1 (pure Rust + CLI)**

## Why this exists

This is a learning project. The goal is to understand how a full-text search
engine actually works by building one — not to compete with `tantivy` or
`MeiliSearch`. The engine targets WebAssembly so it can ship inside a browser
later (Phase 2).

## Roadmap

- [ ] **Phase 1** — Pure Rust library + CLI
  - [ ] Tokenizer (Unicode segmentation + English stopwords)
  - [ ] Inverted index with `serde` + `bincode 2` persistence
  - [ ] BM25 scoring with tunable `k1` / `b`
  - [ ] Snippet extraction with match highlighting
  - [ ] CLI: `index <dir>` and `search <query>`
- [ ] **Phase 2** — `wasm-bindgen` + browser demo
- [ ] **Phase 3** — Optional integration with [knosi](https://www.knosi.xyz)

## Stack

- Rust 2024 edition (rustc 1.85+)
- `unicode-segmentation` for tokenization
- `serde` + `bincode 2` for index serialization
- `thiserror` (library errors) + `anyhow` (CLI)
- `insta` (snapshot tests) + `proptest` (property-based tests)

## Develop

```bash
cargo test
cargo run --example cli -- index ./samples
cargo run --example cli -- search "rust wasm"
```

## License

MIT
