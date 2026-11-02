//! Phases 2, 3 and 10: parsing, trivia normalization and per-language behaviour for
//! Rust, Python, JavaScript and Go.

use std::path::Path;

use tpt_semadiff_core::diff::{diff_files, Options};
use tpt_semadiff_core::normalize::{normalize, Segment, TriviaPolicy};
use tpt_semadiff_core::ops::EditOp;
use tpt_semadiff_core::parse::{ParseError, SourceLang};
use tpt_semadiff_core::tree::Tree;

fn keep() -> Options {
    Options::default()
}

fn strip() -> Options {
    Options {
        ignore_formatting: true,
        ..Options::default()
    }
}

fn ops_of(old: &str, new: &str, lang: SourceLang, options: Options) -> Vec<EditOp> {
    diff_files(old, new, lang, options).expect("parse").ops
}

#[test]
fn language_is_chosen_from_the_extension() {
    assert_eq!(
        SourceLang::from_path(Path::new("a/b.rs")),
        Some(SourceLang::Rust)
    );
    assert_eq!(
        SourceLang::from_path(Path::new("x.PY")),
        Some(SourceLang::Python)
    );
    assert_eq!(
        SourceLang::from_path(Path::new("x.jsx")),
        Some(SourceLang::JavaScript)
    );
    assert_eq!(
        SourceLang::from_path(Path::new("x.go")),
        Some(SourceLang::Go)
    );
    assert_eq!(SourceLang::from_path(Path::new("x.txt")), None);
    assert_eq!(SourceLang::from_path(Path::new("Makefile")), None);
}

#[test]
fn reconstruct_reproduces_the_source_exactly() {
    let samples = [
        (
            SourceLang::Rust,
            "// header\nfn main() {\n    let x = 1; /* inline */\n}\n",
        ),
        (
            SourceLang::Python,
            "def f(a):\n    \"\"\"Doc.\"\"\"\n    # note\n    return a  # trailing\n",
        ),
        (
            SourceLang::JavaScript,
            "// top\nfunction f(a) {\n  /* block */ return a + 1;\n}\n",
        ),
        (
            SourceLang::Go,
            "package main\n\n// Run it.\nfunc main() {\n\tx := 1 // one\n\t_ = x\n}\n",
        ),
    ];
    for (lang, src) in samples {
        for policy in [TriviaPolicy::Keep, TriviaPolicy::Strip] {
            let norm = normalize(src, lang, policy).expect("parse");
            assert_eq!(norm.reconstruct(), src, "{lang:?} {policy:?}");
        }
    }
}

#[test]
fn segments_cover_every_byte_once_and_in_order() {
    let src = "fn f() {\n    // c\n    1\n}\n";
    let norm = normalize(src, SourceLang::Rust, TriviaPolicy::Keep).unwrap();
    let mut pos = 0;
    for seg in norm.segments() {
        let range = match seg {
            Segment::Trivia(r) => r.clone(),
            Segment::Token(id) => {
                let s = norm.cst().span(*id);
                s.start_byte..s.end_byte
            }
        };
        assert_eq!(range.start, pos, "segments must be contiguous");
        pos = range.end;
    }
    assert_eq!(pos, src.len());
}

#[test]
fn pure_reformatting_has_no_effect_in_any_language() {
    let cases = [
        (
            SourceLang::Rust,
            "fn f(a: i32) -> i32 { a + 1 }\n",
            "fn f(a: i32) -> i32 {\n    a + 1\n}\n",
        ),
        (
            SourceLang::Python,
            "def f(a):\n    return a+1\n",
            "def f( a ):\n\n    return a + 1\n",
        ),
        (
            SourceLang::JavaScript,
            "function f(a){return a+1;}",
            "function f(a) {\n  return a + 1;\n}\n",
        ),
        (
            SourceLang::Go,
            "func f(a int) int { return a+1 }\n",
            "func f(a int) int {\n\treturn a + 1\n}\n",
        ),
    ];
    for (lang, old, new) in cases {
        assert!(ops_of(old, new, lang, keep()).is_empty(), "{lang:?} keep");
        assert!(ops_of(old, new, lang, strip()).is_empty(), "{lang:?} strip");
    }
}

#[test]
fn comment_only_edit_is_reported_unless_formatting_is_ignored() {
    let old = "fn f() -> i32 {\n    // old note\n    1\n}\n";
    let new = "fn f() -> i32 {\n    // new note\n    1\n}\n";
    assert!(!ops_of(old, new, SourceLang::Rust, keep()).is_empty());
    assert!(ops_of(old, new, SourceLang::Rust, strip()).is_empty());
}

#[test]
fn python_docstring_is_trivia_when_stripping() {
    let old = "def f(a):\n    \"\"\"Old doc.\"\"\"\n    return a\n";
    let new = "def f(a):\n    \"\"\"New doc.\"\"\"\n    return a\n";
    assert!(!ops_of(old, new, SourceLang::Python, keep()).is_empty());
    assert!(ops_of(old, new, SourceLang::Python, strip()).is_empty());
}

#[test]
fn python_code_change_is_still_reported_when_stripping() {
    let old = "def f(a):\n    return a + 1\n";
    let new = "def f(a):\n    return a - 1\n";
    let ops = ops_of(old, new, SourceLang::Python, strip());
    assert!(!ops.is_empty());
}

#[test]
fn javascript_operator_change_is_a_removed_and_an_added_token() {
    let old = "function f(a) { return a + 1; }";
    let new = "function f(a) { return a - 1; }";
    let diff = diff_files(old, new, SourceLang::JavaScript, keep()).unwrap();
    let removed: Vec<_> = diff
        .ops
        .iter()
        .filter_map(|op| match op {
            EditOp::Delete { value, .. } => value.as_deref(),
            _ => None,
        })
        .collect();
    let added: Vec<_> = diff
        .ops
        .iter()
        .filter_map(|op| match op {
            EditOp::Insert { value, .. } => value.as_deref(),
            _ => None,
        })
        .collect();
    assert!(removed.contains(&"+"), "{:?}", diff.ops);
    assert!(added.contains(&"-"), "{:?}", diff.ops);
}

#[test]
fn go_literal_change_is_an_update() {
    let old = "package main\n\nfunc f() int {\n\tx := 1\n\treturn x\n}\n";
    let new = "package main\n\nfunc f() int {\n\tx := 2\n\treturn x\n}\n";
    let ops = ops_of(old, new, SourceLang::Go, keep());
    assert!(
        ops.iter().any(|op| matches!(
            op,
            EditOp::Update { new_value: Some(v), .. } if v == "2"
        )),
        "{ops:?}"
    );
}

#[test]
fn syntax_errors_are_reported_with_a_line_number() {
    let result = diff_files("fn ok() {}\n", "fn broken( {\n", SourceLang::Rust, keep());
    match result {
        Err(ParseError::Syntax { line }) => assert_eq!(line, 1),
        other => panic!("expected a syntax error, got {other:?}"),
    }
}
