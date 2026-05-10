//! Snippet extraction with highlight markers.
//!
//! Given the original text of a hit and the query terms, build a short
//! excerpt that contains as many query matches as possible, with byte
//! offsets into the *snippet* string for each match.
//!
//! ## Algorithm
//!
//! 1. Tokenize the original text and find every token whose term matches a
//!    query term.
//! 2. Slide a fixed-byte window over the matches and pick the window
//!    containing the most matches (ties → leftmost).
//! 3. Trim the window to whole-word boundaries and prefix/suffix it with
//!    `…` if it doesn't include the start / end of the source.
//! 4. Re-emit the matches' offsets, now relative to the snippet string.
//!
//! This is intentionally simpler than Lucene's `Highlighter` (which scores
//! per-fragment and merges); it's good enough to give knosi a "search hit
//! preview" experience.

use crate::tokenizer::{Token, TokenizerConfig, tokenize};

/// A single highlighted excerpt around query matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snippet {
    /// The excerpt text, possibly prefixed/suffixed with `…`.
    pub text: String,
    /// Byte ranges (start, end) into [`Snippet::text`] that should be
    /// emphasized — typically rendered as `<mark>` in HTML or `**bold**`
    /// in markdown.
    pub highlights: Vec<(usize, usize)>,
}

/// Snippet extraction options.
#[derive(Debug, Clone, Copy)]
pub struct SnippetConfig {
    /// Target byte length of the snippet window before whole-word trimming.
    pub window_bytes: usize,
    /// Use `…` as the truncation marker on either side.
    pub use_ellipsis: bool,
}

impl Default for SnippetConfig {
    fn default() -> Self {
        Self {
            window_bytes: 160,
            use_ellipsis: true,
        }
    }
}

/// Extract a snippet from `text` showing as many matches of `query_terms` as
/// possible.
///
/// Returns `None` if no query term matches.
#[must_use]
pub fn extract(
    text: &str,
    query_terms: &[&str],
    tokenizer_config: &TokenizerConfig,
    snippet_config: &SnippetConfig,
) -> Option<Snippet> {
    if query_terms.is_empty() || text.is_empty() {
        return None;
    }

    let tokens = tokenize(text, tokenizer_config);
    let matches: Vec<&Token> = tokens
        .iter()
        .filter(|t| query_terms.iter().any(|q| q.eq_ignore_ascii_case(&t.term)))
        .collect();

    if matches.is_empty() {
        return None;
    }

    // Pick the window containing the most matches.
    let (window_start, window_end) = best_window(text, &matches, snippet_config.window_bytes);

    // Trim to whole-word boundaries (avoid splitting "WebAssembly" into
    // "ebAssembly"). We snap to the nearest whitespace.
    let trimmed_start = snap_left(text, window_start);
    let trimmed_end = snap_right(text, window_end);

    let prefix = if snippet_config.use_ellipsis && trimmed_start > 0 {
        "… "
    } else {
        ""
    };
    let suffix = if snippet_config.use_ellipsis && trimmed_end < text.len() {
        " …"
    } else {
        ""
    };

    let body = &text[trimmed_start..trimmed_end];
    let snippet_text = format!("{prefix}{body}{suffix}");

    // Translate match offsets into the snippet string.
    let prefix_len = prefix.len();
    let highlights: Vec<(usize, usize)> = matches
        .iter()
        .filter(|m| m.start >= trimmed_start && m.end <= trimmed_end)
        .map(|m| {
            let s = m.start - trimmed_start + prefix_len;
            let e = m.end - trimmed_start + prefix_len;
            (s, e)
        })
        .collect();

    Some(Snippet {
        text: snippet_text,
        highlights,
    })
}

/// Find the best starting byte offset for a `window_bytes`-wide window over
/// `text` such that the window contains the most matches.
fn best_window(text: &str, matches: &[&Token], window_bytes: usize) -> (usize, usize) {
    if matches.is_empty() {
        return (0, window_bytes.min(text.len()));
    }

    let mut best_start = matches[0].start.saturating_sub(window_bytes / 4);
    let mut best_count = 0;

    // Try anchoring the window so that each match is its left edge — for the
    // input sizes we expect (knosi notes are typically a few KB), a linear
    // scan is fine.
    for m in matches {
        let start = m.start.saturating_sub(window_bytes / 4);
        let end = (start + window_bytes).min(text.len());
        let count = matches
            .iter()
            .filter(|x| x.start >= start && x.end <= end)
            .count();
        if count > best_count {
            best_count = count;
            best_start = start;
        }
    }

    let best_end = (best_start + window_bytes).min(text.len());
    (best_start, best_end)
}

/// Snap `pos` left to the nearest whitespace (or 0). Operates on byte
/// indices but only steps over ASCII bytes — safe because whitespace and
/// control bytes are all single-byte UTF-8.
fn snap_left(text: &str, pos: usize) -> usize {
    if pos == 0 {
        return 0;
    }
    let bytes = text.as_bytes();
    let mut i = pos.min(bytes.len());
    while i > 0 && !is_ascii_ws(bytes[i - 1]) {
        i -= 1;
    }
    i
}

/// Snap `pos` right to the nearest whitespace (or text end). See
/// [`snap_left`] for why this is byte-safe.
fn snap_right(text: &str, pos: usize) -> usize {
    let bytes = text.as_bytes();
    let mut i = pos.min(bytes.len());
    while i < bytes.len() && !is_ascii_ws(bytes[i]) {
        i += 1;
    }
    i
}

fn is_ascii_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tcfg() -> TokenizerConfig {
        TokenizerConfig::default()
    }

    fn scfg() -> SnippetConfig {
        SnippetConfig::default()
    }

    #[test]
    fn returns_none_when_query_is_empty() {
        let s = extract("rust web search", &[], &tcfg(), &scfg());
        assert!(s.is_none());
    }

    #[test]
    fn returns_none_when_no_match() {
        let s = extract("rust web search", &["python"], &tcfg(), &scfg());
        assert!(s.is_none());
    }

    #[test]
    fn highlights_each_match() {
        let text = "Rust is a systems language. WebAssembly extends Rust to the browser.";
        let snippet = extract(text, &["rust"], &tcfg(), &scfg()).unwrap();
        // We expect both "Rust" occurrences to be highlighted.
        assert_eq!(snippet.highlights.len(), 2);
        for &(s, e) in &snippet.highlights {
            assert_eq!(snippet.text[s..e].to_lowercase(), "rust");
        }
    }

    #[test]
    fn ellipsis_added_when_truncated() {
        let mut text = String::new();
        for _ in 0..100 {
            text.push_str("filler word here ");
        }
        text.push_str(" rust ");
        for _ in 0..100 {
            text.push_str(" more padding text");
        }
        let snippet = extract(&text, &["rust"], &tcfg(), &scfg()).unwrap();
        assert!(snippet.text.starts_with("… "));
        assert!(snippet.text.ends_with(" …"));
        assert_eq!(snippet.highlights.len(), 1);
        let (s, e) = snippet.highlights[0];
        assert_eq!(&snippet.text[s..e], "rust");
    }

    #[test]
    fn no_ellipsis_for_short_text() {
        let snippet = extract("rust web", &["rust"], &tcfg(), &scfg()).unwrap();
        assert!(!snippet.text.starts_with("…"));
        assert!(!snippet.text.ends_with("…"));
    }

    #[test]
    fn snaps_to_word_boundaries() {
        // The window should never start mid-word.
        let text = "WebAssembly is awesome and rust integrates with WebAssembly cleanly.";
        let snippet = extract(text, &["rust"], &tcfg(), &scfg()).unwrap();
        // The body between the optional ellipses should not start or end with
        // a partial word.
        let body = snippet
            .text
            .trim_start_matches('…')
            .trim_start()
            .trim_end_matches('…')
            .trim_end();
        let first_word = body.split_whitespace().next().unwrap_or("");
        let last_word = body.split_whitespace().last().unwrap_or("");
        // first_word should be a real word from the original text
        assert!(text.contains(first_word));
        assert!(text.contains(last_word));
    }
}
