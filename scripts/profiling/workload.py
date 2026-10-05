"""Deterministic Linux corpus and repeated CLI workloads for perf."""

import argparse
import json
import os
import shutil
import statistics
import subprocess
import time
from pathlib import Path


def generate(root, files, lines):
    root.mkdir(parents=True, exist_ok=True)
    for i in range(files):
        with (root / f"module_{i:04}.rs").open("w") as out:
            for j in range(lines):
                kind = j % 100
                if kind == 0:
                    line = f'fn main() {{ println!("entry point {i}/{j}"); }}'
                elif kind in (3, 7, 15, 33, 67):
                    line = f"use std::collections::HashMap; // import line {j}"
                elif kind == 10:
                    line = f'    eprintln!("error: failed to process item {j}");'
                elif kind == 20:
                    line = (
                        f"impl SubscriptionManager {{ fn handle_{j}(&self) {{ }} }}"
                        if i % 10 == 0
                        else f"    let result_{j} = compute_value({j});"
                    )
                elif kind in (30, 60, 90):
                    line = f"fn process_test_{j}() -> Result<(), Box<dyn std::error::Error>> {{ Ok(()) }}"
                elif kind == 50:
                    line = f"impl Drop for Resource_{i} {{ fn drop(&mut self) {{ cleanup({j}); }} }}"
                else:
                    line = f"    let var_{j} = data.iter().map(|x| x * 2).filter(|x| *x > {j}).collect::<Vec<_>>();"
                out.write(line + "\n")
    print(
        json.dumps(
            {
                "files": files,
                "lines_per_file": lines,
                "bytes": sum(p.stat().st_size for p in root.iterdir()),
            }
        )
    )


def run(args):
    cache = args.cache / args.mode
    env = dict(os.environ, XDG_CACHE_HOME=str(cache))
    modes = {
        "cold": ["-rl", "SubscriptionManager"],
        "warm": ["-rl", "SubscriptionManager"],
        "scan": ["--no-index", "-rl", "SubscriptionManager"],
        "regex": ["--no-index", "-rcE", r"impl\s+Drop"],
        "dense": ["--no-index", "-rn", "let var_"],
    }
    command = [
        str(args.binary),
        f"-j{args.threads}",
        *modes[args.mode],
        str(args.corpus),
    ]
    if args.mode == "warm":
        subprocess.run(command, env=env, stdout=subprocess.DEVNULL, check=True)
    samples = []
    for _ in range(args.repeat):
        if args.mode == "cold":
            shutil.rmtree(cache, ignore_errors=True)
        start = time.perf_counter()
        subprocess.run(
            command, env=env, stdout=subprocess.DEVNULL, check=True, timeout=60
        )
        samples.append((time.perf_counter() - start) * 1000)
    print(
        json.dumps(
            {
                "mode": args.mode,
                "threads": args.threads,
                "repeat": args.repeat,
                "median_ms": statistics.median(samples),
                "min_ms": min(samples),
                "max_ms": max(samples),
                "samples_ms": samples,
            }
        )
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=["generate", "run"])
    parser.add_argument("--corpus", type=Path, default=Path("/work/corpus"))
    parser.add_argument("--cache", type=Path, default=Path("/work/cache"))
    parser.add_argument("--files", type=int, default=512)
    parser.add_argument("--lines", type=int, default=4000)
    parser.add_argument(
        "--binary", type=Path, default=Path("/work/target/release/fastgrep")
    )
    parser.add_argument(
        "--mode", choices=["cold", "warm", "scan", "regex", "dense"], default="cold"
    )
    parser.add_argument("--repeat", type=int, default=10)
    parser.add_argument("--threads", type=int, default=0)
    args = parser.parse_args()
    if args.action == "generate":
        generate(args.corpus, args.files, args.lines)
    else:
        run(args)


if __name__ == "__main__":
    main()
