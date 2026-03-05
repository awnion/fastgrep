# Environment variables and CLI flags

## Environment variables

| Variable | Default | Description |
| --- | --- | --- |
| `FASTGREP_NO_LIMIT` | (unset) | Set to `1` to disable the file size limit. Allows searching files of any size |
| `FASTGREP_MAX_FILE_SIZE` | `104857600` (100 MiB) | Max file size in bytes. Files larger than this are skipped. Same as `--max-file-size` |
| `FASTGREP_MAX_LINE_LEN` | `15000` | Max line length in bytes before truncation. Set to `0` to disable. Same as `--max-line-len` |

## CLI flags

### Pattern and input

| Flag | Description |
| --- | --- |
| `PATTERN` | Search pattern (positional argument) |
| `-e PATTERN` | Specify pattern explicitly (repeatable for multiple patterns) |
| `-E`, `--extended-regexp` | Extended regex — this is the default, accepted for compatibility |
| `-F`, `--fixed-strings` | Treat pattern as a literal string, not a regex |
| `-i`, `--ignore-case` | Case-insensitive matching |
| `-v`, `--invert-match` | Select non-matching lines |
| `-w`, `--word-regexp` | Match whole words only |

### Output control

| Flag | Description |
| --- | --- |
| `-n`, `--line-number` | Show line numbers |
| `-l`, `--files-with-matches` | Print only filenames of matching files |
| `-c`, `--count` | Print only a count of matching lines per file |
| `--color [auto\|always\|never]` | Colorize output (default: `auto`) |
| `--max-line-len N` | Truncate lines longer than N bytes (default: 15000, 0 = no limit) |

### File selection

| Flag | Description |
| --- | --- |
| `-r`, `--recursive` | Recurse into directories |
| `--include GLOB` | Search only files matching glob (e.g. `--include='*.rs'`) |
| `--exclude GLOB` | Skip files matching glob |
| `--max-file-size N` | Skip files larger than N bytes (default: 100 MiB) |

### Performance

| Flag | Description |
| --- | --- |
| `-j N`, `--threads N` | Number of search threads (default: 0 = all CPUs) |
| `--no-index` | Disable trigram index (alias: `--no-cache`) |

### Exit codes

| Code | Meaning |
| --- | --- |
| `0` | At least one match found |
| `1` | No matches found |
| `2` | Error (bad pattern, file not found, etc.) |
