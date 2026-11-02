# Git integration

tpt-semadiff works as a `git difftool`. It is read-only: it shows the structural change and
never writes to the files, so it is not a `mergetool`. Auto-resolving merges is a non-goal.

## Setup

Install the binary so it is on your `PATH`, then:

```
git config --global diff.tool semadiff
git config --global difftool.semadiff.cmd 'semadiff "$LOCAL" "$REMOTE"'
git config --global difftool.prompt false
```

Use it with:

```
git difftool                 # every changed file, one at a time
git difftool -- src/main.rs  # one file
```

To have `git diff` itself use semadiff for supported files, use the external-diff hook:

```
git config --global diff.external semadiff
```

Git then calls `semadiff PATH OLD-FILE OLD-HEX OLD-MODE NEW-FILE NEW-HEX NEW-MODE`, which
semadiff accepts. Files it cannot parse fall back to a line diff, so nothing is lost.
Git requires a zero status from an external diff, so semadiff exits `0` in this form even
when it reports changes. Use the difftool setup above if you want the status to mean
something.

## Exit status with difftool

Git ignores difftool exit codes unless you set `difftool.trustExitCode = true`. With that
set, `0` means no semantic change, `1` means changes, and `2` means error.

## Checking the setup

In any repository with a tracked source file:

```
echo "// note" >> src/lib.rs
git difftool -- src/lib.rs     # comment change: shown
```

Then add `--ignore-formatting` to the command in `difftool.semadiff.cmd` to see comment-only
edits suppressed.
