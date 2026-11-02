//! Phase 6: the TUI layout, rendered into an in-memory buffer.

use ratatui::backend::TestBackend;
use ratatui::widgets::ListState;
use ratatui::Terminal;
use tpt_semadiff_core::diff::{diff_files, Options};
use tpt_semadiff_core::parse::SourceLang;
use tpt_semadiff_core::report::Report;
use tpt_semadiff_tui::draw;

fn report(old: &str, new: &str) -> Report {
    let diff = diff_files(old, new, SourceLang::Rust, Options::default()).unwrap();
    Report::from_diff(&diff, "old.rs", "new.rs", "chawathe")
}

fn screen_text(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    let area = buffer.area;
    let mut text = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            text.push_str(buffer[(x, y)].symbol());
        }
        text.push('\n');
    }
    text
}

#[test]
fn lists_changed_functions_and_shows_the_selected_one() {
    let report = report(
        "fn alpha() -> i32 {\n    1\n}\n\nfn beta() -> i32 {\n    2\n}\n",
        "fn alpha() -> i32 {\n    10\n}\n\nfn beta() -> i32 {\n    2\n}\n",
    );
    let mut terminal = Terminal::new(TestBackend::new(100, 20)).unwrap();
    let mut state = ListState::default();
    state.select(Some(0));
    terminal
        .draw(|frame| draw(frame, &report, &mut state))
        .unwrap();
    let text = screen_text(&terminal);
    assert!(text.contains("fn alpha"), "{text}");
    assert!(text.contains("[UPDATE]"), "{text}");
    assert!(text.contains("Lines 1-3"), "{text}");
}

#[test]
fn empty_report_says_so() {
    let report = report("fn a() {}\n", "fn a() {}\n");
    let mut terminal = Terminal::new(TestBackend::new(60, 10)).unwrap();
    let mut state = ListState::default();
    terminal
        .draw(|frame| draw(frame, &report, &mut state))
        .unwrap();
    assert!(screen_text(&terminal).contains("No semantic changes."));
}
