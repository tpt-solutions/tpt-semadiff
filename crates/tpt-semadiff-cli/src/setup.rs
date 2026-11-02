//! `semadiff git-setup`: writes, or removes, the git config that makes semadiff the
//! difftool. Re-running it changes nothing that is already in place.

use std::error::Error;
use std::process::Command;

/// The difftool command git runs for each file pair. git substitutes the paths.
const DIFFTOOL_CMD: &str = "semadiff \"$LOCAL\" \"$REMOTE\"";
/// semadiff exits 1 when it finds changes, so git must not treat that as a failure.
const TRUST_EXIT_CODE: &str = "false";
const EXTERNAL_CMD: &str = "semadiff";

const DIFFTOOL_CMD_KEY: &str = "difftool.semadiff.cmd";
const TRUST_KEY: &str = "difftool.semadiff.trustExitCode";
const EXTERNAL_KEY: &str = "diff.external";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Global,
    Local,
}

impl Scope {
    fn flag(self) -> &'static str {
        match self {
            Self::Global => "--global",
            Self::Local => "--local",
        }
    }
}

pub struct Setup {
    pub scope: Scope,
    /// Also make `git diff` use semadiff, through `diff.external`.
    pub external: bool,
    pub uninstall: bool,
}

pub fn run(setup: &Setup) -> Result<(), Box<dyn Error>> {
    if setup.uninstall {
        uninstall(setup.scope)
    } else {
        install(setup.scope, setup.external)
    }
}

fn install(scope: Scope, external: bool) -> Result<(), Box<dyn Error>> {
    set(scope, DIFFTOOL_CMD_KEY, DIFFTOOL_CMD)?;
    set(scope, TRUST_KEY, TRUST_EXIT_CODE)?;
    if external {
        set(scope, EXTERNAL_KEY, EXTERNAL_CMD)?;
    }
    println!(
        "Use `git difftool` with semadiff ({}). Run `semadiff doctor` to check the setup.",
        scope_name(scope)
    );
    Ok(())
}

fn uninstall(scope: Scope) -> Result<(), Box<dyn Error>> {
    unset(scope, DIFFTOOL_CMD_KEY)?;
    unset(scope, TRUST_KEY)?;
    // Only remove `diff.external` when it is ours. Another tool may own it.
    match get(scope, EXTERNAL_KEY)? {
        Some(value) if value == EXTERNAL_CMD => unset(scope, EXTERNAL_KEY)?,
        Some(other) => println!("{EXTERNAL_KEY} is {other:?}, not semadiff; leaving it alone."),
        None => {}
    }
    println!("semadiff git settings removed ({}).", scope_name(scope));
    Ok(())
}

fn scope_name(scope: Scope) -> &'static str {
    match scope {
        Scope::Global => "global",
        Scope::Local => "this repository",
    }
}

/// Writes `key`, unless it already holds `value`. Reports a value it replaces.
fn set(scope: Scope, key: &str, value: &str) -> Result<(), Box<dyn Error>> {
    match get(scope, key)? {
        Some(current) if current == value => {
            println!("{key} already set.");
            return Ok(());
        }
        Some(previous) => println!("{key}: replacing {previous:?}."),
        None => {}
    }
    git_config(scope, &[key, value])?;
    println!("{key} = {value}");
    Ok(())
}

/// Removes `key` if it is set. A key that is already absent is not an error.
fn unset(scope: Scope, key: &str) -> Result<(), Box<dyn Error>> {
    if get(scope, key)?.is_none() {
        return Ok(());
    }
    git_config(scope, &["--unset", key])?;
    println!("{key} removed.");
    Ok(())
}

/// The value of `key` in `scope`, or `None` when it is not set there.
fn get(scope: Scope, key: &str) -> Result<Option<String>, Box<dyn Error>> {
    let out = Command::new("git")
        .args(["config", scope.flag(), "--get", key])
        .output()
        .map_err(|e| format!("could not run git: {e}"))?;
    match out.status.code() {
        Some(0) => Ok(Some(
            String::from_utf8_lossy(&out.stdout).trim_end().to_owned(),
        )),
        // `git config --get` exits 1 when the key is absent.
        Some(1) => Ok(None),
        _ => Err(git_error(&out.stderr).into()),
    }
}

fn git_config(scope: Scope, args: &[&str]) -> Result<(), Box<dyn Error>> {
    let out = Command::new("git")
        .args(["config", scope.flag()])
        .args(args)
        .output()
        .map_err(|e| format!("could not run git: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(git_error(&out.stderr).into())
    }
}

fn git_error(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr).trim().to_owned();
    if text.is_empty() {
        "git config failed".to_owned()
    } else {
        text
    }
}
