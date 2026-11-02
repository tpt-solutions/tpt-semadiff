//! Phases 6 and 9: reports and rule-based summaries for common refactorings.

use tpt_semadiff_core::diff::{diff_files, Options};
use tpt_semadiff_core::parse::SourceLang;
use tpt_semadiff_core::render;
use tpt_semadiff_core::report::{OpKind, Report};
use tpt_semadiff_core::summarize::rules;

fn rust_diff(old: &str, new: &str) -> tpt_semadiff_core::diff::FileDiff {
    diff_files(old, new, SourceLang::Rust, Options::default()).expect("parse")
}

#[test]
fn rename_across_a_function_is_summarized_with_occurrences() {
    let old = "fn total(v: &[i32]) -> i32 {\n    let sum = 0;\n    for x in v {\n        sum += x;\n    }\n    sum\n}\n";
    let new = "fn total(v: &[i32]) -> i32 {\n    let acc = 0;\n    for x in v {\n        acc += x;\n    }\n    acc\n}\n";
    let lines = rules::summarize(&rust_diff(old, new));
    assert!(
        lines
            .iter()
            .any(|l| l == "Renamed `sum` to `acc` (3 occurrences)"),
        "{lines:?}"
    );
}

#[test]
fn reordered_imports_are_recognized() {
    let old = "use std::fmt;\nuse std::io;\n\nfn main() {}\n";
    let new = "use std::io;\nuse std::fmt;\n\nfn main() {}\n";
    let lines = rules::summarize(&rust_diff(old, new));
    assert!(
        lines.iter().any(|l| l.starts_with("Reordered imports")),
        "{lines:?}"
    );
}

#[test]
fn extracted_function_is_recognized() {
    let old = "fn process(x: i32) -> i32 {\n    let doubled = x * 2;\n    let shifted = doubled + 7;\n    shifted\n}\n";
    let new = "fn process(x: i32) -> i32 {\n    compute(x)\n}\n\nfn compute(x: i32) -> i32 {\n    let doubled = x * 2;\n    let shifted = doubled + 7;\n    shifted\n}\n";
    let lines = rules::summarize(&rust_diff(old, new));
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("Extracted `compute` from `process`")),
        "{lines:?}"
    );
}

#[test]
fn inlined_variable_is_recognized() {
    let old = "fn area(w: i32, h: i32) -> i32 {\n    let product = w * h;\n    product\n}\n";
    let new = "fn area(w: i32, h: i32) -> i32 {\n    w * h\n}\n";
    let lines = rules::summarize(&rust_diff(old, new));
    assert!(
        lines
            .iter()
            .any(|l| l == "Inlined variable `product` (`w * h`)"),
        "{lines:?}"
    );
}

#[test]
fn no_change_reads_as_no_change() {
    let src = "fn f() {}\n";
    assert_eq!(
        rules::summarize(&rust_diff(src, src)),
        vec!["No structural changes.".to_owned()]
    );
}

#[test]
fn unrecognized_change_falls_back_to_counts() {
    let old = "fn f() -> i32 {\n    1\n}\n";
    let new = "fn f() -> i32 {\n    2\n}\n";
    let lines = rules::summarize(&rust_diff(old, new));
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("updates"), "{lines:?}");
}

#[test]
fn report_groups_changes_by_function_and_counts_them() {
    let old = "fn a() -> i32 {\n    1\n}\n\nfn b() -> i32 {\n    2\n}\n";
    let new = "fn a() -> i32 {\n    10\n}\n\nfn b() -> i32 {\n    2\n}\n";
    let diff = rust_diff(old, new);
    let report = Report::from_diff(&diff, "old.rs", "new.rs", "chawathe");
    assert_eq!(report.counts.update, diff.ops.len());
    let names: Vec<_> = report.changed_functions().collect();
    assert_eq!(names, vec!["a"]);
    assert!(report.groups[0]
        .changes
        .iter()
        .all(|c| c.op == OpKind::Update));
}

#[test]
fn rendered_box_names_the_function_and_its_line_range() {
    let old = "fn a() -> i32 {\n    1\n}\n";
    let new = "fn a() -> i32 {\n    10\n}\n";
    let report = Report::from_diff(&rust_diff(old, new), "old.rs", "new.rs", "chawathe");
    let text = render::render_report(&report, false);
    assert!(text.contains("┌─ Function: a (Lines 1-3)"), "{text}");
    assert!(text.contains("[UPDATE]"), "{text}");
    assert!(!text.contains('\x1b'));
    let colored = render::render_report(&report, true);
    assert!(colored.contains("\x1b[36m[UPDATE]"));
}
