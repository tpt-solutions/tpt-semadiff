//! Phase 15: `--ignore-formatting` guarantees, anonymous functions, function-aware unified
//! headers, and the initializer-based inline rule.

use tpt_semadiff_core::diff::{diff_files, Options};
use tpt_semadiff_core::parse::SourceLang;
use tpt_semadiff_core::render;
use tpt_semadiff_core::report::Report;
use tpt_semadiff_core::summarize::rules;

fn ignoring(old: &str, new: &str, lang: SourceLang) -> bool {
    let options = Options {
        ignore_formatting: true,
        ..Options::default()
    };
    diff_files(old, new, lang, options)
        .expect("parse")
        .ops
        .is_empty()
}

fn strict(old: &str, new: &str, lang: SourceLang) -> bool {
    diff_files(old, new, lang, Options::default())
        .expect("parse")
        .ops
        .is_empty()
}

#[test]
fn trailing_commas_in_lists_and_arguments_are_ignored() {
    let old = "fn f() {\n    g(a, b,);\n    let v = [1, 2,];\n}\n";
    let new = "fn f() {\n    g(a, b);\n    let v = [1, 2];\n}\n";
    assert!(ignoring(old, new, SourceLang::Rust));
    assert!(!strict(old, new, SourceLang::Rust));
}

#[test]
fn a_trailing_comma_that_makes_a_python_tuple_is_kept() {
    let old = "x = (1,)\n";
    let new = "x = (1)\n";
    assert!(!ignoring(old, new, SourceLang::Python));
}

#[test]
fn a_javascript_array_hole_is_not_mistaken_for_a_trailing_comma() {
    let old = "const a = [1,,];\n";
    let new = "const a = [1,];\n";
    assert!(!ignoring(old, new, SourceLang::JavaScript));
}

#[test]
fn redundant_parentheses_are_ignored() {
    let old = "fn f(a: i32) -> i32 {\n    (a)\n}\n";
    let new = "fn f(a: i32) -> i32 {\n    a\n}\n";
    assert!(ignoring(old, new, SourceLang::Rust));
}

#[test]
fn parentheses_that_change_the_grouping_are_kept() {
    let old = "fn f(a: i32, b: i32, c: i32) -> i32 {\n    (a + b) * c\n}\n";
    let new = "fn f(a: i32, b: i32, c: i32) -> i32 {\n    a + b * c\n}\n";
    assert!(!ignoring(old, new, SourceLang::Rust));
}

#[test]
fn anonymous_closures_are_grouped_under_the_enclosing_function() {
    let old = "fn outer(v: &[i32]) -> Vec<i32> {\n    v.iter().map(|x| x + 1).collect()\n}\n";
    let new = "fn outer(v: &[i32]) -> Vec<i32> {\n    v.iter().map(|x| x + 2).collect()\n}\n";
    let diff = diff_files(old, new, SourceLang::Rust, Options::default()).expect("parse");
    let report = Report::from_diff(&diff, "old.rs", "new.rs", "chawathe");
    let names: Vec<_> = report.changed_functions().collect();
    assert_eq!(names, vec!["outer"], "{report:?}");
}

#[test]
fn a_closure_bound_to_a_name_is_grouped_under_that_name() {
    let old = "fn outer() -> i32 {\n    let f = |x: i32| x + 1;\n    f(1)\n}\n";
    let new = "fn outer() -> i32 {\n    let f = |x: i32| x + 2;\n    f(1)\n}\n";
    let diff = diff_files(old, new, SourceLang::Rust, Options::default()).expect("parse");
    let report = Report::from_diff(&diff, "old.rs", "new.rs", "chawathe");
    let names: Vec<_> = report.changed_functions().collect();
    assert_eq!(names, vec!["f"], "{report:?}");
}

#[test]
fn javascript_arrow_functions_are_named_by_their_binding() {
    let old = "const double = (x) => x * 2;\n";
    let new = "const double = (x) => x * 3;\n";
    let diff = diff_files(old, new, SourceLang::JavaScript, Options::default()).expect("parse");
    let report = Report::from_diff(&diff, "old.js", "new.js", "chawathe");
    let names: Vec<_> = report.changed_functions().collect();
    assert_eq!(names, vec!["double"]);
}

#[test]
fn unified_hunks_name_the_enclosing_function() {
    let old = "fn a() -> i32 {\n    1\n}\n\nfn b() -> i32 {\n    2\n}\n";
    let new = "fn a() -> i32 {\n    1\n}\n\nfn b() -> i32 {\n    3\n}\n";
    let text = render::render_unified("old.rs", "new.rs", old, new, Some(SourceLang::Rust));
    assert!(text.starts_with("--- old.rs\n+++ new.rs\n"), "{text}");
    assert!(
        text.lines()
            .any(|l| l.starts_with("@@") && l.ends_with(" b")),
        "{text}"
    );
}

#[test]
fn unified_without_a_language_keeps_plain_headers() {
    let old = "a\n";
    let new = "b\n";
    let text = render::render_unified("old.txt", "new.txt", old, new, None);
    assert!(text.lines().any(|l| l == "@@ -1 +1 @@"), "{text}");
}

#[test]
fn shebang_lines_pick_a_language() {
    assert_eq!(
        SourceLang::from_shebang("#!/usr/bin/env python3"),
        Some(SourceLang::Python)
    );
    assert_eq!(
        SourceLang::from_shebang("#!/usr/bin/env -S node --no-warnings"),
        Some(SourceLang::JavaScript)
    );
    assert_eq!(
        SourceLang::from_shebang("#!/usr/bin/python3.11"),
        Some(SourceLang::Python)
    );
    assert_eq!(SourceLang::from_shebang("#!/bin/sh"), None);
    assert_eq!(SourceLang::from_shebang("print('hi')"), None);
}

#[test]
fn language_names_round_trip() {
    for lang in SourceLang::ALL {
        assert_eq!(SourceLang::from_name(lang.name()), Some(lang));
    }
    assert_eq!(SourceLang::from_name("Rust"), Some(SourceLang::Rust));
    assert_eq!(SourceLang::from_name("cobol"), None);
}

#[test]
fn unified_of_identical_sources_is_empty() {
    let src = "fn a() {}\n";
    assert_eq!(
        render::render_unified("a", "b", src, src, Some(SourceLang::Rust)),
        ""
    );
}

#[test]
fn inline_rule_reads_the_initializer_from_the_tree() {
    let old = "def area(w, h):\n    product: int = w * h\n    return product\n";
    let new = "def area(w, h):\n    return w * h\n";
    let diff = diff_files(old, new, SourceLang::Python, Options::default()).expect("parse");
    let lines = rules::summarize(&diff);
    assert!(
        lines
            .iter()
            .any(|l| l == "Inlined variable `product` (`w * h`)"),
        "{lines:?}"
    );
}

#[test]
fn inline_rule_handles_go_short_declarations() {
    let old =
        "package main\n\nfunc area(w, h int) int {\n\tproduct := w * h\n\treturn product\n}\n";
    let new = "package main\n\nfunc area(w, h int) int {\n\treturn w * h\n}\n";
    let diff = diff_files(old, new, SourceLang::Go, Options::default()).expect("parse");
    let lines = rules::summarize(&diff);
    assert!(
        lines
            .iter()
            .any(|l| l == "Inlined variable `product` (`w * h`)"),
        "{lines:?}"
    );
}
