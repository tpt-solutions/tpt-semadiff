//! Phase 16: project config, `init`, languages and excludes, brief output modes, the man
//! page, and the JSON schema. Runs the built binary, with the working directory set where
//! the config file is looked up.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const OLD_RS: &str = "fn total(v: &[i32]) -> i32 {\n    let sum = 0;\n    sum\n}\n";
const NEW_RS: &str = "fn total(v: &[i32]) -> i32 {\n    let acc = 0;\n    acc\n}\n";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tpt-semadiff-p16-{}-{name}", std::process::id()));
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

/// Runs semadiff with `cwd` as its working directory.
fn semadiff_in(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_semadiff"))
        .current_dir(cwd)
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

#[test]
fn init_writes_a_template_and_refuses_to_overwrite() {
    let dir = scratch("init");
    let first = semadiff_in(&dir, &["init"]);
    assert_eq!(code(&first), 0, "{}", stderr(&first));
    assert!(dir.join(".semadiff.toml").is_file());

    let second = semadiff_in(&dir, &["init"]);
    assert_eq!(code(&second), 2);
    assert!(
        stderr(&second).contains("already exists"),
        "{}",
        stderr(&second)
    );

    let forced = semadiff_in(&dir, &["init", "--force"]);
    assert_eq!(code(&forced), 0);
}

#[test]
fn the_init_template_parses_and_changes_nothing() {
    let dir = scratch("template");
    assert_eq!(code(&semadiff_in(&dir, &["init"])), 0);
    let a = write(&dir, "a.rs", OLD_RS);
    let b = write(&dir, "b.rs", NEW_RS);
    let out = semadiff_in(&dir, &[a.to_str().unwrap(), b.to_str().unwrap()]);
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    assert!(!stderr(&out).contains(".semadiff.toml"), "{}", stderr(&out));
}

#[test]
fn config_sets_the_engine_and_flags_override_it() {
    let dir = scratch("config-override");
    write(&dir, ".semadiff.toml", "engine = \"zhang-shasha\"\n");
    let a = write(&dir, "a.rs", OLD_RS);
    let b = write(&dir, "b.rs", NEW_RS);
    let from_config = semadiff_in(
        &dir,
        &["--out", "json", a.to_str().unwrap(), b.to_str().unwrap()],
    );
    assert!(stdout(&from_config).contains("\"engine\": \"zhang-shasha\""));

    let overridden = semadiff_in(
        &dir,
        &[
            "--engine",
            "chawathe",
            "--out",
            "json",
            a.to_str().unwrap(),
            b.to_str().unwrap(),
        ],
    );
    assert!(stdout(&overridden).contains("\"engine\": \"chawathe\""));
}

#[test]
fn a_bad_config_is_reported_with_its_path() {
    let dir = scratch("bad-config");
    write(&dir, ".semadiff.toml", "engnie = \"chawathe\"\n");
    let a = write(&dir, "a.rs", OLD_RS);
    let out = semadiff_in(&dir, &[a.to_str().unwrap(), a.to_str().unwrap()]);
    assert_eq!(code(&out), 2);
    assert!(stderr(&out).contains(".semadiff.toml"), "{}", stderr(&out));
}

#[test]
fn config_max_functions_gates_the_run() {
    let dir = scratch("config-gate");
    write(&dir, ".semadiff.toml", "max-functions = 0\n");
    let a = write(&dir, "a.rs", OLD_RS);
    let b = write(&dir, "b.rs", NEW_RS);
    let out = semadiff_in(&dir, &[a.to_str().unwrap(), b.to_str().unwrap()]);
    assert_eq!(code(&out), 1);
    assert!(
        stderr(&out).contains("over the limit of 0"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn quiet_prints_nothing_but_keeps_the_exit_status() {
    let dir = scratch("quiet");
    let a = write(&dir, "a.rs", OLD_RS);
    let b = write(&dir, "b.rs", NEW_RS);
    let out = semadiff_in(&dir, &["--quiet", a.to_str().unwrap(), b.to_str().unwrap()]);
    assert_eq!(code(&out), 1);
    assert!(stdout(&out).is_empty(), "{}", stdout(&out));
}

#[test]
fn name_only_lists_changed_files() {
    let old = scratch("name-old");
    let new = scratch("name-new");
    write(&old, "changed.rs", OLD_RS);
    write(&new, "changed.rs", NEW_RS);
    write(&old, "same.rs", OLD_RS);
    write(&new, "same.rs", OLD_RS);
    let out = semadiff_in(
        &old,
        &["--name-only", old.to_str().unwrap(), new.to_str().unwrap()],
    );
    assert_eq!(code(&out), 1);
    let text = stdout(&out);
    assert_eq!(text.lines().count(), 1, "{text}");
    assert!(text.trim_end().ends_with("changed.rs"), "{text}");
}

#[test]
fn stat_gives_one_line_per_changed_file_and_a_total() {
    let old = scratch("stat-old");
    let new = scratch("stat-new");
    write(&old, "a.rs", OLD_RS);
    write(&new, "a.rs", NEW_RS);
    let out = semadiff_in(
        &old,
        &["--stat", old.to_str().unwrap(), new.to_str().unwrap()],
    );
    let text = stdout(&out);
    assert!(
        text.contains("a.rs | 0 inserted, 0 deleted, 0 moved, 2 updated"),
        "{text}"
    );
    assert!(text.contains("1 file(s) changed"), "{text}");
}

#[test]
fn quiet_and_stat_conflict() {
    let dir = scratch("conflict");
    let a = write(&dir, "a.rs", OLD_RS);
    let out = semadiff_in(
        &dir,
        &[
            "--quiet",
            "--stat",
            a.to_str().unwrap(),
            a.to_str().unwrap(),
        ],
    );
    assert_eq!(code(&out), 2);
}

#[test]
fn lang_flag_reads_an_unknown_extension_as_the_given_language() {
    let dir = scratch("lang");
    let a = write(&dir, "old.txt", OLD_RS);
    let b = write(&dir, "new.txt", NEW_RS);
    let out = semadiff_in(
        &dir,
        &[
            "--lang",
            "rust",
            "--out",
            "json",
            a.to_str().unwrap(),
            b.to_str().unwrap(),
        ],
    );
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    assert!(stdout(&out).contains("\"language\": \"rust\""));
}

#[test]
fn an_unknown_lang_is_a_usage_error() {
    let dir = scratch("lang-bad");
    let a = write(&dir, "a.rs", OLD_RS);
    let out = semadiff_in(
        &dir,
        &["--lang", "cobol", a.to_str().unwrap(), a.to_str().unwrap()],
    );
    assert_eq!(code(&out), 2);
    assert!(
        stderr(&out).contains("unknown language"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn config_extension_maps_a_file_type_to_a_language() {
    let dir = scratch("ext-map");
    write(&dir, ".semadiff.toml", "[extensions]\nrs2 = \"rust\"\n");
    let a = write(&dir, "a.rs2", OLD_RS);
    let b = write(&dir, "b.rs2", NEW_RS);
    let out = semadiff_in(
        &dir,
        &["--out", "json", a.to_str().unwrap(), b.to_str().unwrap()],
    );
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    assert!(stdout(&out).contains("\"language\": \"rust\""));
}

#[test]
fn a_script_without_an_extension_is_found_by_its_shebang() {
    let old = scratch("shebang-old");
    let new = scratch("shebang-new");
    write(
        &old,
        "tool",
        "#!/usr/bin/env python3\ndef f():\n    return 1\n",
    );
    write(
        &new,
        "tool",
        "#!/usr/bin/env python3\ndef f():\n    return 2\n",
    );
    let out = semadiff_in(
        &old,
        &["--name-only", old.to_str().unwrap(), new.to_str().unwrap()],
    );
    assert_eq!(code(&out), 1, "{}", stderr(&out));
    assert!(
        stdout(&out).trim_end().ends_with("tool"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn exclude_skips_matching_paths_in_directory_mode() {
    let old = scratch("exclude-old");
    let new = scratch("exclude-new");
    write(&old, "src/keep.rs", OLD_RS);
    write(&new, "src/keep.rs", NEW_RS);
    write(&old, "gen/skip.rs", OLD_RS);
    write(&new, "gen/skip.rs", NEW_RS);
    let out = semadiff_in(
        &old,
        &[
            "--exclude",
            "gen/**",
            "--name-only",
            old.to_str().unwrap(),
            new.to_str().unwrap(),
        ],
    );
    let text = stdout(&out);
    assert!(text.contains("keep.rs"), "{text}");
    assert!(!text.contains("skip.rs"), "{text}");
}

#[test]
fn default_excludes_skip_minified_and_vendored_files() {
    let old = scratch("default-old");
    let new = scratch("default-new");
    write(&old, "vendor/lib.rs", OLD_RS);
    write(&new, "vendor/lib.rs", NEW_RS);
    write(&old, "app.min.js", "var a=1;");
    write(&new, "app.min.js", "var a=2;");
    let out = semadiff_in(
        &old,
        &["--name-only", old.to_str().unwrap(), new.to_str().unwrap()],
    );
    assert_eq!(code(&out), 0, "{}", stdout(&out));
    assert!(stdout(&out).is_empty(), "{}", stdout(&out));
}

#[test]
fn directory_mode_honours_gitignore() {
    let old = scratch("gitignore-old");
    let new = scratch("gitignore-new");
    write(&old, ".gitignore", "ignored/\n");
    write(&new, ".gitignore", "ignored/\n");
    write(&old, "ignored/x.rs", OLD_RS);
    write(&new, "ignored/x.rs", NEW_RS);
    let out = semadiff_in(
        &old,
        &["--name-only", old.to_str().unwrap(), new.to_str().unwrap()],
    );
    assert_eq!(code(&out), 0, "{}", stdout(&out));
}

#[test]
fn man_page_is_printed() {
    let dir = scratch("man");
    let out = semadiff_in(&dir, &["man"]);
    assert_eq!(code(&out), 0);
    assert!(stdout(&out).contains("semadiff"), "{}", stdout(&out));
}

#[test]
fn json_output_matches_the_published_schema() {
    let schema_text = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/semadiff.schema.json"),
    )
    .expect("schema file");
    let schema: serde_json::Value = serde_json::from_str(&schema_text).expect("schema JSON");
    let validator = jsonschema::validator_for(&schema).expect("valid schema");

    let dir = scratch("schema");
    let a = write(&dir, "a.rs", OLD_RS);
    let b = write(&dir, "b.rs", NEW_RS);
    let single = semadiff_in(
        &dir,
        &["--out", "json", a.to_str().unwrap(), b.to_str().unwrap()],
    );
    let instance: serde_json::Value = serde_json::from_str(&stdout(&single)).expect("JSON");
    let errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|e| e.to_string())
        .collect();
    assert!(errors.is_empty(), "{errors:?}");

    let old = scratch("schema-old");
    let new = scratch("schema-new");
    write(&old, "a.rs", OLD_RS);
    write(&new, "a.rs", NEW_RS);
    let multi = semadiff_in(
        &old,
        &[
            "--out",
            "json",
            old.to_str().unwrap(),
            new.to_str().unwrap(),
        ],
    );
    let instance: serde_json::Value = serde_json::from_str(&stdout(&multi)).expect("JSON");
    assert_eq!(instance["schema_version"], 1);
    let errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|e| e.to_string())
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
}
