# GNU grep compatibility

Flags and options not yet supported by fastgrep.

**Perf impact** — potential performance degradation: 🟢 none, 🟡 minor, 🔴 significant.
**Complexity** — implementation effort: 🟢 easy, 🟡 moderate, 🔴 hard.

## Pattern selection

| Flag | Description | Perf impact | Complexity |
|------|-------------|:-----------:|:----------:|
| `-G, --basic-regexp` | Basic regular expressions (BRE) | 🟢 | 🟡 |
| `-P, --perl-regexp` | Perl-compatible regular expressions | 🟡 | 🔴 |
| `-z, --null-data` | Lines end with NUL instead of newline | 🟡 | 🔴 |

## Output control

| Flag | Description | Perf impact | Complexity |
|------|-------------|:-----------:|:----------:|
| `--line-buffered` | Flush output on every line | 🟡 | 🟢 |

## File and directory handling

| Flag | Description | Perf impact | Complexity |
|------|-------------|:-----------:|:----------:|
| `-R, --dereference-recursive` | Recurse and follow symlinks | 🟡 | 🟡 |
| `-d, --directories=ACTION` | How to handle directories (`read`, `recurse`, `skip`) | 🟢 | 🟡 |
| `-D, --devices=ACTION` | How to handle devices/FIFOs (`read`, `skip`) | 🟢 | 🟡 |

## Binary file handling

| Flag | Description | Perf impact | Complexity |
|------|-------------|:-----------:|:----------:|
| `--binary-files=TYPE` | Treat binary files as `binary`, `text`, or `without-match` | 🟢 | 🟡 |

## Miscellaneous

| Flag | Description | Perf impact | Complexity |
|------|-------------|:-----------:|:----------:|
| `-NUM` | Shorthand for `--context=NUM` | 🟢 | 🟡 |
