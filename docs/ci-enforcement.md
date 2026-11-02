# CI enforcement: limit how many functions a change touches

A pull request that rewrites many functions at once is hard to review, and it is the usual
shape of a large AI-generated change. This recipe fails CI when a change touches more than a
set number of functions, so a person has to look at it.

semadiff has the check built in: `--max-functions N` exits `1` when more than `N` distinct
functions changed. `--fail-on-unparseable` adds a second gate for changed files that do not
parse. A shell script that does the same with `--out json` is kept as a fallback, for an older
binary or a CI system without git revision support.

## Built-in gate

```
semadiff --rev origin/main...HEAD --max-functions 3 --out markdown
```

- `--rev origin/main...HEAD` compares the branch with the point where it forked from `main`.
  Only the files this branch changes are compared.
- `--max-functions 3` counts the distinct function names with a change across those files.
- `--out markdown` prints a summary that can go straight into a pull request comment. It is
  written to stdout, and the gate message to stderr.

The exit status is:

- `0` when the count is at most the limit and nothing else failed,
- `1` when the count is above the limit, or when a gate tripped,
- `2` when semadiff fails, for example when the revision does not exist.

Add `--fail-on-unparseable` to also fail when a changed file cannot be parsed. Without it, such
a file gets a line diff and does not fail the gate.

## GitHub Actions example

The checkout must include the base branch, so fetch it first:

```yaml
- name: Install semadiff
  run: cargo install --git https://github.com/tpt-solutions/tpt-semadiff tpt-semadiff-cli

- name: Limit changed functions
  run: |
    set -o pipefail  # without this, tee's status would hide a failed gate
    git fetch origin "${{ github.base_ref }}"
    semadiff --rev "origin/${{ github.base_ref }}...HEAD" --max-functions 3 --out markdown \
      | tee semadiff.md
  # semadiff.md can be posted as a PR comment, for example with actions/github-script.
```

The step fails when the limit is exceeded. A reviewer must then approve the change, or it must be
split up.

Notes:

- The binary is named `semadiff`. The crate is `tpt-semadiff-cli`.
- `--rev A...B` compares against the merge base, so changes that landed on `main` after the
  branch was cut do not count against the branch.
- Summaries are advisory and are not used here. The gate does not depend on them.
- Counting uses function names. Two functions with the same name in different files are
  counted once.

## Fallback: the shell script

For an older binary, or when the job cannot run `--rev`, the script in
[examples/ci/check-changed-functions.sh](../examples/ci/check-changed-functions.sh) does the
same count. It needs only POSIX `sh`, `grep`, `sed` and `sort`.

```
check-changed-functions.sh [-m MAX] OLD NEW [OLD NEW ...]
```

It runs `semadiff --out json` on each pair, collects the distinct function names that
changed, and exits:

- `0` when the count is at most `MAX` (default 3),
- `1` when it is above `MAX`,
- `2` when semadiff fails.

```yaml
- name: Limit changed functions (script)
  run: |
    git fetch origin "${{ github.base_ref }}"
    files=$(git diff --name-only --diff-filter=AMDR "origin/${{ github.base_ref }}...HEAD" -- '*.rs' '*.py' '*.js' '*.ts' '*.tsx' '*.go')
    args=""
    for f in $files; do
      old=$(mktemp)
      git show "origin/${{ github.base_ref }}:$f" > "$old" 2>/dev/null || cp /dev/null "$old"
      args="$args $old $f"
    done
    [ -n "$args" ] && sh examples/ci/check-changed-functions.sh -m 3 $args
```

Files that did not exist on the base branch are compared against an empty file, so their
functions count as changed.
