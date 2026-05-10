//! Unicode-aware tokenizer with optional stopword filtering.
//!
//! The tokenizer's job is to turn a raw `&str` into a stream of normalized
//! terms suitable for indexing or querying. We aim for the **same** function
//! to be used at index time and query time so that recall is symmetric.
//!
//! ## Pipeline
//!
//! 1. Split the input into "word-like" units using
//!    [`unicode-segmentation`](https://crates.io/crates/unicode-segmentation)
//!    so that we handle non-ASCII text correctly.
//! 2. Lowercase each unit (ASCII for now; full Unicode case-folding is a
//!    future upgrade).
//! 3. Drop units that are pure punctuation / whitespace.
//! 4. Optionally drop English stopwords (the common 25-or-so words that carry
//!    almost no signal in a BM25 ranking).
//!
//! Each emitted [`Token`] knows its byte offset in the original input so that
//! downstream code (e.g. snippet extraction) can locate matches without
//! re-scanning the source.

use unicode_segmentation::UnicodeSegmentation;

/// A single token produced by the tokenizer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// The normalized term (lowercased, stopword-filtered).
    pub term: String,
    /// Byte offset of the original word in the input string.
    pub start: usize,
    /// Byte offset *after* the original word in the input string.
    pub end: usize,
}

/// Configuration for [`tokenize`].
#[derive(Debug, Clone)]
pub struct TokenizerConfig {
    /// Drop tokens that are pure punctuation or whitespace. Always true for
    /// search use cases; exposed for tests.
    pub drop_punct: bool,
    /// Drop English stopwords like "the", "is", "a".
    pub drop_stopwords: bool,
    /// Minimum byte length for a token to be kept.
    pub min_len: usize,
}

impl Default for TokenizerConfig {
    fn default() -> Self {
        Self {
            drop_punct: true,
            drop_stopwords: true,
            min_len: 1,
        }
    }
}

/// A small, deliberately *not* exhaustive English stopword list.
///
/// We keep it short on purpose: BM25 already down-weights very common terms
/// via the IDF factor, so an aggressive stopword list mostly hurts recall on
/// short queries (e.g. "to be or not to be").
const STOPWORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "if", "in", "into", "is", "it",
    "no", "not", "of", "on", "or", "such", "that", "the", "their", "then", "there", "these",
    "they", "this", "to", "was", "will", "with",
];

fn is_stopword(term: &str) -> bool {
    STOPWORDS.binary_search(&term).is_ok()
}

fn is_wordlike(s: &str) -> bool {
    s.chars().any(char::is_alphanumeric)
}

/// Tokenize `input` according to `config`.
///
/// The returned tokens are in document order and include byte offsets into
/// the original string.
#[must_use]
pub fn tokenize(input: &str, config: &TokenizerConfig) -> Vec<Token> {
    input
        .unicode_word_indices()
        .filter_map(|(start, word)| {
            if config.drop_punct && !is_wordlike(word) {
                return None;
            }
            if word.len() < config.min_len {
                return None;
            }
            let term = word.to_lowercase();
            if config.drop_stopwords && is_stopword(&term) {
                return None;
            }
            Some(Token {
                term,
                start,
                end: start + word.len(),
            })
        })
        .collect()
}

/// Tokenize using the default configuration.
#[must_use]
pub fn tokenize_default(input: &str) -> Vec<Token> {
    tokenize(input, &TokenizerConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terms(tokens: &[Token]) -> Vec<&str> {
        tokens.iter().map(|t| t.term.as_str()).collect()
    }

    #[test]
    fn stopwords_list_is_sorted() {
        // We binary-search it, so it must stay sorted.
        let mut sorted = STOPWORDS.to_vec();
        sorted.sort_unstable();
        assert_eq!(STOPWORDS, sorted.as_slice());
    }

    #[test]
    fn lowercases_and_drops_punctuation() {
        let tokens = tokenize_default("Rust is GREAT, isn't it?");
        // "is", "it" are stopwords. "isn't" splits into "isn" + "t" by
        // unicode_word_indices; "t" is min_len=1 OK but is a stopword? no — kept.
        assert!(terms(&tokens).contains(&"rust"));
        assert!(terms(&tokens).contains(&"great"));
        assert!(!terms(&tokens).contains(&"is"));
        assert!(!terms(&tokens).contains(&"it"));
    }

    #[test]
    fn preserves_latin_unicode_words() {
        // Latin-script Unicode (accents) should round-trip cleanly.
        let tokens = tokenize_default("café résumé naïve");
        let t = terms(&tokens);
        assert!(t.contains(&"café"));
        assert!(t.contains(&"résumé"));
        assert!(t.contains(&"naïve"));
    }

    #[test]
    fn cjk_splits_per_codepoint_for_now() {
        // The Unicode Text Segmentation algorithm has no rule for CJK word
        // boundaries (no spaces, no language model), so each ideograph becomes
        // its own token. A future upgrade can plug in a CJK segmenter
        // (e.g. jieba-rs) to produce "北京" as a single token.
        let tokens = tokenize_default("北京 search");
        let t = terms(&tokens);
        assert!(t.contains(&"北"));
        assert!(t.contains(&"京"));
        assert!(t.contains(&"search"));
    }

    #[test]
    fn byte_offsets_round_trip() {
        let input = "Rust and WebAssembly";
        let tokens = tokenize_default(input);
        for tok in &tokens {
            let original = &input[tok.start..tok.end];
            assert_eq!(original.to_lowercase(), tok.term);
        }
    }

    #[test]
    fn empty_input_yields_no_tokens() {
        assert!(tokenize_default("").is_empty());
        assert!(tokenize_default("   \n\t  ").is_empty());
        assert!(tokenize_default("!!! ??? ...").is_empty());
    }

    #[test]
    fn stopwords_can_be_disabled() {
        let cfg = TokenizerConfig {
            drop_stopwords: false,
            ..TokenizerConfig::default()
        };
        let tokens = tokenize("the rust", &cfg);
        assert_eq!(terms(&tokens), vec!["the", "rust"]);
    }

    use proptest::prop_assert_eq;
    use proptest::proptest;

    proptest! {
        /// Every emitted token's byte slice must round-trip to its term
        /// (case-insensitively).
        #[test]
        fn prop_offsets_round_trip(s in ".{0,200}") {
            let tokens = tokenize_default(&s);
            for tok in tokens {
                let slice = &s[tok.start..tok.end];
                prop_assert_eq!(slice.to_lowercase(), tok.term);
            }
        }
    }
}
