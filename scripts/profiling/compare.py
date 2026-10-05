"""Compare container-local experimental binaries, alternating run order."""

import json
import statistics
import subprocess

workload = "/src/scripts/profiling/workload.py"
variants = ["baseline", "opt3", "bitmap"]
results = {}
for mode in ["cold", "warm", "scan", "regex", "dense"]:
    samples = {name: [] for name in variants}
    for round_number in range(10):
        order = variants if round_number % 2 == 0 else list(reversed(variants))
        for name in order:
            output = subprocess.check_output(
                [
                    "python3",
                    workload,
                    "run",
                    "--mode",
                    mode,
                    "--repeat",
                    "1",
                    "--binary",
                    f"/work/bin/{name}",
                    "--cache",
                    f"/work/comparison-cache/{name}",
                ],
                text=True,
            )
            samples[name].append(json.loads(output)["median_ms"])
    results[mode] = {
        name: {"median_ms": statistics.median(values), "samples_ms": values}
        for name, values in samples.items()
    }
print(json.dumps(results, indent=2))
