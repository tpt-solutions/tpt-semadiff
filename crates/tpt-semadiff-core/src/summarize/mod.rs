//! Natural-language summaries of a change.
//!
//! [`rules`] is the offline, deterministic summarizer. [`llm`] asks a model for a summary
//! and falls back to the rules whenever that is unavailable or fails.

pub mod llm;
pub mod rules;
