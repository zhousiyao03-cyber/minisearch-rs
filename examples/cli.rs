//! Minimal CLI for minisearch-rs.
//!
//! Usage:
//!   cli index <dir> [--out <file>]      Index every *.md file in <dir>.
//!   cli search <query> [--idx <file>]   Search a previously-built index.
//!   cli info  [--idx <file>]            Print index stats.
//!
//! Default index file: `./minisearch.bin`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use minisearch_rs::tokenizer::TokenizerConfig;
use minisearch_rs::{Engine, EngineConfig, Index, SnippetConfig, extract_snippet};

const DEFAULT_INDEX_PATH: &str = "minisearch.bin";

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let cmd = args.next();
    let rest: Vec<String> = args.collect();

    match cmd.as_deref() {
        Some("index") => cmd_index(&rest),
        Some("search") => cmd_search(&rest),
        Some("info") => cmd_info(&rest),
        Some("--help" | "-h") | None => {
            print_usage();
            Ok(())
        }
        Some(other) => {
            eprintln!("unknown subcommand: {other}\n");
            print_usage();
            std::process::exit(2);
        }
    }
}

fn print_usage() {
    eprintln!(
        "minisearch-rs CLI\n\n\
         USAGE:\n\
         \x20   cli index  <dir> [--out <file>]\n\
         \x20   cli search <query> [--idx <file>]\n\
         \x20   cli info   [--idx <file>]\n\n\
         Default index file: {DEFAULT_INDEX_PATH}",
    );
}

fn cmd_index(args: &[String]) -> Result<()> {
    let mut positional: Vec<&str> = Vec::new();
    let mut out_path = PathBuf::from(DEFAULT_INDEX_PATH);
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        match a.as_str() {
            "--out" => {
                let v = iter.next().context("--out requires a path")?;
                out_path = PathBuf::from(v);
            }
            other if other.starts_with("--") => bail!("unknown flag: {other}"),
            other => positional.push(other),
        }
    }
    let dir = positional.first().context("usage: index <dir>")?;
    let dir = Path::new(dir);
    if !dir.is_dir() {
        bail!("not a directory: {}", dir.display());
    }

    let mut engine = Engine::new(EngineConfig::default());
    let mut indexed = 0usize;
    for entry in walk_md(dir)? {
        let text =
            std::fs::read_to_string(&entry).with_context(|| format!("read {}", entry.display()))?;
        let id = entry
            .strip_prefix(dir)
            .unwrap_or(&entry)
            .to_string_lossy()
            .into_owned();
        engine.add_document(&id, &text)?;
        indexed += 1;
    }

    engine.index().save_to(&out_path)?;
    println!(
        "indexed {indexed} documents into {} ({} unique terms, avg len {:.1})",
        out_path.display(),
        engine.index().term_count(),
        engine.index().avg_doc_length(),
    );
    Ok(())
}

fn cmd_search(args: &[String]) -> Result<()> {
    let mut positional: Vec<String> = Vec::new();
    let mut idx_path = PathBuf::from(DEFAULT_INDEX_PATH);
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        match a.as_str() {
            "--idx" => {
                let v = iter.next().context("--idx requires a path")?;
                idx_path = PathBuf::from(v);
            }
            other if other.starts_with("--") => bail!("unknown flag: {other}"),
            other => positional.push(other.to_owned()),
        }
    }
    if positional.is_empty() {
        bail!("usage: search <query>");
    }
    let query = positional.join(" ");

    let index = Index::load_from(&idx_path)
        .with_context(|| format!("load index from {}", idx_path.display()))?;
    let engine = Engine::from_index(index, EngineConfig::default());

    let hits = engine.search(&query, 10);
    if hits.is_empty() {
        println!("no hits.");
        return Ok(());
    }

    let query_terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let term_refs: Vec<&str> = query_terms.iter().map(String::as_str).collect();

    println!("found {} hit(s):\n", hits.len());
    for (rank, hit) in hits.iter().enumerate() {
        println!(
            "  #{rank}  score={score:.4}  {id}",
            rank = rank + 1,
            score = hit.score,
            id = hit.doc_id,
        );
        // The index stores ids only; if the id is a path we can re-read for
        // a snippet preview. Otherwise we silently skip.
        if let Ok(text) = std::fs::read_to_string(&hit.doc_id) {
            if let Some(snip) = extract_snippet(
                &text,
                &term_refs,
                &TokenizerConfig::default(),
                &SnippetConfig::default(),
            ) {
                println!("       {}", snip.text);
            }
        }
    }
    Ok(())
}

fn cmd_info(args: &[String]) -> Result<()> {
    let mut idx_path = PathBuf::from(DEFAULT_INDEX_PATH);
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        match a.as_str() {
            "--idx" => {
                let v = iter.next().context("--idx requires a path")?;
                idx_path = PathBuf::from(v);
            }
            other => bail!("unknown arg: {other}"),
        }
    }
    let index = Index::load_from(&idx_path)?;
    println!("index:        {}", idx_path.display());
    println!("documents:    {}", index.doc_count());
    println!("unique terms: {}", index.term_count());
    println!("avg doc len:  {:.2}", index.avg_doc_length());
    Ok(())
}

/// Walk `dir` recursively and collect every file with a `.md` extension.
fn walk_md(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    walk(dir, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            walk(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "md") {
            out.push(path);
        }
    }
    Ok(())
}
