//! Turns old/new texts into a report, or into a fallback when the file cannot be diffed
//! structurally. Every front-end mode (file pair, directory, git revision) goes through here.

use tpt_semadiff_core::diff::{diff_files, Options};
use tpt_semadiff_core::parse::SourceLang;
use tpt_semadiff_core::report::Report;
use tpt_semadiff_core::summarize::llm::{self, AnthropicClient, LlmClient};
use tpt_semadiff_core::summarize::rules;

/// Which natural-language summary to add to each report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Summary {
    Rules,
    Llm,
}

/// One file to compare: display names, both texts, and the language if one applies.
pub struct Pair {
    pub old_name: String,
    pub new_name: String,
    pub old_src: String,
    pub new_src: String,
    pub lang: Option<SourceLang>,
}

pub enum Outcome {
    /// Parsed on both sides and diffed structurally.
    Diffed { report: Report, changed: bool },
    /// The extension is not a supported language.
    Unsupported,
    /// The language is supported, but the source did not parse.
    Unparsed { reason: String },
}

pub struct Entry {
    pub pair: Pair,
    pub outcome: Outcome,
}

impl Entry {
    /// Whether the two texts differ in a way semadiff reports. A file that did not parse
    /// counts as changed when its text differs.
    pub fn changed(&self) -> bool {
        match &self.outcome {
            Outcome::Diffed { changed, .. } => *changed,
            Outcome::Unsupported | Outcome::Unparsed { .. } => {
                self.pair.old_src != self.pair.new_src
            }
        }
    }

    /// Why the file fell back to a line diff, if it did.
    pub fn fallback_reason(&self) -> Option<&str> {
        match &self.outcome {
            Outcome::Diffed { .. } => None,
            Outcome::Unsupported => Some("unsupported file type"),
            Outcome::Unparsed { reason } => Some(reason),
        }
    }
}

/// Diffs one pair. `llm` is only read when `summary` is [`Summary::Llm`].
pub fn diff_pair(
    pair: Pair,
    options: Options,
    summary: Option<Summary>,
    llm: Option<&AnthropicClient>,
) -> Entry {
    let outcome = match pair.lang {
        None => Outcome::Unsupported,
        Some(lang) => match diff_files(&pair.old_src, &pair.new_src, lang, options) {
            Err(err) => Outcome::Unparsed {
                reason: err.to_string(),
            },
            Ok(diff) => {
                let mut report =
                    Report::from_diff(&diff, &pair.old_name, &pair.new_name, options.engine.name());
                report.summaries = match summary {
                    None => Vec::new(),
                    Some(Summary::Rules) => rules::summarize(&diff),
                    Some(Summary::Llm) => {
                        let client = llm.map(|c| c as &dyn LlmClient);
                        llm::summarize_with_fallback(client, &diff, &report).lines
                    }
                };
                Outcome::Diffed {
                    changed: !diff.ops.is_empty(),
                    report,
                }
            }
        },
    };
    Entry { pair, outcome }
}
