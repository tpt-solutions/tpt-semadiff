//! Which files are compared, and in which language: the default excludes, `--exclude`
//! globs, `--lang`, extension mappings from the config, and `#!` lines for scripts that
//! have no extension.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::Path;

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use tpt_semadiff_core::parse::SourceLang;

/// Paths left out of directory and revision mode. Generated, minified, vendored and lock
/// files are noise in a review, so they are skipped unless `--exclude` says otherwise.
const DEFAULT_EXCLUDES: &[&str] = &[
    "**/node_modules/**",
    "**/vendor/**",
    "**/*.min.js",
    "**/*.min.mjs",
    "**/*.min.cjs",
    "**/*.bundle.js",
    "**/*.generated.*",
    "**/*_generated.go",
    "**/*.pb.go",
    "**/Cargo.lock",
    "**/package-lock.json",
    "**/yarn.lock",
    "**/pnpm-lock.yaml",
    "**/go.sum",
    "**/poetry.lock",
];

/// Glob matcher for paths relative to the compared root.
pub struct Excludes {
    set: GlobSet,
}

impl Excludes {
    /// The default excludes plus `extra` (from `--exclude` and the config).
    pub fn new(extra: &[String]) -> Result<Self, Box<dyn Error>> {
        let mut builder = GlobSetBuilder::new();
        for pattern in DEFAULT_EXCLUDES
            .iter()
            .copied()
            .chain(extra.iter().map(String::as_str))
        {
            let glob = GlobBuilder::new(pattern)
                .literal_separator(true)
                .build()
                .map_err(|e| format!("bad exclude pattern {pattern:?}: {e}"))?;
            builder.add(glob);
        }
        Ok(Self {
            set: builder.build()?,
        })
    }

    pub fn excludes(&self, rel: &Path) -> bool {
        self.set.is_match(rel)
    }
}

/// Picks the language of a file.
pub struct Languages {
    /// `--lang`: every file is read as this language.
    forced: Option<SourceLang>,
    /// Config extension mappings, lower-cased and without the dot.
    by_ext: BTreeMap<String, SourceLang>,
}

impl Languages {
    pub fn new(
        forced: Option<SourceLang>,
        extensions: &BTreeMap<String, String>,
    ) -> Result<Self, Box<dyn Error>> {
        let mut by_ext = BTreeMap::new();
        for (ext, name) in extensions {
            let lang = SourceLang::from_name(name).ok_or_else(|| {
                format!("unknown language {name:?} for extension {ext:?} in .semadiff.toml")
            })?;
            by_ext.insert(ext.trim_start_matches('.').to_ascii_lowercase(), lang);
        }
        Ok(Self { forced, by_ext })
    }

    /// The language of `path`. Files with an extension go by it. A file without one goes by
    /// `first_line`, its first line, when that is a `#!` line.
    pub fn lang_for(&self, path: &Path, first_line: Option<&str>) -> Option<SourceLang> {
        if self.forced.is_some() {
            return self.forced;
        }
        match path.extension().and_then(|e| e.to_str()) {
            Some(ext) => self
                .by_ext
                .get(&ext.to_ascii_lowercase())
                .copied()
                .or_else(|| SourceLang::from_path(path)),
            None => first_line.and_then(SourceLang::from_shebang),
        }
    }
}

/// The first line of a file, read tolerantly: a binary file yields a lossy line instead of
/// an error. `None` when the file cannot be read.
pub fn first_line(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let head = &bytes[..bytes.len().min(512)];
    String::from_utf8_lossy(head)
        .lines()
        .next()
        .map(str::to_owned)
}
