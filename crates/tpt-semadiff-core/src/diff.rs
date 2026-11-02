//! Top-level diff pipeline: normalize both files, match nodes, derive the semantic script.

use crate::gumtree;
use crate::mapping::Mapping;
use crate::normalize::{normalize, Normalized, TriviaPolicy};
use crate::ops::{script_from_mapping, EditOp};
use crate::parse::{ParseError, SourceLang};
use crate::tree::Tree;
use crate::zhang_shasha;

/// Matching engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Engine {
    /// Zhang-Shasha minimum-cost edit mapping. Exact but slow on large files.
    ZhangShasha,
    /// GumTree-style matching with a Chawathe edit script. Fast; the default.
    #[default]
    Chawathe,
}

impl Engine {
    pub fn name(self) -> &'static str {
        match self {
            Self::ZhangShasha => "zhang-shasha",
            Self::Chawathe => "chawathe",
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    pub engine: Engine,
    /// Strip comments before diffing, so comment-only edits report zero changes.
    /// Whitespace never affects the result.
    pub ignore_formatting: bool,
}

/// The result of diffing two files.
#[derive(Debug, Clone)]
pub struct FileDiff {
    pub lang: SourceLang,
    pub old: Normalized,
    pub new: Normalized,
    pub mapping: Mapping,
    pub ops: Vec<EditOp>,
}

/// Diffs two source texts written in `lang`.
pub fn diff_files(
    old_src: &str,
    new_src: &str,
    lang: SourceLang,
    options: Options,
) -> Result<FileDiff, ParseError> {
    let policy = if options.ignore_formatting {
        TriviaPolicy::Strip
    } else {
        TriviaPolicy::Keep
    };
    let old = normalize(old_src, lang, policy)?;
    let new = normalize(new_src, lang, policy)?;
    let mapping = match options.engine {
        Engine::ZhangShasha => zhang_shasha::mapping(old.tree(), new.tree()).1,
        Engine::Chawathe => gumtree::mapping(old.tree(), new.tree()),
    };
    let ops = script_from_mapping(old.tree(), new.tree(), &mapping);
    Ok(FileDiff {
        lang,
        old,
        new,
        mapping,
        ops,
    })
}

/// Runs the matching and script on two already-built trees.
pub fn diff_trees<A: Tree, B: Tree>(old: &A, new: &B, engine: Engine) -> (Mapping, Vec<EditOp>) {
    let mapping = match engine {
        Engine::ZhangShasha => zhang_shasha::mapping(old, new).1,
        Engine::Chawathe => gumtree::mapping(old, new),
    };
    let ops = script_from_mapping(old, new, &mapping);
    (mapping, ops)
}
