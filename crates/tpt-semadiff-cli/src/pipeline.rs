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

/// Diffs every pair, spreading the work over the available CPUs. The entries come back in
/// the order of `pairs`.
pub fn diff_all(
    pairs: Vec<Pair>,
    options: Options,
    summary: Option<Summary>,
    llm: Option<&AnthropicClient>,
) -> Vec<Entry> {
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    if threads <= 1 || pairs.len() <= 1 {
        return pairs
            .into_iter()
            .map(|pair| diff_pair(pair, options, summary, llm))
            .collect();
    }
    let chunk_len = pairs.len().div_ceil(threads);
    let mut chunks = Vec::with_capacity(threads);
    let mut rest = pairs.into_iter().peekable();
    while rest.peek().is_some() {
        chunks.push(rest.by_ref().take(chunk_len).collect::<Vec<_>>());
    }
    std::thread::scope(|scope| {
        let workers: Vec<_> = chunks
            .into_iter()
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .into_iter()
                        .map(|pair| diff_pair(pair, options, summary, llm))
                        .collect::<Vec<Entry>>()
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("a diff thread panicked"))
            .collect()
    })
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
                match summary {
                    None => {}
                    Some(Summary::Rules) => {
                        report.summaries = rules::summarize(&diff);
                        report.summary_source = Some("rules");
                    }
                    Some(Summary::Llm) => {
                        let client = llm.map(|c| c as &dyn LlmClient);
                        let found = llm::summarize_with_fallback(client, &diff, &report);
                        report.summaries = found.lines;
                        report.summary_source = Some(found.source.name());
                        report.summary_fallback = found.fallback;
                    }
                }
                Outcome::Diffed {
                    changed: !diff.ops.is_empty(),
                    report,
                }
            }
        },
    };
    Entry { pair, outcome }
}
