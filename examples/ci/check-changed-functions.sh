#!/bin/sh
# Fails when a change touches more functions than allowed.
#
# Usage: check-changed-functions.sh [-m MAX] OLD NEW [OLD NEW ...]
#
# Counts the distinct functions that semadiff reports as changed across the given file
# pairs, and exits 1 if that count exceeds MAX (default 3). Exits 2 on a semadiff error.
# Set SEMADIFF to the binary if it is not on PATH.
#
# Example CI rule: "no PR may modify more than 3 functions without a human reviewer".
# Needs only POSIX sh, grep, sed and sort.

max=3
if [ "$1" = "-m" ]; then
    max=$2
    shift 2
fi

bin=${SEMADIFF:-semadiff}
names=$(mktemp)
trap 'rm -f "$names"' EXIT

if [ $# -lt 2 ] || [ $(( $# % 2 )) -ne 0 ]; then
    echo "usage: $0 [-m MAX] OLD NEW [OLD NEW ...]" >&2
    exit 2
fi

while [ $# -ge 2 ]; do
    # Exit 0 means no change and 1 means changes; anything else is an error.
    json=$("$bin" --out json "$1" "$2")
    status=$?
    if [ "$status" -ge 2 ]; then
        echo "semadiff failed on $1 $2" >&2
        exit 2
    fi
    printf '%s\n' "$json" | grep -o '"function": "[^"]*"' | sed 's/^"function": "//; s/"$//' >> "$names"
    shift 2
done

count=$(sort -u "$names" | grep -c .)
echo "changed functions: $count (limit $max)"
sort -u "$names" | sed 's/^/  - /'

if [ "$count" -gt "$max" ]; then
    echo "too many functions changed; a human reviewer is required" >&2
    exit 1
fi
exit 0
