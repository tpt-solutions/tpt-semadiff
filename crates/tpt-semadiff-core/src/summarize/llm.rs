//! LLM-backed summarizer. Sends the semantic change list to a model and asks for a few
//! plain sentences. Any failure (no key, network error, empty answer) falls back to the
//! rule-based summarizer, so callers always get lines back.
//!
//! Privacy: the change descriptions (which quote snippets of your code) are sent to the
//! Anthropic API. This only happens when the caller asks for it.

use std::env;
use std::fmt;
use std::time::Duration;

use serde_json::json;

use crate::diff::FileDiff;
use crate::report::Report;

use super::rules;

/// Model used when `SEMADIFF_LLM_MODEL` is not set.
pub const DEFAULT_MODEL: &str = "claude-haiku-5-5";
/// Response size used when `SEMADIFF_LLM_MAX_TOKENS` is not set.
pub const DEFAULT_MAX_TOKENS: u32 = 512;
const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";
const MAX_LINES: usize = 5;
const TIMEOUT: Duration = Duration::from_secs(20);

const SYSTEM: &str = "You summarize structural code changes for a human reviewer. \
Answer with at most 5 short bullet lines, one change per line. \
No preamble, no headings, no code fences.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmError(pub String);

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LlmError {}

/// Anything that can complete a prompt. Implemented by [`AnthropicClient`]; tests supply mocks.
pub trait LlmClient {
    fn complete(&self, system: &str, prompt: &str) -> Result<String, LlmError>;
}

/// Client for the Anthropic Messages API.
#[derive(Debug, Clone)]
pub struct AnthropicClient {
    api_key: String,
    model: String,
    max_tokens: u32,
    /// Replaces the built-in system prompt when set.
    system: Option<String>,
}

impl AnthropicClient {
    /// Reads `ANTHROPIC_API_KEY` (required) and the optional `SEMADIFF_LLM_MODEL`,
    /// `SEMADIFF_LLM_MAX_TOKENS` (a positive integer) and `SEMADIFF_LLM_SYSTEM` (a
    /// system prompt that replaces the built-in one). Invalid optional values are ignored.
    pub fn from_env() -> Option<Self> {
        let api_key = env::var("ANTHROPIC_API_KEY")
            .ok()
            .filter(|k| !k.trim().is_empty())?;
        let model = env::var("SEMADIFF_LLM_MODEL")
            .ok()
            .filter(|m| !m.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_MODEL.to_owned());
        let max_tokens = env::var("SEMADIFF_LLM_MAX_TOKENS")
            .ok()
            .and_then(|v| v.trim().parse::<u32>().ok())
            .filter(|&n| n > 0)
            .unwrap_or(DEFAULT_MAX_TOKENS);
        let system = env::var("SEMADIFF_LLM_SYSTEM")
            .ok()
            .filter(|s| !s.trim().is_empty());
        Some(Self {
            api_key,
            model,
            max_tokens,
            system,
        })
    }
}

impl LlmClient for AnthropicClient {
    fn complete(&self, system: &str, prompt: &str) -> Result<String, LlmError> {
        let system = self.system.as_deref().unwrap_or(system);
        let body = json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "system": system,
            "messages": [{ "role": "user", "content": prompt }],
        });
        let mut response = ureq::post(ENDPOINT)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .config()
            .timeout_global(Some(TIMEOUT))
            .build()
            .send_json(body)
            .map_err(|e| LlmError(e.to_string()))?;
        let value: serde_json::Value = response
            .body_mut()
            .read_json()
            .map_err(|e| LlmError(e.to_string()))?;
        value["content"][0]["text"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| LlmError("response contained no text block".to_owned()))
    }
}

/// Where a summary came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Llm,
    Rules,
}

impl Source {
    /// The name shown in output: `llm` or `rules`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Llm => "llm",
            Self::Rules => "rules",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub source: Source,
    pub lines: Vec<String>,
    /// Why the LLM was not used, when one was asked for and the rules answered instead.
    pub fallback: Option<String>,
}

/// Summarizes with `client` when given, otherwise (or on any failure) with the rules. A
/// failure is recorded in [`Summary::fallback`].
pub fn summarize_with_fallback(
    client: Option<&dyn LlmClient>,
    diff: &FileDiff,
    report: &Report,
) -> Summary {
    let reason = match client {
        None => "ANTHROPIC_API_KEY is not set".to_owned(),
        Some(client) => match client.complete(SYSTEM, &build_prompt(report)) {
            Err(err) => format!("API request failed: {err}"),
            Ok(text) => {
                let lines = parse_lines(&text);
                if !lines.is_empty() {
                    return Summary {
                        source: Source::Llm,
                        lines,
                        fallback: None,
                    };
                }
                "the model returned no summary lines".to_owned()
            }
        },
    };
    Summary {
        source: Source::Rules,
        lines: rules::summarize(diff),
        fallback: Some(reason),
    }
}

/// The user message: one line per change, grouped by function.
pub fn build_prompt(report: &Report) -> String {
    let mut prompt = format!(
        "Language: {}\nChanges from {} to {}:\n",
        report.language, report.old_name, report.new_name
    );
    for group in &report.groups {
        let name = group.function.as_deref().unwrap_or("(top level)");
        prompt.push_str(&format!("\nIn {name}:\n"));
        for change in &group.changes {
            prompt.push_str(&format!("- {:?}: {}\n", change.op, change.description));
        }
    }
    prompt
}

/// Splits a model answer into clean bullet lines, dropping markers and blanks.
pub fn parse_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| {
            line.trim()
                .trim_start_matches(['-', '*', '•'])
                .trim()
                .to_owned()
        })
        .filter(|line| !line.is_empty())
        .take(MAX_LINES)
        .collect()
}
