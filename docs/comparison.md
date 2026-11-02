# Comparison with difftastic and diffsitter

difftastic and diffsitter are also structural diff tools that use tree-sitter. If you are
choosing between them, this page explains what semadiff is designed to do differently. It
does not claim that semadiff is better at everything. Check each tool's own documentation
for its current language list and options, since these projects change.

## What they have in common

All three compare syntax trees rather than lines, so a reformatting or a rename is not
reported as every line that differs.

## What semadiff adds

**Grouping by function.** semadiff reports changes per function. Each changed function gets
a box (or a Markdown bullet) with the changes listed under it, and changes outside any
function go in a top-level group. The aim is a review view, not a line-by-line view.

**Plain-English summaries.** `--summary rules` recognizes some refactorings: a rename, a
reordering of imports, an extracted function, and an inlined variable. When nothing is
recognized it gives counts. `--summary llm` asks the Anthropic API for up to five bullets,
and falls back to the rules when the call fails. The LLM option sends code to a third party;
see [privacy.md](privacy.md). Rules-only use is fully offline.

**CI gates.** `--max-functions N` fails the run when more than `N` distinct functions change,
and `--fail-on-unparseable` fails it when a changed file cannot be parsed. Both set exit
status `1`, so they can gate a pull request directly. See
[ci-enforcement.md](ci-enforcement.md).

**Output for pull requests and tools.** `--out markdown` renders a comment-ready summary per
file. `--out json` gives a structured report, with a documented format in
[json-schema.md](json-schema.md).

**Revision and directory modes.** `--rev A...B` lists the changed files of a branch and
compares each one, and `OLD_DIR NEW_DIR` compares two trees. These run in one command.

## Where the others may be stronger

- **Language coverage.** semadiff supports six languages: Rust, Python, JavaScript,
  TypeScript (with TSX), and Go. difftastic and diffsitter both rely on tree-sitter grammars
  and may support more languages today. If you need a language semadiff does not cover, one
  of them may be the right choice.
- **Side-by-side and terminal display.** difftastic is known for its side-by-side layout. If
  you want a classic visual diff with the structure highlighted inline, it may suit you
  better than semadiff's boxed per-function view.
- **Maturity.** semadiff is pre-1.0, and its first release has not yet been made. The other
  two have been in use for longer.
- **Git integration.** semadiff provides a `git-setup` command that writes the difftool
  config for you. Check each tool's documentation for how it integrates with git. semadiff's
  setup is newer and has had less use outside this project.

## Choosing

Pick semadiff when you want per-function grouping, a CI gate on how much changed, or a
Markdown or JSON report to attach to a review. Pick difftastic or diffsitter when you need
broader language coverage or a different display. The tools can also be used together.
