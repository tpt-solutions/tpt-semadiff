//! Text renderers: boxed per-function summaries, and unified line diffs.

use std::fmt::Write;

use crate::report::{Group, OpKind, Report};

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
        out.push_str("Summary:\n");
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

/// Unified line diff of the two sources, as `diff -u` would print it.
pub fn render_unified(old_name: &str, new_name: &str, old_src: &str, new_src: &str) -> String {
    similar::TextDiff::from_lines(old_src, new_src)
        .unified_diff()
        .header(old_name, new_name)
        .to_string()
}
