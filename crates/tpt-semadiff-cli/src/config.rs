//! `.semadiff.toml`: project settings. The nearest file at or above the current directory
//! applies. Command-line flags override its values.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::Path;

use serde::Deserialize;

/// File name searched for, walking up from the current directory.
pub const FILE_NAME: &str = ".semadiff.toml";

/// The template `semadiff init` writes. Every setting is commented out, so the file changes
/// nothing until someone enables a line.
pub const TEMPLATE: &str = "\
# semadiff project settings. Command-line flags override these.

# engine = \"chawathe\"           # or \"zhang-shasha\" (exact, slower)
# ignore-formatting = false      # comments, docstrings, redundant parens and trailing commas
# summary = \"rules\"             # \"rules\", \"llm\" or \"off\"
# max-functions = 3              # CI gate: fail when more functions change
# fail-on-unparseable = false    # CI gate: fail when a changed file does not parse

# Globs, relative to the compared directory or repository root. `**/` matches at any depth.
# exclude = [\"docs/**\", \"**/generated/**\"]

# File extensions mapped to a language. Names are: rust, python, javascript,
# typescript, tsx, go.
# [extensions]
# mjs = \"javascript\"
";

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    /// `chawathe` or `zhang-shasha`.
    pub engine: Option<String>,
    #[serde(default)]
    pub ignore_formatting: bool,
    /// `rules`, `llm` or `off`.
    pub summary: Option<String>,
    pub max_functions: Option<usize>,
    #[serde(default)]
    pub fail_on_unparseable: bool,
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Extension (without the dot) to language name.
    #[serde(default)]
    pub extensions: BTreeMap<String, String>,
}

/// The config that applies to `start`: the nearest `.semadiff.toml` at or above it, if any.
pub fn discover(start: &Path) -> Result<Option<Config>, Box<dyn Error>> {
    for dir in start.ancestors() {
        let file = dir.join(FILE_NAME);
        if file.is_file() {
            let text = fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
            let config = toml::from_str(&text).map_err(|e| format!("{}: {e}", file.display()))?;
            return Ok(Some(config));
        }
    }
    Ok(None)
}
