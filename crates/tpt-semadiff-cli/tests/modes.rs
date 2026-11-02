//! Phase 14: directory and git revision modes, the CI gate, Markdown output, `git-setup`,
//! `completions` and `doctor`, run against the built binary.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const OLD_RS: &str = "fn total(v: &[i32]) -> i32 {\n    let sum = 0;\n    for x in v {\n        sum += x;\n    }\n    sum\n}\n";
const RENAMED_RS: &str = "fn total(v: &[i32]) -> i32 {\n    let acc = 0;\n    for x in v {\n        acc += x;\n    }\n    acc\n}\n";
const TWO_FNS_OLD: &str = "fn a() -> i32 { 1 }\nfn b() -> i32 { 2 }\n";
const TWO_FNS_NEW: &str = "fn a() -> i32 { 10 }\nfn b() -> i32 { 20 }\n";

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("tpt-semadiff-modes-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
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

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Runs git in `dir` with a fixed identity, so commits work on any machine.
fn git(dir: &Path, args: &[&str]) -> Output {
    let out = Command::new("git")
        .current_dir(dir)
        .args(["-c", "user.name=test", "-c", "user.email=test@example.com"])
        .args(["-c", "core.autocrlf=false"])
        .args(args)
        .output()
        .expect("run git");
    assert!(out.status.success(), "git {args:?}: {}", stderr(&out));
    out
}

/// A repository with two commits: `calc.rs` changes in the second, and `notes.txt` is
/// not a supported language.
fn repo_with_history(name: &str) -> PathBuf {
    let dir = scratch(name);
    git(&dir, &["init", "-q"]);
    write(&dir, "calc.rs", OLD_RS);
    write(&dir, "notes.txt", "one\n");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    write(&dir, "calc.rs", RENAMED_RS);
    write(&dir, "notes.txt", "two\n");
    git(&dir, &["commit", "-q", "-am", "second"]);
    dir
}

#[test]
fn directory_mode_lists_only_the_files_that_changed() {
    let dir = scratch("dirs");
    let (old, new) = (dir.join("old"), dir.join("new"));
    write(&old, "src/calc.rs", OLD_RS);
    write(&new, "src/calc.rs", RENAMED_RS);
    write(&old, "src/same.rs", OLD_RS);
    write(&new, "src/same.rs", OLD_RS);
    write(&new, "src/added.rs", "fn fresh() {}\n");
    write(&old, "readme.md", "unsupported\n");

    let out = semadiff(&[
        "--out",
        "json",
        old.to_str().unwrap(),
        new.to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("\"files\""), "{text}");
    assert!(text.contains("calc.rs"), "{text}");
    assert!(text.contains("added.rs"), "{text}");
    assert!(
        !text.contains("same.rs"),
        "unchanged files must not be listed: {text}"
    );
    assert!(
        !text.contains("readme.md"),
        "unsupported files must not be listed: {text}"
    );
}

#[test]
fn identical_directories_exit_zero() {
    let dir = scratch("dirs-identical");
    write(&dir.join("a"), "x.rs", OLD_RS);
    write(&dir.join("b"), "x.rs", OLD_RS);
    let out = semadiff(&[
        dir.join("a").to_str().unwrap(),
        dir.join("b").to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 0, "{}", stdout(&out));
    assert!(stdout(&out).contains("No semantic changes."));
}

#[test]
fn directory_compared_with_a_file_is_an_error() {
    let dir = scratch("dir-file");
    write(&dir, "a.rs", OLD_RS);
    let out = semadiff(&[dir.to_str().unwrap(), dir.join("a.rs").to_str().unwrap()]);
    assert_eq!(code(&out), 2);
}

#[test]
fn tui_refuses_more_than_one_file() {
    let dir = scratch("tui-dirs");
    write(&dir.join("a"), "x.rs", OLD_RS);
    write(&dir.join("b"), "x.rs", RENAMED_RS);
    let out = semadiff(&[
        "--tui",
        dir.join("a").to_str().unwrap(),
        dir.join("b").to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 2);
}

#[test]
fn json_lists_an_unparseable_file_with_its_error() {
    let dir = scratch("dirs-broken");
    write(&dir.join("a"), "bad.rs", "fn ok() {}\n");
    write(&dir.join("b"), "bad.rs", "fn broken( {\n");
    let out = semadiff(&[
        "--out",
        "json",
        dir.join("a").to_str().unwrap(),
        dir.join("b").to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    assert!(stdout(&out).contains("\"error\""), "{}", stdout(&out));
}

#[test]
fn markdown_output_matches_the_snapshot() {
    let dir = scratch("markdown");
    write(&dir, "a.rs", OLD_RS);
    write(&dir, "b.rs", RENAMED_RS);
    let out = Command::new(env!("CARGO_BIN_EXE_semadiff"))
        .current_dir(&dir)
        .args(["--out", "markdown", "a.rs", "b.rs"])
        .output()
        .unwrap();
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    let expected = "\
### `a.rs` → `b.rs`

**Changes:** 0 inserted, 0 deleted, 0 moved, 3 updated

- `total` (lines 1-7)
  - **UPDATE** line 2: Changed `sum` to `acc`
  - **UPDATE** line 4: Changed `sum` to `acc`
  - **UPDATE** line 6: Changed `sum` to `acc`

";
    assert_eq!(stdout(&out), expected);
}

#[test]
fn max_functions_gate_fails_when_too_many_functions_change() {
    let dir = scratch("gate-max");
    let a = write(&dir, "a.rs", TWO_FNS_OLD);
    let b = write(&dir, "b.rs", TWO_FNS_NEW);
    let paths = [a.to_str().unwrap(), b.to_str().unwrap()];

    let over = semadiff(&["--max-functions", "1", paths[0], paths[1]]);
    assert_eq!(code(&over), 1);
    assert!(
        stderr(&over).contains("over the limit of 1"),
        "{}",
        stderr(&over)
    );

    let within = semadiff(&["--max-functions", "2", paths[0], paths[1]]);
    assert!(
        !stderr(&within).contains("over the limit"),
        "{}",
        stderr(&within)
    );
}

#[test]
fn max_functions_counts_distinct_names_across_files() {
    let dir = scratch("gate-dirs");
    write(&dir.join("a"), "one.rs", "fn shared() -> i32 { 1 }\n");
    write(&dir.join("b"), "one.rs", "fn shared() -> i32 { 2 }\n");
    write(&dir.join("a"), "two.rs", "fn shared() -> i32 { 3 }\n");
    write(&dir.join("b"), "two.rs", "fn shared() -> i32 { 4 }\n");
    let out = semadiff(&[
        "--max-functions",
        "1",
        dir.join("a").to_str().unwrap(),
        dir.join("b").to_str().unwrap(),
    ]);
    assert!(!stderr(&out).contains("over the limit"), "{}", stderr(&out));
}

#[test]
fn fail_on_unparseable_exits_one_and_names_the_file() {
    let dir = scratch("gate-parse");
    let a = write(&dir, "a.rs", "fn ok() {}\n");
    let b = write(&dir, "b.rs", "fn broken( {\n");
    let out = semadiff(&[
        "--fail-on-unparseable",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 1);
    assert!(
        stderr(&out).contains("could not be parsed"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn typescript_files_are_diffed_structurally() {
    let dir = scratch("typescript");
    let a = write(
        &dir,
        "a.ts",
        "function f(a: number): number { return a; }\n",
    );
    let b = write(
        &dir,
        "b.ts",
        "function f(a: number): number { return a + 1; }\n",
    );
    let out = semadiff(&["--out", "json", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("\"language\": \"typescript\""),
        "{}",
        stdout(&out)
    );
}

#[test]
fn revision_mode_compares_a_revision_with_the_working_tree() {
    let dir = repo_with_history("rev-worktree");
    write(&dir, "calc.rs", TWO_FNS_NEW);
    let out = Command::new(env!("CARGO_BIN_EXE_semadiff"))
        .current_dir(&dir)
        .args(["--color", "never", "--rev", "HEAD~1"])
        .output()
        .unwrap();
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("HEAD~1:calc.rs -> calc.rs"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn revision_range_compares_two_revisions() {
    let dir = repo_with_history("rev-range");
    let out = Command::new(env!("CARGO_BIN_EXE_semadiff"))
        .current_dir(&dir)
        .args(["--out", "json", "--rev", "HEAD~1..HEAD"])
        .output()
        .unwrap();
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("\"files\""), "{text}");
    assert!(text.contains("HEAD:calc.rs"), "{text}");
    // notes.txt changed, but it is not a supported language, so it is skipped.
    assert!(!text.contains("notes.txt"), "{text}");
    assert!(stderr(&out).contains("skipped 1"), "{}", stderr(&out));
}

#[test]
fn revision_range_touching_only_unsupported_files_exits_zero() {
    let dir = repo_with_history("rev-unsupported");
    write(&dir, "notes.txt", "three\n");
    git(&dir, &["commit", "-q", "-am", "notes only"]);
    let out = Command::new(env!("CARGO_BIN_EXE_semadiff"))
        .current_dir(&dir)
        .args(["--color", "never", "--rev", "HEAD~1..HEAD"])
        .output()
        .unwrap();
    assert_eq!(code(&out), 0, "{}", stderr(&out));
    assert!(
        stdout(&out).contains("No semantic changes."),
        "{}",
        stdout(&out)
    );
    assert!(stderr(&out).contains("skipped 1"), "{}", stderr(&out));
}

#[test]
fn rev_conflicts_with_file_arguments() {
    let dir = scratch("rev-conflict");
    let a = write(&dir, "a.rs", OLD_RS);
    let b = write(&dir, "b.rs", RENAMED_RS);
    let out = semadiff(&["--rev", "HEAD", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert_eq!(code(&out), 2);
}

#[test]
fn git_setup_writes_idempotently_and_uninstalls_locally() {
    let dir = scratch("git-setup-local");
    git(&dir, &["init", "-q"]);
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_semadiff"))
            .current_dir(&dir)
            .arg("git-setup")
            .args(args)
            .output()
            .unwrap()
    };
    let first = run(&["--local", "--external"]);
    assert_eq!(code(&first), 0, "{}", stderr(&first));
    let second = run(&["--local", "--external"]);
    assert_eq!(code(&second), 0, "{}", stderr(&second));
    assert!(
        stdout(&second).contains("already set"),
        "{}",
        stdout(&second)
    );

    let cmd = git(
        &dir,
        &["config", "--local", "--get", "difftool.semadiff.cmd"],
    );
    assert!(stdout_of(&cmd).contains("$LOCAL"), "{}", stdout_of(&cmd));
    let external = git(&dir, &["config", "--local", "--get", "diff.external"]);
    assert_eq!(stdout_of(&external).trim(), "semadiff");

    let removed = run(&["--local", "--uninstall"]);
    assert_eq!(code(&removed), 0, "{}", stderr(&removed));
    let gone = Command::new("git")
        .current_dir(&dir)
        .args(["config", "--local", "--get", "difftool.semadiff.cmd"])
        .output()
        .unwrap();
    assert_eq!(code(&gone), 1);
}

#[test]
fn git_setup_global_uses_the_config_file_git_points_at() {
    let dir = scratch("git-setup-global");
    let config = dir.join("gitconfig");
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_semadiff"))
            .env("GIT_CONFIG_GLOBAL", &config)
            .arg("git-setup")
            .args(args)
            .output()
            .unwrap()
    };
    assert_eq!(code(&run(&[])), 0);
    assert_eq!(code(&run(&[])), 0);
    let text = fs::read_to_string(&config).unwrap();
    assert_eq!(text.matches("cmd =").count(), 1, "{text}");
    assert_eq!(code(&run(&["--uninstall"])), 0);
    let after = fs::read_to_string(&config).unwrap_or_default();
    assert!(!after.contains("semadiff"), "{after}");
}

#[test]
fn completions_print_a_script_for_the_shell() {
    let out = semadiff(&["completions", "bash"]);
    assert_eq!(code(&out), 0);
    assert!(stdout(&out).contains("semadiff"), "{}", stdout(&out));
}

#[test]
fn doctor_checks_each_grammar() {
    let out = semadiff(&["doctor"]);
    let text = stdout(&out);
    for lang in ["rust", "python", "javascript", "typescript", "tsx", "go"] {
        assert!(text.contains(lang), "{text}");
    }
    assert!(
        !text.contains("ANTHROPIC_API_KEY=") && !text.contains("sk-"),
        "{text}"
    );
}

#[test]
fn summary_is_off_when_piped_unless_asked_for() {
    let dir = scratch("summary-default");
    let a = write(&dir, "a.rs", OLD_RS);
    let b = write(&dir, "b.rs", RENAMED_RS);
    let piped = semadiff(&["--color", "never", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert!(!stdout(&piped).contains("Summary:"), "{}", stdout(&piped));
    let off = semadiff(&[
        "--color",
        "never",
        "--summary",
        "off",
        a.to_str().unwrap(),
        b.to_str().unwrap(),
    ]);
    assert!(!stdout(&off).contains("Summary:"), "{}", stdout(&off));
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}
