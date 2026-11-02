//! Renders entries in each output format. Single file pairs print the report alone;
//! directory and revision modes print a list of files.

use std::error::Error;
use std::fmt::Write;

use serde_json::{json, Value};
use tpt_semadiff_core::render;
use tpt_semadiff_core::report::SCHEMA_VERSION;

use crate::pipeline::{Entry, Outcome};
use crate::OutFormat;

/// How much of each file to print. `Full` is the chosen `--out` format; the others are
/// brief forms for scripts and hooks, and they ignore `--out`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Brief {
    Full,
    /// One line per changed file, with its change counts.
    Stat,
    /// The name of each changed file.
    NameOnly,
    /// Nothing; only the exit status says whether anything changed.
    Quiet,
}

/// Prints `entries` in `format`, or in the brief form. `multi` selects the file-list shape
/// for JSON.
pub fn print(
    entries: &[Entry],
    format: OutFormat,
    color: bool,
    multi: bool,
    brief: Brief,
) -> Result<(), Box<dyn Error>> {
    match brief {
        Brief::Quiet => return Ok(()),
        Brief::NameOnly => {
            for entry in entries.iter().filter(|e| e.changed()) {
                println!("{}", entry.pair.new_name);
            }
            return Ok(());
        }
        Brief::Stat => {
            print!("{}", stat(entries));
            return Ok(());
        }
        Brief::Full => {}
    }
    match format {
        OutFormat::Text => print!("{}", text(entries, color)),
        OutFormat::Unified => {
            for entry in entries {
                warn_fallback(entry);
                print!("{}", unified(entry));
            }
        }
        OutFormat::Json => println!("{}", json(entries, multi)?),
        OutFormat::Markdown => print!("{}", markdown(entries)),
    }
    Ok(())
}

/// One line per changed file with its change counts, then a total. Files that did not parse
/// say so instead.
fn stat(entries: &[Entry]) -> String {
    let changed: Vec<&Entry> = entries.iter().filter(|e| e.changed()).collect();
    if changed.is_empty() {
        return "No semantic changes.\n".to_owned();
    }
    let width = changed
        .iter()
        .map(|e| e.pair.new_name.chars().count())
        .max()
        .unwrap_or(0);
    let mut out = String::new();
    for entry in &changed {
        let detail = match &entry.outcome {
            Outcome::Diffed { report, .. } => {
                let c = report.counts;
                format!(
                    "{} inserted, {} deleted, {} moved, {} updated",
                    c.insert, c.delete, c.moved, c.update
                )
            }
            Outcome::Unsupported | Outcome::Unparsed { .. } => format!(
                "not diffed structurally ({})",
                entry.fallback_reason().unwrap_or("unknown")
            ),
        };
        let _ = writeln!(out, "{:<width$} | {detail}", entry.pair.new_name);
    }
    let _ = writeln!(out, "{} file(s) changed", changed.len());
    out
}

fn warn_fallback(entry: &Entry) {
    if let Some(reason) = entry.fallback_reason() {
        eprintln!(
            "semadiff: falling back to a line diff for {} ({reason})",
            entry.pair.new_name
        );
    }
}

fn unified(entry: &Entry) -> String {
    let pair = &entry.pair;
    render::render_unified(
        &pair.old_name,
        &pair.new_name,
        &pair.old_src,
        &pair.new_src,
        pair.lang,
    )
}

fn text(entries: &[Entry], color: bool) -> String {
    if entries.is_empty() {
        return "No semantic changes.\n".to_owned();
    }
    let mut out = String::new();
    for (i, entry) in entries.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match &entry.outcome {
            Outcome::Diffed { report, .. } => out.push_str(&render::render_report(report, color)),
            Outcome::Unsupported | Outcome::Unparsed { .. } => {
                warn_fallback(entry);
                let _ = writeln!(
                    out,
                    "File: {} -> {}",
                    entry.pair.old_name, entry.pair.new_name
                );
                out.push_str(&unified(entry));
            }
        }
    }
    out
}

/// One report as JSON, or a `{"files": [...]}` envelope when several files are listed.
/// A file that did not parse appears in the list with an `error` field.
fn json(entries: &[Entry], multi: bool) -> Result<String, Box<dyn Error>> {
    if !multi {
        return match entries.first().map(|e| &e.outcome) {
            Some(Outcome::Diffed { report, .. }) => Ok(report.to_json()?),
            Some(_) => Err("no structural diff available".into()),
            None => Err("no file pair given".into()),
        };
    }
    let files = entries
        .iter()
        .map(|entry| match &entry.outcome {
            Outcome::Diffed { report, .. } => serde_json::to_value(report),
            Outcome::Unsupported | Outcome::Unparsed { .. } => Ok(json!({
                "old_name": entry.pair.old_name,
                "new_name": entry.pair.new_name,
                "error": entry.fallback_reason(),
            })),
        })
        .collect::<Result<Vec<Value>, _>>()?;
    Ok(serde_json::to_string_pretty(&json!({
        "schema_version": SCHEMA_VERSION,
        "files": files,
    }))?)
}

/// Markdown sections for a pull request comment. A file that did not parse gets its line
/// diff in a fenced block instead.
fn markdown(entries: &[Entry]) -> String {
    if entries.is_empty() {
        return "No semantic changes.\n".to_owned();
    }
    let mut out = String::new();
    for entry in entries {
        match &entry.outcome {
            Outcome::Diffed { report, .. } => out.push_str(&render::render_markdown(report)),
            Outcome::Unsupported | Outcome::Unparsed { .. } => {
                let _ = writeln!(out, "### `{}`\n", entry.pair.new_name);
                let _ = writeln!(
                    out,
                    "Not diffed structurally ({}); line diff:\n",
                    entry.fallback_reason().unwrap_or("unknown")
                );
                let mut diff = unified(entry);
                if !diff.ends_with('\n') {
                    diff.push('\n');
                }
                let _ = writeln!(out, "```diff\n{diff}```");
            }
        }
        out.push('\n');
    }
    out
}
