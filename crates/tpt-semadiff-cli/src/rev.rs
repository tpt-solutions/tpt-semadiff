//! Git revision mode. `--rev REV` compares REV with the working tree, `--rev A..B`
//! compares two revisions, and `--rev A...B` compares B with the merge base of A and B.

use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

use tpt_semadiff_core::parse::SourceLang;

use crate::pipeline::Pair;

/// Pairs for every file that git reports as changed, plus the number of changed files
/// skipped because their extension is not a supported language.
pub fn pairs(spec: &str) -> Result<(Vec<Pair>, usize), Box<dyn Error>> {
    let (old_label, old_rev, new_side) = parse_spec(spec)?;
    let root = git(&["rev-parse", "--show-toplevel"])?;
    let root = Path::new(root.trim_end());

    let listing = git(&[
        "diff",
        "--no-color",
        "--name-status",
        "-z",
        "-M",
        spec,
        "--",
    ])?;
    let mut tokens = listing.split('\0').filter(|t| !t.is_empty());

    let mut pairs = Vec::new();
    let mut skipped = 0;
    while let Some(status) = tokens.next() {
        let code = status.chars().next().unwrap_or('M');
        // Renames and copies carry two paths: the old one, then the new one.
        let (old_path, new_path) = if matches!(code, 'R' | 'C') {
            let old = tokens.next().ok_or("malformed git diff output")?;
            let new = tokens.next().ok_or("malformed git diff output")?;
            (old, new)
        } else {
            let path = tokens.next().ok_or("malformed git diff output")?;
            (path, path)
        };

        let lang = SourceLang::from_path(Path::new(new_path))
            .or_else(|| SourceLang::from_path(Path::new(old_path)));
        let Some(lang) = lang else {
            skipped += 1;
            continue;
        };

        let old_src = if code == 'A' {
            String::new()
        } else {
            git(&["show", &format!("{old_rev}:{old_path}")])?
        };
        let new_src = match (&new_side, code) {
            (_, 'D') => String::new(),
            (NewSide::Rev(rev), _) => git(&["show", &format!("{rev}:{new_path}")])?,
            (NewSide::WorkTree, _) => {
                let full = root.join(new_path);
                fs::read_to_string(&full).map_err(|e| format!("{}: {e}", full.display()))?
            }
        };

        let new_name = match &new_side {
            NewSide::Rev(rev) => format!("{rev}:{new_path}"),
            NewSide::WorkTree => new_path.to_owned(),
        };
        pairs.push(Pair {
            old_name: format!("{old_label}:{old_path}"),
            new_name,
            old_src,
            new_src,
            lang: Some(lang),
        });
    }
    Ok((pairs, skipped))
}

/// The side a revision spec compares against.
enum NewSide {
    Rev(String),
    WorkTree,
}

/// Splits a spec into the old side's label, the revision its contents are read from, and
/// the new side.
fn parse_spec(spec: &str) -> Result<(String, String, NewSide), Box<dyn Error>> {
    let (label, old_rev, new) = if let Some((a, b)) = spec.split_once("...") {
        let (a, b) = non_empty(a, b, spec)?;
        let base = git(&["merge-base", a, b])?.trim().to_owned();
        (a.to_owned(), base, NewSide::Rev(b.to_owned()))
    } else if let Some((a, b)) = spec.split_once("..") {
        let (a, b) = non_empty(a, b, spec)?;
        (a.to_owned(), a.to_owned(), NewSide::Rev(b.to_owned()))
    } else {
        if spec.is_empty() || spec.starts_with('-') {
            return Err(format!("not a revision: {spec:?}").into());
        }
        (spec.to_owned(), spec.to_owned(), NewSide::WorkTree)
    };
    Ok((label, old_rev, new))
}

fn non_empty<'a>(a: &'a str, b: &'a str, spec: &str) -> Result<(&'a str, &'a str), Box<dyn Error>> {
    if a.is_empty() || b.is_empty() || a.starts_with('-') || b.starts_with('-') {
        return Err(format!("expected REV or A..B, got {spec:?}").into());
    }
    Ok((a, b))
}

/// Runs `git` and returns its standard output, or its error text on failure.
fn git(args: &[&str]) -> Result<String, Box<dyn Error>> {
    let out = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| format!("could not run git: {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_owned();
        return Err(if stderr.is_empty() {
            format!("git {} failed", args.join(" ")).into()
        } else {
            stderr.into()
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}
