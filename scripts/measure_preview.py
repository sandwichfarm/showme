#!/usr/bin/env python3
"""Measure one real CLI preview's first output and peak child memory on Unix."""

import argparse
import json
from pathlib import Path
import resource
import subprocess
import sys
import tempfile
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("input", type=Path)
    parser.add_argument("--frames", type=int, default=1)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="showme-measure-") as temporary:
        output = Path(temporary) / "output.txt"
        command = [str(args.binary.resolve()), "--quiet", "--backend", "unicode", "--frames", str(args.frames), "--output", str(output), str(args.input.resolve())]
        started = time.monotonic()
        process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        first_output = None
        try:
            while process.poll() is None:
                if output.exists() and output.stat().st_size and first_output is None:
                    first_output = time.monotonic() - started
                if time.monotonic() - started > 30:
                    raise TimeoutError("preview exceeded 30 seconds")
                time.sleep(0.002)
            stdout, stderr = process.communicate()
            elapsed = time.monotonic() - started
            if process.returncode or stdout or not output.exists() or not output.stat().st_size:
                raise RuntimeError(f"preview failed: {process.returncode}, {stderr!r}")
            maximum_rss = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
            print(json.dumps({"input": str(args.input), "frames_requested": args.frames,
                              "time_to_first_output_ms_upper_bound": round((first_output or elapsed) * 1000, 2),
                              "elapsed_ms": round(elapsed * 1000, 2),
                              "peak_memory_bytes": maximum_rss if sys.platform == "darwin" else maximum_rss * 1024,
                              "output_bytes": output.stat().st_size}, indent=2))
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()


if __name__ == "__main__":
    main()
