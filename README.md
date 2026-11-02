# tpt-semadiff

A structural, AST-aware diff and merge-review tool. It compares two versions of a source
file by their syntax trees rather than by lines, so a reformatting or a rename shows up as
what it is, not as every line that happens to differ.

It is built for reviewing large blocks of AI-generated code, where line diffs are noisy and
easily mislead a reviewer.

Quickstart, on two versions of one function (`old.rs` and `new.rs`):

```
$ semadiff old.rs new.rs
File: old.rs -> new.rs
┌─ Function: calculate_metrics (Lines 1-10) ───────┐
│ [MOVED] `if values.is_empty() {` moved to line 2 │
│ [INSERT] Added `if values.is_empty() {`          │
│ [DELETE] Removed `for v in values {`             │
│ [UPDATE] Changed `sum` to `acc`                  │
│ [INSERT] Added `for v in values {`               │
│ [UPDATE] Changed `sum` to `acc`                  │
└──────────────────────────────────────────────────┘
```

Each change is listed under the function it belongs to. The exit status is `1`, since changes
were found. Add `--stat` for one line per file, or `--name-only` for the list of changed files.

## Status

Pre-1.0 (version 0.1.0). Supported languages: Rust, Python, JavaScript, TypeScript (including
TSX), and Go. See [CHANGELOG.md](CHANGELOG.md) for what each release includes,
[todo.md](todo.md) for progress and [spec.txt](spec.txt) for the design.

## Goals

- Parse source into syntax trees (tree-sitter) and keep formatting where possible.
- Compute a minimal set of semantic operations: insert, delete, move, update.
- Support many languages through tree-sitter grammars.
- Work as a `git difftool`, a `git diff` driver, or a CI gate.

## Non-goals

- Replacing the Git object database. tpt-semadiff works on files in the working tree.
- Resolving complex three-way merge conflicts without a human.

## Install

The prebuilt binary installs with one command:

```
# macOS and Linux
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/tpt-solutions/tpt-semadiff/releases/latest/download/tpt-semadiff-cli-installer.sh | sh

# Windows (PowerShell)
powershell -ExecutionPolicy Bypass -c "irm https://github.com/tpt-solutions/tpt-semadiff/releases/latest/download/tpt-semadiff-cli-installer.ps1 | iex"
```

To build from source, you need a Rust toolchain (1.90 or later) and a C compiler for the
tree-sitter grammars.

```
cargo build --release
```

The binary is `target/release/semadiff`.

Then wire it into git once:

```
semadiff git-setup                 # writes the difftool config to your global git config
semadiff git-setup --driver --alias  # also a diff driver for .gitattributes and a `git semadiff` alias
semadiff git-setup --uninstall     # removes what git-setup wrote
```

Run `semadiff doctor` at any time to check git, the difftool config, the API key and the grammars.

## Usage

```
semadiff OLD NEW                       # boxed per-function summary
semadiff OLD_DIR NEW_DIR               # every changed file in two directory trees
semadiff --rev origin/main...HEAD      # every file this branch changes
semadiff --out unified OLD NEW         # classic line diff, each hunk named after its function
semadiff --out json OLD NEW            # structured report for CI
semadiff --out markdown --rev A..B     # Markdown for a pull request comment
semadiff --ignore-formatting OLD NEW   # comment-only edits report zero changes
semadiff --summary llm OLD NEW         # plain-English summary via the Anthropic API
semadiff --max-functions 3 --rev A...B # CI gate: fail when more than 3 functions change
semadiff --fail-on-unparseable --rev A...B  # CI gate: fail when a changed file does not parse
semadiff --stat --rev A...B            # one line per changed file
semadiff --name-only --rev A...B       # just the names of changed files, for scripts
semadiff --quiet OLD NEW               # print nothing; the exit status says whether anything changed
semadiff --exclude 'vendor/**' --rev A...B  # leave paths out of directory and revision mode
semadiff --lang python OLD NEW         # read every file as one language, whatever its extension
semadiff --tui OLD NEW                 # interactive viewer
semadiff init                          # write a commented .semadiff.toml
semadiff doctor                        # check git, the difftool and the grammars
semadiff completions bash              # shell completion script (bash, zsh, fish, powershell, ...)
semadiff man                           # man page, in roff
```

Exit status: `0` no semantic changes, `1` changes found or a CI gate tripped, `2` error.

Other options include `--engine zhang-shasha` (exact but slower matching), `--color always|never`
and `--summary rules|llm|off`. Run `semadiff --help` for the full list.

Project settings (engine, summary, excludes, CI gates, extension mappings) can live in a
`.semadiff.toml`. Flags override it. See [docs/usage.md](docs/usage.md#project-config).

Full reference: [docs/usage.md](docs/usage.md). Git setup: [docs/git-integration.md](docs/git-integration.md).
JSON format: [docs/json-schema.md](docs/json-schema.md). CI gate: [docs/ci-enforcement.md](docs/ci-enforcement.md).
Comparison with difftastic and diffsitter: [docs/comparison.md](docs/comparison.md).
Privacy, including what the LLM summarizer sends: [docs/privacy.md](docs/privacy.md).

## Languages

| Language | Extensions |
| --- | --- |
| Rust | `.rs` |
| Python | `.py` |
| JavaScript | `.js`, `.mjs`, `.cjs`, `.jsx` |
| TypeScript | `.ts`, `.mts`, `.cts` |
| TSX | `.tsx` |
| Go | `.go` |

Other files are compared as plain text. Directory and revision modes skip them.

## Workspace layout

| Crate | Purpose |
| --- | --- |
| `crates/tpt-semadiff-core` | Parsing, normalization, matching, edit scripts, reports, summaries |
| `crates/tpt-semadiff-cli` | The `semadiff` command |
| `crates/tpt-semadiff-tui` | Interactive terminal viewer (ratatui) |

Design notes: [docs/architecture.md](docs/architecture.md). Benchmarks: [docs/benchmarks.md](docs/benchmarks.md).
Contributing: [CONTRIBUTING.md](CONTRIBUTING.md). Reporting vulnerabilities: [SECURITY.md](SECURITY.md).

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
Copyright TPT Solutions.
