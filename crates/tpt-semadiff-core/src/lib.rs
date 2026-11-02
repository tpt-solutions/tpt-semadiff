//! Structural, AST-aware diffing for tpt-semadiff.
//!
//! The pipeline is: parse ([`parse`]) → normalize trivia ([`normalize`]) →
//! match nodes ([`gumtree`] or [`zhang_shasha`]) → derive semantic operations
//! ([`ops`]). [`diff::diff_files`] runs the whole thing; [`report`] and
//! [`render`] turn the result into output.

pub mod diff;
pub mod gumtree;
pub mod ir;
pub mod mapping;
pub mod normalize;
pub mod ops;
pub mod parse;
pub mod render;
pub mod report;
pub mod summarize;
pub mod tree;
pub mod zhang_shasha;
