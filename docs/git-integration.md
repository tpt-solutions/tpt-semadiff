# Git integration

tpt-semadiff works as a `git difftool`. It is read-only: it shows the structural change and
never writes to the files, so it is not a `mergetool`. Auto-resolving merges is a non-goal.

## Setup

Install the binary so it is on your `PATH`, then run:

```
semadiff git-setup
```

This writes the global git config:

```
[difftool "semadiff"]
	cmd = semadiff \"$LOCAL\" \"$REMOTE\"
	trustExitCode = false
```

Running it again changes nothing that is already in place. Use `--local` to write the current
repository's config instead of the global one. Use `--uninstall` to remove the settings
again. Uninstall never touches a `diff.external` that names some other tool.

Use it with:

```
git difftool                 # every changed file, one at a time
git difftool -- src/main.rs  # one file
git difftool HEAD~1 HEAD     # two revisions
```

By default `git difftool` asks before each file. Set `difftool.prompt` to `false` to skip the
question.

To have `git diff` itself use semadiff for supported files, add `--external`:

```
semadiff git-setup --external
```

This sets `diff.external`. Git then calls `semadiff PATH OLD-FILE OLD-HEX OLD-MODE NEW-FILE
NEW-HEX NEW-MODE`, which semadiff accepts. Files it cannot parse fall back to a line diff, so
nothing is lost. Git requires a zero status from an external diff, so semadiff exits `0` in
this form even when it reports changes. Use the difftool setup if you want the status to mean
something.

The same config can be written by hand with `git config`, if you prefer to manage it yourself:

```
git config --global difftool.semadiff.cmd 'semadiff "$LOCAL" "$REMOTE"'
git config --global difftool.semadiff.trustExitCode false
```

## Revision mode without difftool

`semadiff --rev` does not need any git config. It lists the files git reports as changed, and
compares each one. Run it inside a repository:

```
semadiff --rev HEAD~1                 # the last commit, plus uncommitted work
semadiff --rev origin/main...HEAD     # everything this branch changes
```

See [usage.md](usage.md#input-modes) for every form.

## Exit status with difftool

Git ignores difftool exit codes unless `difftool.trustExitCode` is `true`. `git-setup` writes
`false`, so git never treats semadiff's `1` (changes found) as a failure. If you set it to
`true` yourself, `0` means no semantic change, `1` means changes, and `2` means error.

## Checking the setup

`semadiff doctor` prints the state of git, the difftool config, the API key and the grammars.

To check it by hand, in any repository with a tracked source file:

```
echo "// note" >> src/lib.rs
git difftool -- src/lib.rs     # comment change: shown
```

Then add `--ignore-formatting` to the command in `difftool.semadiff.cmd` to see comment-only
edits suppressed.
