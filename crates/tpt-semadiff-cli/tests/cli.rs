//! Phase 7: CLI flags, output formats and exit codes, run against the built binary.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const OLD_RS: &str = "fn total(v: &[i32]) -> i32 {\n    let sum = 0;\n    for x in v {\n        sum += x;\n    }\n    sum\n}\n";
const RENAMED_RS: &str = "fn total(v: &[i32]) -> i32 {\n    let acc = 0;\n    for x in v {\n        acc += x;\n    }\n    acc\n}\n";

/// A scratch directory unique to one test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tpt-semadiff-cli-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    path
}

fn semadiff(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_semadiff"))
        .args(args)
        .output()
        .expect("run semadiff")
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("exit code")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn identical_files_exit_zero() {
    let dir = scratch("identical");
    let a = write(&dir, "a.rs", OLD_RS);
    let b = write(&dir, "b.rs", OLD_RS);
    let out = semadiff(&[a.to_str().unwrap(), b.to_str().unwrap()]);
    assert_eq!(code(&out), 0, "{}", stdout(&out));
    assert!(stdout(&out).contains("No semantic changes."));
}

#[test]
fn semantic_change_exits_one_and_prints_the_box() {
    let dir = scratch("rename");
    let a = write(&dir, "a.rs", OLD_RS);
    let b = write(&dir, "b.rs", RENAMED_RS);
    let out = semadiff(&["--color", "never", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert_eq!(code(&out), 1);
    let text = stdout(&out);
    assert!(text.contains("Function: total"), "{text}");
    assert!(text.contains("[UPDATE]"), "{text}");
}

#[test]
fn json_output_is_a_structured_report() {
    let dir = scratch("json");
    let a = write(&dir, "a.rs", OLD_RS);
    let b = write(&dir, "b.rs", RENAMED_RS);
    let out = semadiff(&["--out", "json", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert_eq!(code(&out), 1);
    let text = stdout(&out);
    assert!(text.trim_start().starts_with('{'), "{text}");
    assert!(text.contains("\"counts\""), "{text}");
    assert!(text.contains("\"engine\": \"chawathe\""), "{text}");
    assert!(text.contains("\"function\": \"total\""), "{text}");
}

#[test]
fn unified_output_is_a_line_diff() {
    let dir = scratch("unified");
    let a = write(&dir, "a.rs", OLD_RS);
    let b = write(&dir, "b.rs", RENAMED_RS);
    let out = semadiff(&["--out", "unified", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert_eq!(code(&out), 1);
    assert!(stdout(&out).contains("@@"), "{}", stdout(&out));
}

#[test]
fn ignore_formatting_makes_comment_only_edits_zero() {
    let dir = scratch("ignore-formatting");
    let a = write(&dir, "a.rs", "fn f() -> i32 {\n    // old\n    1\n}\n");
    let b = write(&dir, "b.rs", "fn f() -> i32 {\n    // new\n    1\n}\n");
    let without = semadiff(&[a.to_str().unwrap(), b.to_str().unwrap()]);
    assert_eq!(code(&without), 1);
    let with = semadiff(&[
        "--ignore-formatting",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
    ]);
    assert_eq!(code(&with), 0, "{}", stdout(&with));
}

#[test]
fn zhang_shasha_engine_is_selectable() {
    let dir = scratch("engine");
    let a = write(&dir, "a.rs", OLD_RS);
    let b = write(&dir, "b.rs", RENAMED_RS);
    let out = semadiff(&[
        "--out",
        "json",
        "--engine",
        "zhang-shasha",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 1);
    assert!(stdout(&out).contains("\"engine\": \"zhang-shasha\""));
}

#[test]
fn rules_summary_appears_above_the_boxes() {
    let dir = scratch("summary");
    let a = write(&dir, "a.rs", OLD_RS);
    let b = write(&dir, "b.rs", RENAMED_RS);
    let out = semadiff(&[
        "--color",
        "never",
        "--summary",
        "rules",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
    ]);
    let text = stdout(&out);
    assert!(text.contains("Summary:"), "{text}");
    assert!(text.contains("Renamed `sum` to `acc`"), "{text}");
}

#[test]
fn git_external_diff_form_uses_the_old_and_new_files() {
    let dir = scratch("external");
    let a = write(&dir, "old.rs", OLD_RS);
    let b = write(&dir, "new.rs", RENAMED_RS);
    // path old-file old-hex old-mode new-file new-hex new-mode
    let out = semadiff(&[
        "--color",
        "never",
        "--out",
        "json",
        "src/x.rs",
        a.to_str().unwrap(),
        "1111111",
        "100644",
        b.to_str().unwrap(),
        "2222222",
        "100644",
    ]);
    // Git's external protocol: changes are reported in the output, but the status is 0.
    assert_eq!(code(&out), 0, "{}", String::from_utf8_lossy(&out.stderr));
    assert!(stdout(&out).contains("\"counts\""));
}

#[test]
fn dev_null_means_an_empty_side() {
    let dir = scratch("devnull");
    let b = write(&dir, "new.rs", OLD_RS);
    let out = semadiff(&["--out", "json", "/dev/null", b.to_str().unwrap()]);
    assert_eq!(code(&out), 1, "{}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn unsupported_extension_falls_back_to_a_line_diff() {
    let dir = scratch("fallback");
    let a = write(&dir, "notes.txt", "one\ntwo\n");
    let b = write(&dir, "notes2.txt", "one\nthree\n");
    let out = semadiff(&[a.to_str().unwrap(), b.to_str().unwrap()]);
    assert_eq!(code(&out), 1);
    assert!(stdout(&out).contains("-two"), "{}", stdout(&out));
    assert!(String::from_utf8_lossy(&out.stderr).contains("falling back"));
}

#[test]
fn json_mode_refuses_to_fall_back() {
    let dir = scratch("json-unsupported");
    let a = write(&dir, "notes.txt", "one\n");
    let b = write(&dir, "notes2.txt", "two\n");
    let out = semadiff(&["--out", "json", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert_eq!(code(&out), 2);
}

#[test]
fn syntax_error_in_json_mode_is_an_error() {
    let dir = scratch("syntax");
    let a = write(&dir, "a.rs", "fn ok() {}\n");
    let b = write(&dir, "b.rs", "fn broken( {\n");
    let out = semadiff(&["--out", "json", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert_eq!(code(&out), 2);
    assert!(String::from_utf8_lossy(&out.stderr).contains("syntax error"));
}

#[test]
fn missing_file_is_an_error() {
    let out = semadiff(&["does-not-exist-a.rs", "does-not-exist-b.rs"]);
    assert_eq!(code(&out), 2);
    assert!(String::from_utf8_lossy(&out.stderr).contains("does-not-exist-a.rs"));
}

#[test]
fn wrong_number_of_files_is_an_error() {
    let dir = scratch("arity");
    let a = write(&dir, "a.rs", OLD_RS);
    let out = semadiff(&[a.to_str().unwrap()]);
    assert_eq!(code(&out), 2);
}
