# GNU grep compatibility

Flags and options not yet supported by fastgrep.

## Pattern selection

| Flag | Description |
|------|-------------|
| `-G, --basic-regexp` | Basic regular expressions (BRE) |
| `-P, --perl-regexp` | Perl-compatible regular expressions |
| `-f, --file=FILE` | Read patterns from file |
| `--no-ignore-case` | Undo `-i` |
| `-x, --line-regexp` | Match whole lines only |
| `-z, --null-data` | Lines end with NUL instead of newline |

## Output control

| Flag | Description |
|------|-------------|
| `-b, --byte-offset` | Print byte offset with output |
| `--label=LABEL` | Label for stdin in output |
| `-T, --initial-tab` | Align tabs in output |
| `-Z, --null` | Print NUL after filename |
| `--line-buffered` | Flush output on every line |

## File and directory handling

| Flag | Description |
|------|-------------|
| `-R, --dereference-recursive` | Recurse and follow symlinks |
| `--exclude-from=FILE` | Read exclude patterns from file |
| `-d, --directories=ACTION` | How to handle directories (`read`, `recurse`, `skip`) |
| `-D, --devices=ACTION` | How to handle devices/FIFOs (`read`, `skip`) |

## Binary file handling

| Flag | Description |
|------|-------------|
| `--binary-files=TYPE` | Treat binary files as `binary`, `text`, or `without-match` |
| `-a, --text` | Treat binary files as text |
| `-I` | Ignore binary files |
| `-U, --binary` | Do not strip CR at EOL (Windows) |

## Miscellaneous

| Flag | Description |
|------|-------------|
| `-s, --no-messages` | Suppress error messages |
| `-NUM` | Shorthand for `--context=NUM` |
| `--group-separator=SEP` | Custom separator between context groups |
| `--no-group-separator` | No separator between context groups |
| `--colour` | Alias for `--color` |
