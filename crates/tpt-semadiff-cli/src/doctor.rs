//! `semadiff doctor`: checks that the pieces semadiff relies on are in place.

use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use tpt_semadiff_core::parse::{parse_tree, SourceLang};

/// Prints one line per check. Exits 1 when git is missing or a grammar fails to load;
/// the other checks are advice, not failures.
pub fn run() -> ExitCode {
    let mut failed = false;

    match git_version() {
        Some(version) => report("ok", "git", &version),
        None => {
            failed = true;
            report(
                "fail",
                "git",
                "not found on PATH; semadiff git features need it",
            );
        }
    }

    match find_on_path("semadiff") {
        Some(path) => report("ok", "semadiff on PATH", &path.display().to_string()),
        None => report(
            "warn",
            "semadiff on PATH",
            "not found; git cannot run it by name",
        ),
    }

    match git_config_value("difftool.semadiff.cmd") {
        Some(cmd) => report("ok", "difftool", &cmd),
        None => report("warn", "difftool", "not set; run `semadiff git-setup`"),
    }
    match git_config_value("diff.external") {
        Some(cmd) => report("ok", "diff.external", &cmd),
        None => report(
            "info",
            "diff.external",
            "not set (only needed for `git diff`)",
        ),
    }

    // Only report whether the key is set. Its value must never reach the output.
    match env::var("ANTHROPIC_API_KEY") {
        Ok(key) if !key.trim().is_empty() => report(
            "ok",
            "ANTHROPIC_API_KEY",
            "set (--summary llm can call the API)",
        ),
        _ => report(
            "info",
            "ANTHROPIC_API_KEY",
            "not set (--summary llm uses rules)",
        ),
    }

    for lang in SourceLang::ALL {
        match parse_tree("", lang) {
            Ok(_) => report("ok", "grammar", lang.name()),
            Err(err) => {
                failed = true;
                report("fail", "grammar", &format!("{}: {err}", lang.name()));
            }
        }
    }

    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn report(status: &str, check: &str, detail: &str) {
    println!("[{status:>4}] {check:<18} {detail}");
}

fn git_version() -> Option<String> {
    let out = Command::new("git").arg("--version").output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// The effective value of a git config key, or `None` when unset or git is unavailable.
fn git_config_value(key: &str) -> Option<String> {
    let out = Command::new("git")
        .args(["config", "--get", key])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let names = [name.to_owned(), format!("{name}.exe")];
    env::split_paths(&env::var_os("PATH")?).find_map(|dir| {
        names
            .iter()
            .map(|n| dir.join(n))
            .find(|candidate| candidate.is_file())
    })
}
