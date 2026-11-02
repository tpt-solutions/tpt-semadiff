//! The LLM summarizer's fallback behaviour, with a mock client so no network is needed.
//! Needs the `llm` feature: `cargo test -p tpt-semadiff-core --features llm`.

#![cfg(feature = "llm")]

use std::cell::Cell;

use tpt_semadiff_core::diff::{diff_files, FileDiff, Options};
use tpt_semadiff_core::parse::SourceLang;
use tpt_semadiff_core::report::Report;
use tpt_semadiff_core::summarize::llm::{self, LlmClient, LlmError, Source};
use tpt_semadiff_core::summarize::rules;

struct Mock {
    answer: Result<String, LlmError>,
    calls: Cell<usize>,
}

impl LlmClient for Mock {
    fn complete(&self, _system: &str, _prompt: &str) -> Result<String, LlmError> {
        self.calls.set(self.calls.get() + 1);
        self.answer.clone()
    }
}

fn llm_fixture() -> (FileDiff, Report) {
    let diff = diff_files(
        "fn a() -> i32 {\n    1\n}\n",
        "fn a() -> i32 {\n    2\n}\n",
        SourceLang::Rust,
        Options::default(),
    )
    .expect("parse");
    let report = Report::from_diff(&diff, "old.rs", "new.rs", "chawathe");
    (diff, report)
}

#[test]
fn llm_summary_is_used_when_the_model_answers() {
    let (diff, report) = llm_fixture();
    let mock = Mock {
        answer: Ok("- The return value of `a` changed from 1 to 2.\n- Nothing else.".to_owned()),
        calls: Cell::new(0),
    };
    let summary = llm::summarize_with_fallback(Some(&mock), &diff, &report);
    assert_eq!(summary.source, Source::Llm);
    assert_eq!(summary.fallback, None);
    assert_eq!(
        summary.lines,
        vec![
            "The return value of `a` changed from 1 to 2.".to_owned(),
            "Nothing else.".to_owned()
        ]
    );
    assert_eq!(mock.calls.get(), 1);
}

#[test]
fn llm_failure_falls_back_to_rules_and_says_why() {
    let (diff, report) = llm_fixture();
    let mock = Mock {
        answer: Err(LlmError("network down".to_owned())),
        calls: Cell::new(0),
    };
    let summary = llm::summarize_with_fallback(Some(&mock), &diff, &report);
    assert_eq!(summary.source, Source::Rules);
    assert_eq!(summary.lines, rules::summarize(&diff));
    let reason = summary.fallback.expect("a reason");
    assert!(reason.contains("network down"), "{reason}");
}

#[test]
fn empty_answer_falls_back_with_a_reason() {
    let (diff, report) = llm_fixture();
    let mock = Mock {
        answer: Ok("\n   \n".to_owned()),
        calls: Cell::new(0),
    };
    let summary = llm::summarize_with_fallback(Some(&mock), &diff, &report);
    assert_eq!(summary.source, Source::Rules);
    assert_eq!(
        summary.fallback.as_deref(),
        Some("the model returned no summary lines")
    );
}

#[test]
fn missing_client_uses_rules_without_calling_anything() {
    let (diff, report) = llm_fixture();
    let summary = llm::summarize_with_fallback(None, &diff, &report);
    assert_eq!(summary.source, Source::Rules);
    assert_eq!(
        summary.fallback.as_deref(),
        Some("ANTHROPIC_API_KEY is not set")
    );
}

#[test]
fn llm_prompt_lists_every_change_under_its_function() {
    let (_, report) = llm_fixture();
    let prompt = llm::build_prompt(&report);
    assert!(prompt.contains("In a:"), "{prompt}");
    assert!(prompt.contains("Update"), "{prompt}");
}

#[test]
fn parse_lines_strips_bullets_and_blanks() {
    let lines = llm::parse_lines("\n* one\n• two\n\n- three\n- four\n- five\n- six\n");
    assert_eq!(lines, vec!["one", "two", "three", "four", "five"]);
}
