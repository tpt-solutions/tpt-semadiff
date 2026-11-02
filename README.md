# tpt-semadiff

A structural, AST-aware diff and merge-review tool. It compares two versions of a source
file by their syntax trees rather than by lines, so a reformatting or a rename shows up as
what it is, not as every line that happens to differ.

It is built for reviewing large blocks of AI-generated code, where line diffs are noisy and
easily mislead a reviewer.

```
File: src/main.rs
┌─ Function: calculate_metrics (Lines 40-85) ──────────────────────┐
│ [UPDATE] Changed `sum` to `acc`                                  │
│ [INSERT] Added `if metrics.is_empty() {`                         │
└──────────────────────────────────────────────────────────────────┘
```

## Status

Early development (target release Q3 2026). Supported languages: Rust, Python, JavaScript,
TypeScript (including TSX), and Go. See [todo.md](todo.md) for progress and [spec.txt](spec.txt)
for the design.

## Goals

- Parse source into syntax trees (tree-sitter) and keep formatting where possible.
- Compute a minimal set of semantic operations: insert, delete, move, update.
- Support many languages through tree-sitter grammars.
- Work as a `git difftool`.

## Non-goals

- Replacing the Git object database. tpt-semadiff works on files in the working tree.
- Resolving complex three-way merge conflicts without a human.

## Install

Once v0.1.0 is released, the prebuilt binary installs with one command:

```
# macOS and Linux
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/tpt-solutions/tpt-semadiff/releases/latest/download/tpt-semadiff-cli-installer.sh | sh

# Windows (PowerShell)
powershell -ExecutionPolicy Bypass -c "irm https://github.com/tpt-solutions/tpt-semadiff/releases/latest/download/tpt-semadiff-cli-installer.ps1 | iex"
```

To build from source, you need a Rust toolchain and a C compiler for the tree-sitter grammars.

```
cargo build --release
```

The binary is `target/release/semadiff`.

Then wire it into git once:

```
semadiff git-setup
```

## Usage

```
semadiff OLD NEW                       # boxed per-function summary
semadiff OLD_DIR NEW_DIR               # every changed file in two directory trees
semadiff --rev origin/main...HEAD      # every file this branch changes
semadiff --out unified OLD NEW         # classic line diff
semadiff --out json OLD NEW            # structured report for CI
semadiff --out markdown --rev A..B     # Markdown for a pull request comment
semadiff --ignore-formatting OLD NEW   # comment-only edits report zero changes
semadiff --summary llm OLD NEW         # plain-English summary via the Anthropic API
semadiff --max-functions 3 --rev A...B # CI gate: fail when more than 3 functions change
semadiff --tui OLD NEW                 # interactive viewer
semadiff doctor                        # check git, the difftool and the grammars
```

Exit status: `0` no semantic changes, `1` changes found or a CI gate tripped, `2` error.

Full reference: [docs/usage.md](docs/usage.md). Git setup: [docs/git-integration.md](docs/git-integration.md).
JSON format: [docs/json-schema.md](docs/json-schema.md). CI gate: [docs/ci-enforcement.md](docs/ci-enforcement.md).

## Workspace layout

| Crate | Purpose |
| --- | --- |
| `crates/tpt-semadiff-core` | Parsing, normalization, matching, edit scripts, reports, summaries |
| `crates/tpt-semadiff-cli` | The `semadiff` command |
| `crates/tpt-semadiff-tui` | Interactive terminal viewer (ratatui) |

Design notes: [docs/architecture.md](docs/architecture.md).

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
Copyright TPT Solutions.
