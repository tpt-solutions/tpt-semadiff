//! `semadiff`: structural, AST-aware diff for source files.
//!
//! Exit status: `0` no semantic changes, `1` changes found, `2` usage or I/O error.

use std::error::Error;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use tpt_semadiff_core::diff::{diff_files, Engine, FileDiff, Options};
use tpt_semadiff_core::parse::SourceLang;
use tpt_semadiff_core::render;
use tpt_semadiff_core::report::Report;
use tpt_semadiff_core::summarize::llm::{self, AnthropicClient, LlmClient};
use tpt_semadiff_core::summarize::rules;

#[derive(Parser, Debug)]
#[command(
    name = "semadiff",
    version,
    about = "Structural, AST-aware diff for source files",
    long_about = "Compares two versions of a source file by its syntax tree rather than by \
                  lines. Exit status: 0 no semantic changes, 1 changes found, 2 error.\n\n\
                  As a git difftool: git config difftool.semadiff.cmd 'semadiff \"$LOCAL\" \"$REMOTE\"'"
)]
struct Cli {
    /// OLD and NEW file. Git's external-diff form (seven arguments) is also accepted.
    #[arg(required = true, num_args = 2..=7, value_name = "FILE")]
    files: Vec<PathBuf>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutFormat::Text)]
    out: OutFormat,

    /// Matching engine.
    #[arg(long, value_enum, default_value_t = EngineArg::Chawathe)]
    engine: EngineArg,

    /// Ignore comments, so that comment-only edits report zero changes.
    #[arg(long)]
    ignore_formatting: bool,

    /// When to colour text output.
    #[arg(long, value_enum, default_value_t = ColorArg::Auto)]
    color: ColorArg,

    /// Add a natural-language summary. `rules` runs offline; `llm` calls the Anthropic
    /// API (needs ANTHROPIC_API_KEY) and falls back to `rules` on failure.
    #[arg(long, value_enum)]
    summary: Option<SummaryArg>,

    /// Browse the changes in an interactive terminal viewer instead of printing.
    #[arg(long)]
    tui: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
enum OutFormat {
    /// Boxed per-function summary.
    Text,
    /// Classic line-based unified diff.
    Unified,
    /// Structured semantic diff as JSON.
    Json,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
enum EngineArg {
    /// GumTree-style matching with a Chawathe script (default).
    Chawathe,
    /// Zhang-Shasha minimum edit mapping (exact, slower).
    ZhangShasha,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
enum ColorArg {
    Auto,
    Always,
    Never,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
enum SummaryArg {
    Rules,
    Llm,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("semadiff: {err}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode, Box<dyn Error>> {
    let (old_path, new_path) = match cli.files.len() {
        2 => (&cli.files[0], &cli.files[1]),
        7 => (&cli.files[1], &cli.files[4]),
        n => return Err(format!("expected 2 files, got {n}").into()),
    };
    // Git's external-diff protocol treats any non-zero status as a failure, so that form
    // always exits 0 on success. Changes are then only visible in the output.
    let external = cli.files.len() == 7;
    let old_src = read_side(old_path)?;
    let new_src = read_side(new_path)?;
    let old_name = old_path.display().to_string();
    let new_name = new_path.display().to_string();

    let engine = match cli.engine {
        EngineArg::Chawathe => Engine::Chawathe,
        EngineArg::ZhangShasha => Engine::ZhangShasha,
    };
    let options = Options {
        engine,
        ignore_formatting: cli.ignore_formatting,
    };
    let lang = SourceLang::from_path(new_path).or_else(|| SourceLang::from_path(old_path));
    let outcome = match lang {
        Some(lang) => diff_files(&old_src, &new_src, lang, options).map_err(|e| e.to_string()),
        None => Err("unsupported file type".to_owned()),
    };

    let diff = match outcome {
        Ok(diff) => diff,
        Err(reason) if cli.out == OutFormat::Json || cli.tui => return Err(reason.into()),
        Err(reason) => {
            eprintln!("semadiff: falling back to a line diff ({reason})");
            print!(
                "{}",
                render::render_unified(&old_name, &new_name, &old_src, &new_src)
            );
            return Ok(exit_code(external, old_src == new_src));
        }
    };

    let mut report = Report::from_diff(&diff, &old_name, &new_name, engine.name());
    report.summaries = summarize(cli.summary, &diff, &report);

    if cli.tui {
        tpt_semadiff_tui::run(&report)?;
    } else {
        match cli.out {
            OutFormat::Json => println!("{}", report.to_json()?),
            OutFormat::Unified => print!(
                "{}",
                render::render_unified(&old_name, &new_name, &old_src, &new_src)
            ),
            OutFormat::Text => print!("{}", render::render_report(&report, use_color(cli.color))),
        }
    }
    Ok(exit_code(external, diff.ops.is_empty()))
}

fn summarize(choice: Option<SummaryArg>, diff: &FileDiff, report: &Report) -> Vec<String> {
    match choice {
        None => Vec::new(),
        Some(SummaryArg::Rules) => rules::summarize(diff),
        Some(SummaryArg::Llm) => {
            let client = AnthropicClient::from_env();
            if client.is_none() {
                eprintln!("semadiff: ANTHROPIC_API_KEY is not set; using the rule-based summary");
            }
            let client = client.as_ref().map(|c| c as &dyn LlmClient);
            llm::summarize_with_fallback(client, diff, report).lines
        }
    }
}

fn exit_code(external: bool, unchanged: bool) -> ExitCode {
    if external || unchanged {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn use_color(choice: ColorArg) -> bool {
    match choice {
        ColorArg::Always => true,
        ColorArg::Never => false,
        ColorArg::Auto => io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none(),
    }
}

/// Reads one side of the comparison. `/dev/null` and `NUL` mean an empty file, which is
/// what git passes for files that were added or removed.
fn read_side(path: &Path) -> Result<String, Box<dyn Error>> {
    if matches!(path.to_str(), Some("/dev/null") | Some("NUL")) {
        return Ok(String::new());
    }
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()).into())
}
