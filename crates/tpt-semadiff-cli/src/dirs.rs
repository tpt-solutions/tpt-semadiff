//! Directory mode: `semadiff OLD_DIR NEW_DIR` pairs up the supported source files of two
//! trees by their relative path. A file present on one side only is compared with an
//! empty file.

use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use tpt_semadiff_core::parse::SourceLang;

use crate::pipeline::Pair;

/// Pairs for every supported file under either directory, in path order.
pub fn pairs(old_root: &Path, new_root: &Path) -> Result<Vec<Pair>, Box<dyn Error>> {
    let mut paths = BTreeSet::new();
    collect(old_root, Path::new(""), &mut paths)?;
    collect(new_root, Path::new(""), &mut paths)?;

    let mut pairs = Vec::with_capacity(paths.len());
    for rel in paths {
        let old_full = old_root.join(&rel);
        let new_full = new_root.join(&rel);
        let lang = SourceLang::from_path(&rel);
        pairs.push(Pair {
            old_name: old_full.display().to_string(),
            new_name: new_full.display().to_string(),
            old_src: read_or_empty(&old_full)?,
            new_src: read_or_empty(&new_full)?,
            lang,
        });
    }
    Ok(pairs)
}

/// Adds the relative path of each supported file under `root/rel`. Git metadata and
/// symbolic links are skipped.
fn collect(root: &Path, rel: &Path, out: &mut BTreeSet<PathBuf>) -> Result<(), Box<dyn Error>> {
    let dir = root.join(rel);
    for entry in fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".git" {
            continue;
        }
        let child = rel.join(&name);
        let kind = entry.file_type()?;
        if kind.is_dir() {
            collect(root, &child, out)?;
        } else if kind.is_file() && SourceLang::from_path(&child).is_some() {
            out.insert(child);
        }
    }
    Ok(())
}

fn read_or_empty(path: &Path) -> Result<String, Box<dyn Error>> {
    if !path.exists() {
        return Ok(String::new());
    }
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()).into())
}
