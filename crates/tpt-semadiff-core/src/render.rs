//! Text renderers: boxed per-function summaries, and unified line diffs.

use std::fmt::Write;

use crate::parse::{function_name, parse_tree, SourceLang};
use crate::report::{Group, OpKind, Report};
use crate::tree::Tree;

const RESET: &str = "\x1b[0m";

fn label(op: OpKind) -> &'static str {
    match op {
        OpKind::Insert => "INSERT",
        OpKind::Delete => "DELETE",
        OpKind::Move => "MOVED",
        OpKind::Update => "UPDATE",
    }
}

fn color(op: OpKind) -> &'static str {
    match op {
        OpKind::Insert => "\x1b[32m",
        OpKind::Delete => "\x1b[31m",
        OpKind::Move => "\x1b[33m",
        OpKind::Update => "\x1b[36m",
    }
}

/// Renders the report as boxed per-function blocks, one box per group.
pub fn render_report(report: &Report, color_output: bool) -> String {
    let mut out = String::new();
    if report.old_name == report.new_name {
        let _ = writeln!(out, "File: {}", report.new_name);
    } else {
        let _ = writeln!(out, "File: {} -> {}", report.old_name, report.new_name);
    }
    if !report.summaries.is_empty() {
        let _ = writeln!(out, "{}", summary_heading(report));
        for line in &report.summaries {
            let _ = writeln!(out, "  - {line}");
        }
    }
    if report.groups.is_empty() {
        out.push_str("No semantic changes.\n");
        return out;
    }
    for group in &report.groups {
        out.push_str(&render_group(group, color_output));
    }
    out
}

/// Heading for the summary block. Names the source, and the reason when the LLM fell back.
fn summary_heading(report: &Report) -> String {
    match (report.summary_source, &report.summary_fallback) {
        (Some("llm"), _) => "Summary (llm):".to_owned(),
        (_, Some(reason)) => format!("Summary (rules; LLM not used: {reason}):"),
        _ => "Summary:".to_owned(),
    }
}

/// Renders one group as a box. The top border names the function and its line range.
pub fn render_group(group: &Group, color_output: bool) -> String {
    let title = match &group.function {
        Some(name) => format!("Function: {name}"),
        None => "Top level".to_owned(),
    };
    let title = format!("{title} (Lines {}-{})", group.start_line, group.end_line);

    // Each body line is (plain text, text with colour codes), for measuring and printing.
    let body: Vec<(String, String)> = group
        .changes
        .iter()
        .map(|c| {
            let plain = format!("[{}] {}", label(c.op), c.description);
            let styled = if color_output {
                format!("{}[{}]{RESET} {}", color(c.op), label(c.op), c.description)
            } else {
                plain.clone()
            };
            (plain, styled)
        })
        .collect();

    let inner = body
        .iter()
        .map(|(plain, _)| plain.chars().count())
        .chain(std::iter::once(title.chars().count() + 3))
        .max()
        .unwrap_or(0);

    let mut out = String::new();
    let title_width = title.chars().count();
    let fill = (inner + 2).saturating_sub(title_width + 3);
    let _ = writeln!(out, "┌─ {title} {}┐", "─".repeat(fill));
    for (plain, styled) in &body {
        let pad = inner - plain.chars().count();
        let _ = writeln!(out, "│ {styled}{} │", " ".repeat(pad));
    }
    let _ = writeln!(out, "└{}┘", "─".repeat(inner + 2));
    out
}

/// GitHub-flavoured Markdown for one file, for pasting into a pull request comment: a
/// heading, the change counts, then one bullet per function with its changes nested below.
pub fn render_markdown(report: &Report) -> String {
    let mut out = String::new();
    if report.old_name == report.new_name {
        let _ = writeln!(out, "### `{}`\n", report.new_name);
    } else {
        let _ = writeln!(out, "### `{}` → `{}`\n", report.old_name, report.new_name);
    }
    for line in &report.summaries {
        let _ = writeln!(out, "- {line}");
    }
    if !report.summaries.is_empty() {
        out.push('\n');
    }
    let c = report.counts;
    let _ = writeln!(
        out,
        "**Changes:** {} inserted, {} deleted, {} moved, {} updated\n",
        c.insert, c.delete, c.moved, c.update
    );
    if report.groups.is_empty() {
        out.push_str("No semantic changes.\n");
        return out;
    }
    for group in &report.groups {
        let title = match &group.function {
            Some(name) => format!("`{name}`"),
            None => "Top level".to_owned(),
        };
        let _ = writeln!(
            out,
            "- {title} (lines {}-{})",
            group.start_line, group.end_line
        );
        for change in &group.changes {
            let line = change
                .new_line
                .or(change.old_line)
                .unwrap_or(group.start_line);
            let _ = writeln!(
                out,
                "  - **{}** line {line}: {}",
                label(change.op),
                change.description
            );
        }
    }
    out
}

/// Unified line diff of the two sources, as `diff -u` would print it. When `lang` is given
/// and both sides parse, each hunk header names the function its first change falls in,
/// the same way `git diff` does.
pub fn render_unified(
    old_name: &str,
    new_name: &str,
    old_src: &str,
    new_src: &str,
    lang: Option<SourceLang>,
) -> String {
    let diff = similar::TextDiff::from_lines(old_src, new_src);
    let hunks: Vec<_> = diff.unified_diff().iter_hunks().collect();
    if hunks.is_empty() {
        return String::new();
    }
    let old_fns = lang.and_then(|l| function_spans(old_src, l));
    let new_fns = lang.and_then(|l| function_spans(new_src, l));

    let mut out = format!("--- {old_name}\n+++ {new_name}\n");
    for hunk in hunks {
        let text = hunk.to_string();
        let (header, body) = text.split_once('\n').unwrap_or((&text, ""));
        out.push_str(header);
        let first = hunk
            .iter_changes()
            .find(|c| c.tag() != similar::ChangeTag::Equal);
        let function = match first {
            Some(change) => match (change.new_index(), change.old_index()) {
                (Some(i), _) => new_fns.as_deref().and_then(|f| innermost(f, i + 1)),
                (None, Some(i)) => old_fns.as_deref().and_then(|f| innermost(f, i + 1)),
                (None, None) => None,
            },
            None => None,
        };
        if let Some(name) = function {
            let _ = write!(out, " {name}");
        }
        out.push('\n');
        out.push_str(body);
    }
    out
}

/// A named function's name and 1-based line range.
struct FnSpan {
    name: String,
    start: usize,
    end: usize,
}

/// Every named function in `src`, or `None` when the source does not parse.
fn function_spans(src: &str, lang: SourceLang) -> Option<Vec<FnSpan>> {
    let tree = parse_tree(src, lang).ok()?;
    let spans = tree
        .preorder()
        .into_iter()
        .filter(|&n| lang.function_kinds().contains(&tree.kind(n)))
        .filter_map(|n| {
            let name = function_name(&tree, n)?.to_owned();
            let span = tree.span(n);
            Some(FnSpan {
                name,
                start: span.start_line,
                end: span.end_line,
            })
        })
        .collect();
    Some(spans)
}

/// Name of the innermost function that covers `line`.
fn innermost(fns: &[FnSpan], line: usize) -> Option<&str> {
    fns.iter()
        .filter(|f| f.start <= line && line <= f.end)
        .min_by_key(|f| f.end - f.start)
        .map(|f| f.name.as_str())
}
