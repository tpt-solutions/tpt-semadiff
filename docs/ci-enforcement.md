# CI enforcement: limit how many functions a change touches

A pull request that rewrites many functions at once is hard to review, and it is the usual
shape of a large AI-generated change. This recipe fails CI when a change touches more than a
set number of functions, so a person has to look at it.

The script is [examples/ci/check-changed-functions.sh](../examples/ci/check-changed-functions.sh).
It needs only POSIX `sh`, `grep`, `sed` and `sort`.

## Usage

```
check-changed-functions.sh [-m MAX] OLD NEW [OLD NEW ...]
```

It runs `semadiff --out json` on each pair, collects the distinct function names that
changed, and exits:

- `0` when the count is at most `MAX` (default 3),
- `1` when it is above `MAX`,
- `2` when semadiff fails.

## GitHub Actions example

Compare each changed source file against the base branch:

```yaml
- name: Install semadiff
  run: cargo install --git https://github.com/tpt-solutions/tpt-semadiff tpt-semadiff-cli

- name: Limit changed functions
  run: |
    git fetch origin "${{ github.base_ref }}"
    files=$(git diff --name-only --diff-filter=AMDR "origin/${{ github.base_ref }}...HEAD" -- '*.rs' '*.py' '*.js' '*.go')
    args=""
    for f in $files; do
      old=$(mktemp)
      git show "origin/${{ github.base_ref }}:$f" > "$old" 2>/dev/null || cp /dev/null "$old"
      args="$args $old $f"
    done
    [ -n "$args" ] && sh examples/ci/check-changed-functions.sh -m 3 $args
```

Notes:

- The binary is named `semadiff`. The crate is `tpt-semadiff-cli`.
- Files that did not exist on the base branch are compared against an empty file, so their
  functions count as changed.
- Summaries (`--summary`) are advisory and are not used here.
- Counting uses function names. Two functions with the same name in different files are
  counted once.
