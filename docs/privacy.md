# Privacy

semadiff runs on your machine. Only one option sends anything over the network:
`--summary llm`. Everything else, including the default summary, is offline.

## What leaves the machine

Only `--summary llm` makes a network call. It sends a request to the Anthropic Messages API
at `https://api.anthropic.com/v1/messages`, over HTTPS.

The request contains:

- The language of the file, for example `rust`.
- The file names as you passed them to semadiff. In `OLD NEW` mode these are the two paths.
  In directory and `--rev` mode they are the relative paths of the changed files.
- For each change, a short description and the name of the function it is in. The
  description is built from the snippets below. It also says whether the change is an
  insertion, deletion, move or update, and gives the line number for a move.
- A fixed instruction that asks for at most five summary lines.

The snippets are the first non-blank line of the changed code, with whitespace collapsed
and cut to 60 characters (the last one replaced by `…`). For an update, the old and new
snippets are both sent. Nothing else from your files is sent: not the rest of the code, not
the unchanged parts of a function, and not your git history.

The request also includes the model name (`claude-haiku-5-5` by default, or the value of
`SEMADIFF_LLM_MODEL`) and your API key, in the `x-api-key` header.

## What stays on the machine

- `--summary rules` (the default in a terminal) is fully offline.
- `--summary off`, and all the other output modes (`text`, `unified`, `json`, `markdown`),
  are fully offline.
- The `--tui` viewer is offline.
- The CI gates (`--max-functions`, `--fail-on-unparseable`) are offline.
- `semadiff doctor` is offline. It only checks whether `ANTHROPIC_API_KEY` is set. It never
  prints the value.
- Git is run locally. semadiff does not contact a remote.

## Your API key

`--summary llm` reads the key from the `ANTHROPIC_API_KEY` environment variable. semadiff
does not store it, write it to a file, or print it in any output. If the key is missing,
semadiff prints a notice on stderr and uses the rule-based summary.

Anthropic handles the request under its own terms and data policies. See Anthropic's
documentation for how it retains API data.

## Keeping code local

If the code must not leave your machine, do not pass `--summary llm`, and do not set
`ANTHROPIC_API_KEY` in the environment where you run semadiff. Use `--summary rules` if you
want a plain-English summary.
