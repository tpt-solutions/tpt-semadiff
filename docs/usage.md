# Usage

## Synopsis

```
semadiff [OPTIONS] OLD NEW                  # two files, or two directories
semadiff [OPTIONS] --rev REV                # REV against the working tree
semadiff [OPTIONS] --rev A..B               # two revisions
semadiff [OPTIONS] --rev A...B              # B against the merge base of A and B
semadiff [OPTIONS] PATH OLD OLD-HEX OLD-MODE NEW NEW-HEX NEW-MODE   # git external diff
semadiff git-setup [--global | --local] [--external] [--uninstall]
semadiff completions <shell>
semadiff doctor
```

## Languages

The language is chosen from the file extension.

| Language | Extensions |
| --- | --- |
| Rust | `.rs` |
| Python | `.py` |
| JavaScript | `.js`, `.mjs`, `.cjs`, `.jsx` |
| TypeScript | `.ts`, `.mts`, `.cts` |
| TSX | `.tsx` |
| Go | `.go` |

Other files are compared as plain text, in the line-diff fallback described under
[Exit status](#exit-status).

## Input modes

**Two files.** `semadiff OLD NEW` compares the two files. The language is taken from `NEW`,
or from `OLD` if `NEW` has an extension that is not recognized.

**Two directories.** `semadiff OLD_DIR NEW_DIR` pairs up the supported files in both trees by
relative path. A file that exists on one side only is compared with an empty file. Files that
are identical are not listed. `.git` directories and symbolic links are skipped, and so are
paths matched by a `.gitignore` file and the [default excludes](#excludes). Files in unsupported
languages are ignored. A directory and a file cannot be compared with each other.

**Git revisions.** `--rev` asks git which files changed, then compares each one.

| Spec | Old side | New side |
| --- | --- | --- |
| `--rev REV` | `REV` | the working tree |
| `--rev A..B` | `A` | `B` |
| `--rev A...B` | merge base of `A` and `B` | `B` |

Only files git reports as changed are compared, and renames are followed. Changed files in
unsupported languages are skipped, and the count is printed on stderr. A file whose change
is only a rename or only formatting, under `--ignore-formatting`, is not listed.

`--rev` must be run inside a git repository and cannot be combined with `OLD NEW` arguments.
In directory and revision modes, the output lists only the files that have semantic changes.

## Options

| Option | Default | Meaning |
| --- | --- | --- |
| `--out text\|unified\|json\|markdown` | `text` | Output format. See [Output formats](#output-formats). |
| `--rev REV` | | Compare git revisions instead of files. See [Input modes](#input-modes). |
| `--engine chawathe\|zhang-shasha` | `chawathe` | Matching engine. `chawathe` is GumTree-style and fast. `zhang-shasha` computes a minimum-cost edit mapping and is exact but slow on large files. |
| `--ignore-formatting` | off | Ignore formatting-only edits. See [What `--ignore-formatting` ignores](#what---ignore-formatting-ignores). |
| `--lang NAME` | from the file | Read every file as this language: `rust`, `python`, `javascript`, `typescript`, `tsx` or `go`. |
| `--exclude GLOB` | | Leave out matching paths in directory and revision mode. Repeatable. See [Excludes](#excludes). |
| `--quiet` | off | Print nothing. The exit status says whether anything changed. |
| `--stat` | off | One line per changed file, with its change counts, then a total. |
| `--name-only` | off | The path of each changed file, one per line. |
| `--color auto\|always\|never` | `auto` | Colour text output. `auto` colours only a terminal and honours `NO_COLOR`. |
| `--summary rules\|llm\|off` | `rules` on a terminal, otherwise `off` | Add a plain-English summary above the changes. `off` turns it off on a terminal too. |
| `--max-functions N` | | CI gate: exit `1` when more than `N` distinct functions changed. See [CI gates](#ci-gates). |
| `--fail-on-unparseable` | off | CI gate: exit `1` when a changed file cannot be parsed. See [CI gates](#ci-gates). |
| `--tui` | off | Browse the changes in an interactive viewer. Works on one file pair only. |

`--quiet`, `--stat` and `--name-only` replace the `--out` format. Each run is one of them or none.
`--quiet` cannot be combined with the other two.

Flags override the [project config](#project-config). Options that take a value, and the
booleans, can all be set there.
| `-h`, `--help` | | Help. |
| `-V`, `--version` | | Version. |

Summaries are offline by default when you run semadiff in a terminal, so you see them without
asking. When output is piped or redirected, summaries are off unless `--summary` is given.

## Project config

Put a `.semadiff.toml` in the directory you run semadiff from, or in one of its parents. The
nearest file applies. `semadiff init` writes a template with every setting commented out.

```toml
engine = "chawathe"            # or "zhang-shasha"
ignore-formatting = false
summary = "rules"              # "rules", "llm" or "off"
max-functions = 3              # CI gate
fail-on-unparseable = false    # CI gate
exclude = ["docs/**", "**/generated/**"]

[extensions]
mjs = "javascript"             # extension (no dot) to language name
```

Unknown keys are an error, so a misspelt setting fails loudly. Flags on the command line win over
the file. A boolean in the file can be turned on by a flag but not turned off.

## Excludes

In directory and revision mode, these paths are left out by default:

- `node_modules/` and `vendor/` directories.
- Minified and bundled JavaScript (`*.min.js`, `*.min.mjs`, `*.min.cjs`, `*.bundle.js`).
- Generated code (`*.generated.*`, `*_generated.go`, `*.pb.go`).
- Lock files (`Cargo.lock`, `package-lock.json`, `yarn.lock`, `pnpm-lock.yaml`, `go.sum`,
  `poetry.lock`).

`--exclude GLOB` and the `exclude` list in the config add to these. Patterns are matched against
the path relative to the compared directory or the repository root. `*` does not cross `/`, so
use `**/` to match at any depth: `--exclude "**/*.snap"`.

## Languages

A file is read as the language its extension names. A file with no extension is read as a
script if its first line is a `#!` line for Python or Node. `--lang` overrides the choice for
every file, and the config's `[extensions]` table maps extra extensions onto a language.

## What `--ignore-formatting` ignores

With `--ignore-formatting`, these are removed before the diff, on both sides:

- Comments.
- Python docstrings.
- Parentheses around a single expression: `(a)` matches `a`. Grouping that changes the meaning
  still counts. `(a + b) * c` and `a + b * c` differ.
- A trailing comma after the last item of a list, argument list, parameter list, or object or
  array literal: `f(a, b,)` matches `f(a, b)`. A Python one-element tuple keeps its comma.

Whitespace never affects the result. Anything else is compared, so a change to a token, an
argument order or a statement is always reported.

## Subcommands

### `semadiff git-setup`

Writes the git configuration that makes semadiff the difftool. Run it once; running it again
changes nothing that is already in place.

| Option | Meaning |
| --- | --- |
| `--global` | Write the user's global git config. This is the default. |
| `--local` | Write the current repository's config. Fails outside a repository. |
| `--external` | Also make `git diff` use semadiff, by setting `diff.external`. |
| `--driver` | Also register a diff driver, `diff.semadiff.command`, for `.gitattributes` to select, and set `difftool.prompt=false`. |
| `--alias` | Also add a `git semadiff` alias. |
| `--uninstall` | Remove the settings semadiff wrote. `diff.external`, the driver and the alias are removed only when they name semadiff. |

It sets `difftool.semadiff.cmd` to `semadiff "$LOCAL" "$REMOTE"` and
`difftool.semadiff.trustExitCode` to `false`. Git would otherwise treat semadiff's exit status
`1` (changes found) as a failure. See [git-integration.md](git-integration.md).

### `semadiff init`

Writes a commented `.semadiff.toml` in the current directory. Refuses to overwrite one unless
`--force` is given.

### `semadiff completions <shell>`

Prints a completion script to stdout. The shells are `bash`, `elvish`, `fish`, `powershell`
and `zsh`.

```
semadiff completions bash > ~/.local/share/bash-completion/completions/semadiff
```

Other shells: `semadiff completions zsh > "${fpath[1]}/_semadiff"`, then restart the shell. For
PowerShell, add `semadiff completions powershell | Out-String | Invoke-Expression` to your profile.

### `semadiff man`

Prints the man page in roff. To install it on Linux or macOS:

```
semadiff man > ~/.local/share/man/man1/semadiff.1
```

### `semadiff doctor`

Checks the setup and prints one line per check:

- git is on `PATH`, and the version.
- `semadiff` is on `PATH`.
- The difftool and `diff.external` settings.
- Whether `ANTHROPIC_API_KEY` is set. The value is never printed.
- Every built-in grammar loads.

It exits `1` when git is missing or a grammar fails to load. Other lines are advice.

## Output formats

- **text** (default): one box per function, listing each change as `[INSERT]`, `[DELETE]`,
  `[MOVED]` or `[UPDATE]`. Changes outside any function go in a `Top level` box. Directory and
  revision modes print one section per changed file.
- **unified**: a classic line diff, for when you want the familiar view. Each hunk header names
  the function its first change falls in, as `git diff` does, so `@@ -3,4 +3,4 @@ total` shows
  where the change is.
- **json**: the semantic report. A single file pair prints one object. Directory and revision
  modes print a `{"schema_version": 1, "files": [...]}` object. See
  [json-schema.md](json-schema.md) and [semadiff.schema.json](semadiff.schema.json).
- **markdown**: GitHub-flavoured Markdown, for a pull request comment. Each file gets a heading,
  the change counts, and a bullet per function with its changes nested underneath. A file that
  cannot be parsed gets its line diff in a fenced block.

## Exit status

| Code | Meaning |
| --- | --- |
| `0` | No semantic changes. |
| `1` | Semantic changes found, or a CI gate tripped. |
| `2` | Error: bad arguments, unreadable file, syntax error, or unsupported language in `json` or `--tui` mode. |

The seven-argument git external-diff form always exits `0` on success, because git treats
any other status as a failed diff. CI gates do not apply in that form.

In text and unified modes, a file in an unsupported language, or one with a syntax error, falls
back to a line diff, with a note on stderr. Its exit status then reflects whether the two files
differ at all. In `json` and `--tui` modes it is an error instead.

## CI gates

`--max-functions N` and `--fail-on-unparseable` turn semadiff into a CI check. Each problem
is printed on stderr, and the exit status is `1`.

- `--max-functions N` counts the distinct function names with a change across all the files
  compared. The count is over names, so a function that appears in two files is counted once.
  It fails when the count is above `N`.
- `--fail-on-unparseable` fails when a file that changed could not be parsed.

For a worked example, see [ci-enforcement.md](ci-enforcement.md).

## Summaries

`--summary rules` is offline and deterministic. It recognizes:

- **Rename**: `Renamed \`sum\` to \`acc\` (3 occurrences)`
- **Reordered imports**: only import declarations changed, and some moved.
- **Extracted function**: a new function received statements from another function.
- **Inlined variable**: a binding was removed, its right-hand side now appears verbatim,
  and the name no longer occurs.

When nothing is recognized it gives a count of insertions, deletions, moves and updates.

`--summary llm` sends the change list to the Anthropic API and asks for up to five
bullets. It needs `ANTHROPIC_API_KEY`. If the key is missing, the call fails, or the answer is
empty, the rule-based summary is used instead. The text output then says why, for example
`Summary (rules; LLM not used: ANTHROPIC_API_KEY is not set):`. The JSON output carries the same
information in `summary_source` and `summary_fallback`.

**Privacy:** the change descriptions quote snippets of your code, and `--summary llm` sends
them to Anthropic. Use `--summary rules` for code that must not leave your machine.

## Environment

| Variable | Used by |
| --- | --- |
| `ANTHROPIC_API_KEY` | `--summary llm` |
| `SEMADIFF_LLM_MODEL` | `--summary llm` (optional; default `claude-haiku-5-5`) |
| `SEMADIFF_LLM_MAX_TOKENS` | `--summary llm` (optional; default `512`) |
| `SEMADIFF_LLM_SYSTEM` | `--summary llm` (optional; replaces the built-in system prompt) |
| `NO_COLOR` | Turns colour off under `--color auto` |

For what the LLM summary sends, see [privacy.md](privacy.md).
| `NO_COLOR` | Disables colour under `--color auto` |

## TUI keys

`j`/`k` or the arrow keys move between functions. `q` or `Esc` quits.
