//! Renders entries in each output format. Single file pairs print the report alone;
//! directory and revision modes print a list of files.

use std::error::Error;
use std::fmt::Write;

use serde_json::{json, Value};
use tpt_semadiff_core::render;

use crate::pipeline::{Entry, Outcome};
use crate::OutFormat;

/// Prints `entries` in `format`. `multi` selects the file-list shape for JSON.
pub fn print(
    entries: &[Entry],
    format: OutFormat,
    color: bool,
    multi: bool,
) -> Result<(), Box<dyn Error>> {
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
    render::render_unified(&pair.old_name, &pair.new_name, &pair.old_src, &pair.new_src)
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
    Ok(serde_json::to_string_pretty(&json!({ "files": files }))?)
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
