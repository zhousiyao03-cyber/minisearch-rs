# Changelog

All notable changes to this project will be documented in this file.

The format is loosely based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] — 2026-05-10

Initial public release.

### Added

- BM25 ranking with Lucene-flavor parameters (`k1`, `b`).
- Inverted index with `serde` + `bincode 2` persistence
  (`Index::to_bytes` / `Index::from_bytes` / `save_to` / `load_from`).
- Unicode-aware tokenizer (`unicode-segmentation`) with optional
  English stopwords and configurable minimum token length.
- Best-window snippet extractor with byte-offset highlights and
  whole-word boundary trimming.
- `Engine` façade combining tokenizer + index + scoring under one
  config (`EngineConfig`).
- `examples/cli.rs` — `index <dir> [--out <file>]`,
  `search <query> [--idx <file>]`, `info` subcommands.
- WebAssembly bindings (`#[cfg(target_arch = "wasm32")]`) via
  `wasm-bindgen` — exposes `JsEngine` with `addDocument`, `search`,
  `toBytes`, `fromBytes`. Snippet highlight offsets are translated
  from UTF-8 bytes to UTF-16 code units so JS can index strings
  directly.
- Browser demo under `demo/` with a curated 8-document corpus, live
  stats, and snippet previews.
- 29 unit tests, 1 integration test, and a `proptest`-based property
  test for tokenizer offset round-tripping.
- GitHub Actions CI: `cargo fmt --check`, `cargo clippy -- -D warnings`
  (with `clippy::pedantic`), `cargo test` on every push and PR.

[Unreleased]: https://github.com/zhousiyao03-cyber/minisearch-rs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/zhousiyao03-cyber/minisearch-rs/releases/tag/v0.1.0
