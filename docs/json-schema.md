# JSON report schema

`semadiff --out json OLD NEW` prints one JSON object. Keys are stable within a minor
version; new keys may be added.

```json
{
  "old_name": "src/main.rs",
  "new_name": "src/main.rs",
  "language": "rust",
  "engine": "chawathe",
  "counts": { "insert": 1, "delete": 0, "move": 0, "update": 3 },
  "groups": [
    {
      "function": "calculate_metrics",
      "start_line": 40,
      "end_line": 85,
      "changes": [
        {
          "op": "update",
          "node_kind": "identifier",
          "old_line": 42,
          "new_line": 42,
          "description": "Changed `sum` to `acc`"
        }
      ]
    }
  ],
  "summaries": ["Renamed `sum` to `acc` (3 occurrences)"]
}
```

## Fields

| Field | Type | Meaning |
| --- | --- | --- |
| `old_name`, `new_name` | string | The paths as given on the command line. |
| `language` | string | `rust`, `python`, `javascript` or `go`. |
| `engine` | string | `chawathe` or `zhang-shasha`. |
| `counts` | object | Number of changes per `op`: `insert`, `delete`, `move`, `update`. |
| `groups` | array | Changes grouped by enclosing function, ordered by first line. |
| `groups[].function` | string or null | Function name, or `null` for top-level changes. |
| `groups[].start_line`, `end_line` | integer | 1-based. The function's extent, or the span of its changes at top level. |
| `groups[].changes` | array | Changes in source order. |
| `changes[].op` | string | `insert`, `delete`, `move` or `update`. |
| `changes[].node_kind` | string | Syntax-tree kind of the affected node, e.g. `identifier`, `let_declaration`. For updates, the new kind. |
| `changes[].old_line` | integer or null | Line in the old file. Null for inserts. |
| `changes[].new_line` | integer or null | Line in the new file. Null for deletes. |
| `changes[].description` | string | Human-readable text, safe to display. |
| `summaries` | array of string | Present only when `--summary` was given. |

## Semantics

- `insert` and `delete` are reported once per maximal unmatched subtree, not once per node.
- `move` means a node changed parent or position among its siblings. Moved punctuation and
  keywords are not reported on their own.
- `update` means a matched node changed its kind or its value.
- A change with no enclosing function appears in the group whose `function` is `null`.
- Summaries are advisory. Do not key CI rules on them.

## Using it in CI

See [ci-enforcement.md](ci-enforcement.md) for an example that counts changed functions.
