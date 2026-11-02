# Usage

## Synopsis

```
semadiff [OPTIONS] OLD NEW
semadiff [OPTIONS] PATH OLD OLD-HEX OLD-MODE NEW NEW-HEX NEW-MODE   # git external diff
```

`OLD` and `NEW` are file paths. The language is chosen from the extension of `NEW`, or of
`OLD` if `NEW` has none it recognizes.

## Options

| Option | Default | Meaning |
| --- | --- | --- |
| `--out text\|unified\|json` | `text` | Output format. See below. |
| `--engine chawathe\|zhang-shasha` | `chawathe` | Matching engine. `chawathe` is GumTree-style and fast. `zhang-shasha` computes a minimum-cost edit mapping and is exact but slow on large files. |
| `--ignore-formatting` | off | Strip comments before diffing, so that comment-only edits report zero changes. Whitespace never affects the result. |
| `--color auto\|always\|never` | `auto` | Colour text output. `auto` colours only a terminal and honours `NO_COLOR`. |
| `--summary rules\|llm` | off | Add a plain-English summary above the changes. |
| `--tui` | off | Browse the changes in an interactive viewer. |
| `-h`, `--help` | | Help. |
| `-V`, `--version` | | Version. |

## Output formats

- **text** (default): one box per function, listing each change as `[INSERT]`, `[DELETE]`,
  `[MOVED]` or `[UPDATE]`. Changes outside any function go in a `Top level` box.
- **unified**: a classic line diff, for when you want the familiar view.
- **json**: the semantic report. See [json-schema.md](json-schema.md).

## Exit status

| Code | Meaning |
| --- | --- |
| `0` | No semantic changes. |
| `1` | Semantic changes found. |
| `2` | Error: bad arguments, unreadable file, syntax error, or unsupported language in `json` or `--tui` mode. |

The seven-argument git external-diff form always exits `0` on success, because git treats
any other status as a failed diff. In text mode, an unsupported language or a file with a syntax error falls back to a line
diff, with a note on stderr. Exit status then reflects whether the two files differ at all.

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
