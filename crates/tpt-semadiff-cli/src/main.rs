//! `semadiff`: structural, AST-aware diff for source files, directories and git revisions.
//!
//! Exit status: `0` no semantic changes, `1` changes found or a CI gate tripped, `2` usage
//! or I/O error. Git's external-diff form always exits `0`, as git requires.

mod config;
mod detect;
mod dirs;
mod doctor;
mod output;
mod pipeline;
mod rev;
mod setup;

use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use tpt_semadiff_core::diff::{Engine, Options};
use tpt_semadiff_core::parse::SourceLang;
use tpt_semadiff_core::summarize::llm::AnthropicClient;

use detect::{Excludes, Languages};
use output::Brief;
use pipeline::{Entry, Outcome, Pair, Summary};

/// Pairs with more bytes than this get a warning when `--engine zhang-shasha` is used.
const SLOW_ENGINE_BYTES: usize = 200_000;

#[derive(Parser, Debug)]
#[command(
    name = "semadiff",
    version,
    about = "Structural, AST-aware diff for source files",
    long_about = "Compares two versions of a source file by its syntax tree rather than by \
                  lines. Exit status: 0 no semantic changes, 1 changes found, 2 error.\n\n\
                  As a git difftool: `semadiff git-setup` writes the git config for you. \
                  Project settings can go in a .semadiff.toml file (see `semadiff init`).",
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

    /// Matching engine. Defaults to `chawathe`, or the config file's `engine`.
    #[arg(long, value_enum)]
    engine: Option<EngineArg>,

    /// Ignore comments, docstrings, redundant parentheses and trailing commas, so that
    /// edits to them report zero changes.
    #[arg(long)]
    ignore_formatting: bool,

    /// When to colour text output.
    #[arg(long, value_enum, default_value_t = ColorArg::Auto)]
    color: ColorArg,

    /// Add a natural-language summary. `rules` runs offline; `llm` calls the Anthropic
    /// API (needs ANTHROPIC_API_KEY) and falls back to `rules` on failure. Without this
    /// flag, the config file's `summary` applies, then `rules` when stdout is a terminal.
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

    /// Read every file as this language, whatever its extension (rust, python,
    /// javascript, typescript, tsx or go).
    #[arg(long, value_name = "NAME")]
    lang: Option<String>,

    /// Leave out paths matching this glob in directory and revision mode. Relative to the
    /// compared root; `**/` matches at any depth. Repeatable.
    #[arg(long, value_name = "GLOB")]
    exclude: Vec<String>,

    /// Print nothing. The exit status alone says whether anything changed.
    #[arg(long, conflicts_with_all = ["stat", "name_only"])]
    quiet: bool,

    /// One line per changed file, with its change counts.
    #[arg(long, conflicts_with = "name_only")]
    stat: bool,

    /// The path of each changed file, one per line.
    #[arg(long)]
    name_only: bool,
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
    /// Print the man page (roff) to stdout.
    Man,
    /// Write a commented .semadiff.toml in the current directory.
    Init {
        /// Overwrite an existing file.
        #[arg(long)]
        force: bool,
    },
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

    /// Also register a diff driver for `.gitattributes` (`diff.semadiff.command`), and set
    /// `difftool.prompt=false`.
    #[arg(long)]
    driver: bool,

    /// Also add a `git semadiff` alias.
    #[arg(long)]
    alias: bool,

    /// Remove the settings instead of writing them.
    #[arg(long)]
    uninstall: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
enum OutFormat {
    /// Boxed per-function summary.
    Text,
    /// Classic line-based unified diff, with each hunk named after its function.
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

/// The effective settings for one run: flags first, then the config file, then defaults.
struct Plan {
    options: Options,
    summary: Option<Summary>,
    llm: Option<AnthropicClient>,
    color: bool,
    max_functions: Option<usize>,
    fail_on_unparseable: bool,
    languages: Languages,
    excludes: Excludes,
    brief: Brief,
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
            driver: args.driver,
            alias: args.alias,
            uninstall: args.uninstall,
        })
        .map(|()| ExitCode::SUCCESS),
        Some(Command::Completions { shell }) => {
            clap_complete::generate(shell, &mut Cli::command(), "semadiff", &mut io::stdout());
            Ok(ExitCode::SUCCESS)
        }
        Some(Command::Doctor) => Ok(doctor::run()),
        Some(Command::Man) => clap_mangen::Man::new(Cli::command())
            .render(&mut io::stdout())
            .map(|()| ExitCode::SUCCESS)
            .map_err(Into::into),
        Some(Command::Init { force }) => init(force).map(|()| ExitCode::SUCCESS),
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

/// Writes the commented config template into the current directory.
fn init(force: bool) -> Result<(), Box<dyn Error>> {
    let path = Path::new(config::FILE_NAME);
    if path.exists() && !force {
        return Err(format!(
            "{} already exists; pass --force to overwrite it",
            config::FILE_NAME
        )
        .into());
    }
    fs::write(path, config::TEMPLATE)?;
    println!(
        "Wrote {}. Uncomment the settings you want.",
        config::FILE_NAME
    );
    Ok(())
}

/// Combines the flags, the config file found from the current directory, and the defaults.
fn build_plan(cli: &Cli) -> Result<Plan, Box<dyn Error>> {
    let config = config::discover(&env::current_dir()?)?.unwrap_or_default();
    let engine = match cli.engine {
        Some(engine) => engine,
        None => from_config(config.engine.as_deref(), "engine")?.unwrap_or(EngineArg::Chawathe),
    };
    let summary_arg = match cli.summary {
        Some(arg) => Some(arg),
        None => from_config(config.summary.as_deref(), "summary")?,
    };
    let summary = summary_choice(summary_arg);
    let forced = cli
        .lang
        .as_deref()
        .map(|name| {
            SourceLang::from_name(name).ok_or_else(|| {
                let known = SourceLang::ALL.map(SourceLang::name).join(", ");
                format!("unknown language {name:?}; expected one of {known}")
            })
        })
        .transpose()?;
    let mut patterns = config.exclude.clone();
    patterns.extend(cli.exclude.iter().cloned());

    Ok(Plan {
        options: Options {
            engine: match engine {
                EngineArg::Chawathe => Engine::Chawathe,
                EngineArg::ZhangShasha => Engine::ZhangShasha,
            },
            ignore_formatting: cli.ignore_formatting || config.ignore_formatting,
        },
        summary,
        llm: llm_client(summary),
        color: use_color(cli.color),
        max_functions: cli.max_functions.or(config.max_functions),
        fail_on_unparseable: cli.fail_on_unparseable || config.fail_on_unparseable,
        languages: Languages::new(forced, &config.extensions)?,
        excludes: Excludes::new(&patterns)?,
        brief: brief(cli),
    })
}

/// A setting from the config file, parsed the same way as the matching flag.
fn from_config<T: ValueEnum>(value: Option<&str>, key: &str) -> Result<Option<T>, Box<dyn Error>> {
    value
        .map(|v| T::from_str(v, true).map_err(|e| format!(".semadiff.toml: {key}: {e}").into()))
        .transpose()
}

fn brief(cli: &Cli) -> Brief {
    if cli.quiet {
        Brief::Quiet
    } else if cli.name_only {
        Brief::NameOnly
    } else if cli.stat {
        Brief::Stat
    } else {
        Brief::Full
    }
}

fn run_diff(cli: &Cli) -> Result<ExitCode, Box<dyn Error>> {
    let plan = build_plan(cli)?;

    if let Some(spec) = &cli.rev {
        let (pairs, skipped) = rev::pairs(spec, &plan.languages, &plan.excludes)?;
        if skipped > 0 {
            eprintln!("semadiff: skipped {skipped} changed file(s) with unsupported languages");
        }
        return run_multi(cli, &plan, pairs);
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
        let pairs = dirs::pairs(old_path, new_path, &plan.languages, &plan.excludes)?;
        return run_multi(cli, &plan, pairs);
    }

    // Git's external-diff protocol treats any non-zero status as a failure, so that form
    // always exits 0 on success. Changes are then only visible in the output.
    let external = cli.files.len() == 7;
    let old_src = read_side(old_path)?;
    let new_src = read_side(new_path)?;
    let lang = plan
        .languages
        .lang_for(new_path, new_src.lines().next())
        .or_else(|| plan.languages.lang_for(old_path, old_src.lines().next()));
    let pair = Pair {
        old_name: old_path.display().to_string(),
        new_name: new_path.display().to_string(),
        old_src,
        new_src,
        lang,
    };
    warn_slow_engine(std::slice::from_ref(&pair), plan.options.engine);
    let entry = pipeline::diff_pair(pair, plan.options, plan.summary, plan.llm.as_ref());
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
        output::print(
            std::slice::from_ref(&entry),
            cli.out,
            plan.color,
            false,
            plan.brief,
        )?;
    }
    Ok(finish(&plan, std::slice::from_ref(&entry), external))
}

/// Diffs several pairs and prints the ones that changed.
fn run_multi(cli: &Cli, plan: &Plan, pairs: Vec<Pair>) -> Result<ExitCode, Box<dyn Error>> {
    if cli.tui {
        return Err("--tui shows one file pair; pass OLD and NEW files".into());
    }
    warn_slow_engine(&pairs, plan.options.engine);
    let entries: Vec<Entry> =
        pipeline::diff_all(pairs, plan.options, plan.summary, plan.llm.as_ref())
            .into_iter()
            .filter(Entry::changed)
            .collect();
    output::print(&entries, cli.out, plan.color, true, plan.brief)?;
    Ok(finish(plan, &entries, false))
}

/// Zhang-Shasha is exact but slow on large inputs, so say so when it is asked to work on them.
fn warn_slow_engine(pairs: &[Pair], engine: Engine) {
    if engine != Engine::ZhangShasha {
        return;
    }
    let big = pairs
        .iter()
        .filter(|p| p.old_src.len() + p.new_src.len() > SLOW_ENGINE_BYTES)
        .count();
    if big > 0 {
        eprintln!(
            "semadiff: --engine zhang-shasha can be slow; {big} file pair(s) are over 200 KB. \
             The default chawathe engine is much faster on large files."
        );
    }
}

/// The exit status for the entries that were printed, after any CI gate.
fn finish(plan: &Plan, entries: &[Entry], external: bool) -> ExitCode {
    if external {
        return ExitCode::SUCCESS;
    }
    let mut failed = entries.iter().any(Entry::changed);
    for problem in gate_problems(plan, entries) {
        eprintln!("semadiff: {problem}");
        failed = true;
    }
    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// The CI gate checks that the flags or config ask for. Each problem is one line for stderr.
fn gate_problems(plan: &Plan, entries: &[Entry]) -> Vec<String> {
    let mut problems = Vec::new();
    if let Some(limit) = plan.max_functions {
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
    if plan.fail_on_unparseable {
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
