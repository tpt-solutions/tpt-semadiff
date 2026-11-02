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
are identical are not listed. `.git` directories are skipped, and so are symbolic links.
Unsupported files are ignored. A directory and a file cannot be compared with each other.

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
| `--ignore-formatting` | off | Strip comments before diffing, so that comment-only edits report zero changes. Whitespace never affects the result. |
| `--color auto\|always\|never` | `auto` | Colour text output. `auto` colours only a terminal and honours `NO_COLOR`. |
| `--summary rules\|llm\|off` | `rules` on a terminal, otherwise `off` | Add a plain-English summary above the changes. `off` turns it off on a terminal too. |
| `--max-functions N` | | CI gate: exit `1` when more than `N` distinct functions changed. See [CI gates](#ci-gates). |
| `--fail-on-unparseable` | off | CI gate: exit `1` when a changed file cannot be parsed. See [CI gates](#ci-gates). |
| `--tui` | off | Browse the changes in an interactive viewer. Works on one file pair only. |
| `-h`, `--help` | | Help. |
| `-V`, `--version` | | Version. |

Summaries are offline by default when you run semadiff in a terminal, so you see them without
asking. When output is piped or redirected, summaries are off unless `--summary` is given.

## Subcommands

### `semadiff git-setup`

Writes the git configuration that makes semadiff the difftool. Run it once; running it again
changes nothing that is already in place.

| Option | Meaning |
| --- | --- |
| `--global` | Write the user's global git config. This is the default. |
| `--local` | Write the current repository's config. Fails outside a repository. |
| `--external` | Also make `git diff` use semadiff, by setting `diff.external`. |
| `--uninstall` | Remove the settings semadiff wrote. `diff.external` is removed only when it names semadiff. |

It sets `difftool.semadiff.cmd` to `semadiff "$LOCAL" "$REMOTE"` and
`difftool.semadiff.trustExitCode` to `false`. Git would otherwise treat semadiff's exit status
`1` (changes found) as a failure. See [git-integration.md](git-integration.md).

### `semadiff completions <shell>`

Prints a completion script to stdout. The shells are `bash`, `elvish`, `fish`, `powershell`
and `zsh`.

```
semadiff completions bash > ~/.local/share/bash-completion/completions/semadiff
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
- **unified**: a classic line diff, for when you want the familiar view.
- **json**: the semantic report. A single file pair prints one object. Directory and revision
  modes print a `{"files": [...]}` object. See [json-schema.md](json-schema.md).
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
bullets. It needs `ANTHROPIC_API_KEY`; `SEMADIFF_LLM_MODEL` overrides the model (default
`claude-haiku-5-5`). If the key is missing, the call fails, or the answer is empty, the
rule-based summary is used instead.

**Privacy:** the change descriptions quote snippets of your code, and `--summary llm` sends
them to Anthropic. Use `--summary rules` for code that must not leave your machine.

## Environment

| Variable | Used by |
| --- | --- |
| `ANTHROPIC_API_KEY` | `--summary llm` |
| `SEMADIFF_LLM_MODEL` | `--summary llm` (optional) |
| `NO_COLOR` | Disables colour under `--color auto` |

## TUI keys

`j`/`k` or the arrow keys move between functions. `q` or `Esc` quits.
