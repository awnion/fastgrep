import argparse
import json
import os
import statistics
import subprocess
import tempfile
import time
from pathlib import Path

parser = argparse.ArgumentParser(
    description="Compare cold and warm recursive searches on an isolated corpus."
)
parser.add_argument("--before", type=Path, required=True)
parser.add_argument("--after", type=Path, required=True)
args = parser.parse_args()
bins = {"before": args.before.resolve(), "after": args.after.resolve()}
with tempfile.TemporaryDirectory(prefix="fg-perf-") as temp:
    base = Path(temp)
    data = base / "data"
    data.mkdir()
    for i in range(200):
        body = "".join(
            f"normal source line {j} file {i} with alpha beta gamma and return value\n"
            for j in range(1000)
        )
        if i == 0:
            body += "UniqueNeedleToken\n"
        (data / f"{i}.txt").write_text(body)
    samples = {
        mode: {name: [] for name in bins} for mode in ["cold", "warm", "no-index"]
    }

    def run(name, home, no_index=False):
        home.mkdir(exist_ok=True)
        env = dict(os.environ, HOME=str(home), XDG_CACHE_HOME=str(home / "cache"))
        args = [str(bins[name]), "-r", "UniqueNeedleToken", str(data)]
        if no_index:
            args.insert(1, "--no-index")
        start = time.perf_counter()
        subprocess.run(
            args,
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            check=True,
            timeout=30,
        )
        return (time.perf_counter() - start) * 1000

    for i in range(6):
        for name in list(bins) if i % 2 == 0 else list(reversed(bins)):
            samples["cold"][name].append(run(name, base / f"cold-{name}-{i}"))
    for name in bins:
        run(name, base / f"warm-{name}")
    for i in range(20):
        for name in list(bins) if i % 2 == 0 else list(reversed(bins)):
            samples["warm"][name].append(run(name, base / f"warm-{name}"))
            samples["no-index"][name].append(run(name, base / f"warm-{name}", True))
    result = {
        "files": 200,
        "lines_per_file": 1000,
        "bytes": sum(p.stat().st_size for p in data.iterdir()),
        "median_ms": {
            mode: {name: round(statistics.median(v), 3) for name, v in cases.items()}
            for mode, cases in samples.items()
        },
    }
    print(json.dumps(result, indent=2))
