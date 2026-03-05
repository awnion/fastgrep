# AI Agent Grep Use Cases

Primary consumers: Claude (Claude Code), Cursor, Windsurf, Copilot, Aider, and other coding agents.

## Why this matters

AI agents use grep as their primary code exploration tool. In Claude Code, the `Grep` tool
(backed by `rg` or `grep -r`) is called dozens of times per session. When grep hangs on a
large binary or a massive generated file, the entire agent workflow stalls — the agent is
blocked waiting for output, burning context window and user patience.

## Core use cases (by frequency)

### 1. Symbol lookup (most common)

```bash
grep -rn "class UserService" --include="*.py"
grep -rn "fn walk\b" --include="*.rs"
grep -rn "export function" --include="*.ts"
grep -rn "def test_" --include="*.py"
```

Agent needs: file path + line number. Speed is critical — agents call this 5-20 times
while exploring a codebase before writing code.

### 2. Import / dependency tracing

```bash
grep -rn "from fastapi import" --include="*.py"
grep -rn "use crate::searcher" --include="*.rs"
grep -rn "require('express')" --include="*.js"
```

Agent needs: understand module graph, find where things come from.

### 3. Usage search (who calls this?)

```bash
grep -rn "search_file" --include="*.rs"
grep -rn "handleSubmit" --include="*.tsx"
grep -rn "UserService" --include="*.{py,java}"
```

Agent needs: all call sites to understand impact before refactoring.

### 4. Configuration / string lookup

```bash
grep -rn "DATABASE_URL" -r
grep -rn "OPENAI_API_KEY" -r
grep -rn "localhost:8080" -r
```

Agent needs: find hardcoded values, env vars, config strings.

### 5. Error message tracing

```bash
grep -rn "invalid token" -r
grep -rn "connection refused" -r
grep -rn "Permission denied" -ri
```

Agent needs: find where an error is raised to debug it.

### 6. Pattern/structure discovery

```bash
grep -rn "TODO\|FIXME\|HACK" -r
grep -rn "unsafe " --include="*.rs"
grep -rn "@deprecated" -r
grep -rn "// eslint-disable" -r
```

Agent needs: understand code quality, find tech debt.

### 7. Files-only mode (what files contain X?)

```bash
grep -rl "test" --include="*.py"
grep -rl "migration" -r
```

Agent needs: just file paths for further reading with `cat`/`Read` tool.

### 8. Count mode (how prevalent is X?)

```bash
grep -rc "unwrap()" --include="*.rs"
grep -rc "console.log" --include="*.ts"
```

Agent needs: scope estimation before refactoring.

### 9. Case-insensitive search

```bash
grep -rni "error" --include="*.log"
grep -rni "readme" -r
```

### 10. Fixed-string search (no regex)

```bash
grep -rFn "Vec<Box<dyn Error>>" --include="*.rs"
grep -rFn "{ useState }" --include="*.tsx"
```

Agent needs: search for strings with regex metacharacters safely.

## The "killer" scenarios (where standard grep fails)

These are the cases where `grep -r` can take 10s+ or effectively hang:

### A. Accidental large binary in repo

- `node_modules/.cache/` with large cached files
- A `.wasm` file, `.so`, `.dll` committed by mistake
- A large `.sqlite` database in the repo
- Docker layer cache files
- ML model weights (`.pt`, `.onnx`, `.safetensors`)

### B. Generated / vendored code

- `vendor/` directories with thousands of files
- `generated/` protobuf or gRPC stubs
- Minified JS bundles (`bundle.min.js` at 10+ MB)
- Source maps (`.map` files, often huge)

### C. Log / data files

- Application logs committed to repo
- CSV/JSON test fixtures (100+ MB)
- Seed data SQL dumps

### D. Monorepo scale

- 100k+ files across many packages
- Deep `node_modules` trees (not gitignored properly)

## What agents expect from grep

1. **Fast response** — under 500ms for most queries, under 2s for recursive
2. **Structured output** — `file:line:content` format (parseable)
3. **Bounded output** — agents typically read first 50-200 lines of grep output
4. **Graceful failure** — if grep can't finish quickly, tell the agent WHY so it can adapt
   (e.g., skip a directory, add `--include`, narrow the search)

## Ideal behavior for AI agents

When a search is going to take too long:

```
WARNING: Search is taking unusually long. A large file or binary may be causing slowdown.
Slow files detected:
  - ./data/model.bin (2.3 GB, binary)
  - ./vendor/bundle.min.js (45 MB)

To continue anyway, re-run with FASTGREP_NO_TIMEOUT=1
To skip large files, use --exclude or --include flags.
```

This output is machine-readable — an agent can parse it, decide to add `--exclude`,
and retry immediately without user intervention.

## Benchmark scenarios

For measuring fastgrep improvements with trigram index:

| Scenario                            | What to measure             |
| ----------------------------------- | --------------------------- |
| Clean Rust project (1k files)       | Baseline, should be <100ms  |
| Medium Python project (10k files)   | Symbol lookup latency       |
| Large JS monorepo (100k files)      | Recursive search latency    |
| Repo with 1GB binary blob           | Timeout behavior, not hang  |
| Repo with node_modules (500k files) | Walker + filter performance |
| Repeated searches (warm index)      | Index hit ratio, speedup    |
| Regex with no literal prefix        | Fallback performance        |
| Multiple -e patterns                | Multi-pattern search        |
