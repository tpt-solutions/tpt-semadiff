//! Directory mode: `semadiff OLD_DIR NEW_DIR` pairs up the supported source files of two
//! trees by their relative path. A file present on one side only is compared with an
//! empty file. `.gitignore` files are honoured, and the excludes in `detect` are skipped.

use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::detect::{first_line, Excludes, Languages};
use crate::pipeline::Pair;

/// Pairs for every supported file under either directory, in path order.
pub fn pairs(
    old_root: &Path,
    new_root: &Path,
    languages: &Languages,
    excludes: &Excludes,
) -> Result<Vec<Pair>, Box<dyn Error>> {
    let mut paths = BTreeSet::new();
    collect(old_root, &mut paths, excludes)?;
    collect(new_root, &mut paths, excludes)?;

    let mut pairs = Vec::with_capacity(paths.len());
    for rel in paths {
        let old_full = old_root.join(&rel);
        let new_full = new_root.join(&rel);
        // Only a file without an extension can be identified by its first line.
        let head = if rel.extension().is_none() {
            first_line(&new_full).or_else(|| first_line(&old_full))
        } else {
            None
        };
        let Some(lang) = languages.lang_for(&rel, head.as_deref()) else {
            continue;
        };
        pairs.push(Pair {
            old_name: old_full.display().to_string(),
            new_name: new_full.display().to_string(),
            old_src: read_or_empty(&old_full)?,
            new_src: read_or_empty(&new_full)?,
            lang: Some(lang),
        });
    }
    Ok(pairs)
}

/// Adds the path, relative to `root`, of every file under it that is not excluded. Git
/// metadata and symbolic links are skipped. Whether a file is supported is decided later.
fn collect(
    root: &Path,
    out: &mut BTreeSet<PathBuf>,
    excludes: &Excludes,
) -> Result<(), Box<dyn Error>> {
    let walk = WalkBuilder::new(root)
        .hidden(false)
        .require_git(false)
        .build();
    for entry in walk {
        let entry = entry.map_err(|e| format!("{}: {e}", root.display()))?;
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let rel = entry.path().strip_prefix(root)?.to_path_buf();
        if rel.components().any(|c| c.as_os_str() == ".git") || excludes.excludes(&rel) {
            continue;
        }
        out.insert(rel);
    }
    Ok(())
}

fn read_or_empty(path: &Path) -> Result<String, Box<dyn Error>> {
    if !path.exists() {
        return Ok(String::new());
    }
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()).into())
}
