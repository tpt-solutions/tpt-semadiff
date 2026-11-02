//! `semadiff`: structural, AST-aware diff for source files, directories and git revisions.
//!
//! Exit status: `0` no semantic changes, `1` changes found or a CI gate tripped, `2` usage
//! or I/O error. Git's external-diff form always exits `0`, as git requires.

mod dirs;
mod doctor;
mod output;
mod pipeline;
mod rev;
mod setup;

use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use tpt_semadiff_core::diff::{Engine, Options};
use tpt_semadiff_core::parse::SourceLang;
use tpt_semadiff_core::summarize::llm::AnthropicClient;

use pipeline::{Entry, Outcome, Pair, Summary};

#[derive(Parser, Debug)]
#[command(
    name = "semadiff",
    version,
    about = "Structural, AST-aware diff for source files",
    long_about = "Compares two versions of a source file by its syntax tree rather than by \
                  lines. Exit status: 0 no semantic changes, 1 changes found, 2 error.\n\n\
                  As a git difftool: `semadiff git-setup` writes the git config for you.",
    args_conflicts_with_subcommands = true,
    subcommand_negates_reqs = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// OLD and NEW file, or two directories. Git's external-diff form (seven arguments) is
    /// also accepted.
    #[arg(
        num_args = 2..=7,
        value_name = "FILE",
        required_unless_present = "rev",
        conflicts_with = "rev"
    )]
    files: Vec<PathBuf>,

    /// Compare REV with the working tree, or `A..B` (two revisions) or `A...B` (B with the
    /// merge base of A and B). Only files git reports as changed are compared.
    #[arg(long, value_name = "REV")]
    rev: Option<String>,

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
    /// API (needs ANTHROPIC_API_KEY) and falls back to `rules` on failure. Without this
    /// flag, `rules` is used when stdout is a terminal; `off` turns it off there too.
    #[arg(long, value_enum)]
    summary: Option<SummaryArg>,

    /// Browse the changes in an interactive terminal viewer instead of printing. Works on
    /// one file pair only.
    #[arg(long)]
    tui: bool,

    /// CI gate: exit 1 when more than N distinct functions changed across all files.
    #[arg(long, value_name = "N")]
    max_functions: Option<usize>,

    /// CI gate: exit 1 when a changed file cannot be parsed.
    #[arg(long)]
    fail_on_unparseable: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Configure git to use semadiff for `git difftool`.
    GitSetup(GitSetupArgs),
    /// Print a shell completion script to stdout.
    Completions {
        /// The shell to generate for.
        shell: clap_complete::Shell,
    },
    /// Check git, the difftool config, the API key and the built-in grammars.
    Doctor,
}

#[derive(Args, Debug)]
struct GitSetupArgs {
    /// Write to the global git config (the default).
    #[arg(long, conflicts_with = "local")]
    global: bool,

    /// Write to the current repository's config.
    #[arg(long)]
    local: bool,

    /// Also make `git diff` use semadiff, through `diff.external`.
    #[arg(long)]
    external: bool,

    /// Remove the settings instead of writing them.
    #[arg(long)]
    uninstall: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
enum OutFormat {
    /// Boxed per-function summary.
    Text,
    /// Classic line-based unified diff.
    Unified,
    /// Structured semantic diff as JSON.
    Json,
    /// GitHub-flavoured Markdown, for pull request comments.
    Markdown,
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
    /// No summary, even on a terminal.
    Off,
}

fn main() -> ExitCode {
    let mut cli = Cli::parse();
    let result = match cli.command.take() {
        Some(Command::GitSetup(args)) => setup::run(&setup::Setup {
            scope: if args.local {
                setup::Scope::Local
            } else {
                setup::Scope::Global
            },
            external: args.external,
            uninstall: args.uninstall,
        })
        .map(|()| ExitCode::SUCCESS),
        Some(Command::Completions { shell }) => {
            clap_complete::generate(shell, &mut Cli::command(), "semadiff", &mut io::stdout());
            Ok(ExitCode::SUCCESS)
        }
        Some(Command::Doctor) => Ok(doctor::run()),
        None => run_diff(&cli),
    };
    match result {
        Ok(code) => code,
        Err(err) => {
            eprintln!("semadiff: {err}");
            ExitCode::from(2)
        }
    }
}

fn run_diff(cli: &Cli) -> Result<ExitCode, Box<dyn Error>> {
    let options = Options {
        engine: match cli.engine {
            EngineArg::Chawathe => Engine::Chawathe,
            EngineArg::ZhangShasha => Engine::ZhangShasha,
        },
        ignore_formatting: cli.ignore_formatting,
    };
    let summary = summary_choice(cli.summary);
    let llm = llm_client(summary);
    let color = use_color(cli.color);

    if let Some(spec) = &cli.rev {
        let (pairs, skipped) = rev::pairs(spec)?;
        if skipped > 0 {
            eprintln!("semadiff: skipped {skipped} changed file(s) with unsupported extensions");
        }
        return run_multi(cli, pairs, options, summary, llm.as_ref(), color);
    }

    let (old_path, new_path) = match cli.files.len() {
        2 => (&cli.files[0], &cli.files[1]),
        7 => (&cli.files[1], &cli.files[4]),
        n => return Err(format!("expected 2 files, got {n}").into()),
    };
    if old_path.is_dir() || new_path.is_dir() {
        if !(old_path.is_dir() && new_path.is_dir()) {
            return Err("cannot compare a directory with a file".into());
        }
        let pairs = dirs::pairs(old_path, new_path)?;
        return run_multi(cli, pairs, options, summary, llm.as_ref(), color);
    }

    // Git's external-diff protocol treats any non-zero status as a failure, so that form
    // always exits 0 on success. Changes are then only visible in the output.
    let external = cli.files.len() == 7;
    let pair = Pair {
        old_name: old_path.display().to_string(),
        new_name: new_path.display().to_string(),
        old_src: read_side(old_path)?,
        new_src: read_side(new_path)?,
        lang: SourceLang::from_path(new_path).or_else(|| SourceLang::from_path(old_path)),
    };
    let entry = pipeline::diff_pair(pair, options, summary, llm.as_ref());
    if let Some(reason) = entry.fallback_reason() {
        if cli.out == OutFormat::Json || cli.tui {
            return Err(reason.into());
        }
    }

    if cli.tui {
        if let Outcome::Diffed { report, .. } = &entry.outcome {
            tpt_semadiff_tui::run(report)?;
        }
    } else {
        output::print(std::slice::from_ref(&entry), cli.out, color, false)?;
    }
    Ok(finish(cli, std::slice::from_ref(&entry), external))
}

/// Diffs several pairs and prints the ones that changed.
fn run_multi(
    cli: &Cli,
    pairs: Vec<Pair>,
    options: Options,
    summary: Option<Summary>,
    llm: Option<&AnthropicClient>,
    color: bool,
) -> Result<ExitCode, Box<dyn Error>> {
    if cli.tui {
        return Err("--tui shows one file pair; pass OLD and NEW files".into());
    }
    let entries: Vec<Entry> = pairs
        .into_iter()
        .map(|pair| pipeline::diff_pair(pair, options, summary, llm))
        .filter(Entry::changed)
        .collect();
    output::print(&entries, cli.out, color, true)?;
    Ok(finish(cli, &entries, false))
}

/// The exit status for the entries that were printed, after any CI gate.
fn finish(cli: &Cli, entries: &[Entry], external: bool) -> ExitCode {
    if external {
        return ExitCode::SUCCESS;
    }
    let mut failed = entries.iter().any(Entry::changed);
    for problem in gate_problems(cli, entries) {
        eprintln!("semadiff: {problem}");
        failed = true;
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// The CI gate checks that the flags ask for. Each problem is one line for stderr.
fn gate_problems(cli: &Cli, entries: &[Entry]) -> Vec<String> {
    let mut problems = Vec::new();
    if let Some(limit) = cli.max_functions {
        let mut names = BTreeSet::new();
        for entry in entries {
            if let Outcome::Diffed { report, .. } = &entry.outcome {
                names.extend(report.changed_functions());
            }
        }
        if names.len() > limit {
            let list: Vec<String> = names.iter().map(|n| format!("  - {n}")).collect();
            problems.push(format!(
                "{} functions changed, over the limit of {limit}; a human reviewer is required\n{}",
                names.len(),
                list.join("\n")
            ));
        }
    }
    if cli.fail_on_unparseable {
        let bad: Vec<&str> = entries
            .iter()
            .filter(|e| e.changed() && matches!(e.outcome, Outcome::Unparsed { .. }))
            .map(|e| e.pair.new_name.as_str())
            .collect();
        if !bad.is_empty() {
            problems.push(format!(
                "{} file(s) could not be parsed: {}",
                bad.len(),
                bad.join(", ")
            ));
        }
    }
    problems
}

/// `--summary` as given, with the terminal default applied: `rules` on a terminal,
/// nothing otherwise, so that piped output stays plain.
fn summary_choice(arg: Option<SummaryArg>) -> Option<Summary> {
    match arg {
        Some(SummaryArg::Rules) => Some(Summary::Rules),
        Some(SummaryArg::Llm) => Some(Summary::Llm),
        Some(SummaryArg::Off) => None,
        None if io::stdout().is_terminal() => Some(Summary::Rules),
        None => None,
    }
}

/// The API client for `--summary llm`. Missing credentials are reported once here, and
/// the summary then falls back to rules.
fn llm_client(summary: Option<Summary>) -> Option<AnthropicClient> {
    if summary != Some(Summary::Llm) {
        return None;
    }
    let client = AnthropicClient::from_env();
    if client.is_none() {
        eprintln!("semadiff: ANTHROPIC_API_KEY is not set; using the rule-based summary");
    }
    client
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
