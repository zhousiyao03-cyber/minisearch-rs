/* Curated sample corpus for the minisearch-rs browser demo. */
export const SAMPLE_DOCS = [
  {
    id: "rust.md",
    text: `Rust is a multi-paradigm, general-purpose programming language designed for performance and safety, especially safe concurrency. Rust is syntactically similar to C++, but provides memory safety without using garbage collection. Rust has been called a systems programming language, and is supported by Mozilla.`,
  },
  {
    id: "webassembly.md",
    text: `WebAssembly (abbreviated Wasm) is a binary instruction format for a stack-based virtual machine. Wasm is designed as a portable compilation target for programming languages, enabling deployment on the web for client and server applications. WebAssembly has been embraced by all major browsers.`,
  },
  {
    id: "bm25.md",
    text: `In information retrieval, Okapi BM25 is a ranking function used by search engines to estimate the relevance of documents to a given search query. It is based on the probabilistic retrieval framework. BM25 and its newer variants represent state-of-the-art TF-IDF-like retrieval functions used in document retrieval.`,
  },
  {
    id: "lucene.md",
    text: `Apache Lucene is a free and open-source search engine software library, written in Java. It is supported by the Apache Software Foundation and is released under the Apache Software License. Lucene is widely used as the engine behind search platforms like Solr and Elasticsearch.`,
  },
  {
    id: "tantivy.md",
    text: `Tantivy is a full-text search engine library written in Rust. Inspired by Apache Lucene, Tantivy is designed to be embedded into applications. It supports BM25 ranking, faceted search, and aggregations, and is one of the fastest search engines available, with most of the heavy lifting done at compile time.`,
  },
  {
    id: "inverted-index.md",
    text: `An inverted index is a database index storing a mapping from content, such as words or numbers, to its locations in a table, document, or set of documents. The inverted index is the most popular data structure used in document retrieval systems, used at scale for search engines and full-text search libraries.`,
  },
  {
    id: "tokenization.md",
    text: `Tokenization in natural language processing is the process of breaking a stream of text into meaningful elements called tokens. The tokens are then passed on to other components such as parsers and search indexers. Modern tokenizers handle Unicode segmentation, normalization, and language-specific rules like stemming and stopword removal.`,
  },
  {
    id: "browser-storage.md",
    text: `Browsers offer several persistent storage APIs for web applications, including localStorage, sessionStorage, and IndexedDB. IndexedDB is a low-level API for client-side storage of significant amounts of structured data, including files and binary blobs. Search engines compiled to WebAssembly can persist their indexes in IndexedDB for instant offline retrieval.`,
  },
];
