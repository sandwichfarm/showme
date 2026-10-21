#!/usr/bin/env python3
"""Check overall and rendering/media coverage independently."""

import json
from pathlib import Path
import sys

THRESHOLDS = {
    "total": 80.0,
    "src/renderer.rs": 80.0,
    "src/image.rs": 75.0,
    "src/video.rs": 80.0,
}


def check(report):
    data = report["data"]
    if len(data) != 1:
        raise ValueError("expected one merged LLVM coverage report")
    measured = {"total": data[0]["totals"]["lines"]["percent"]}
    for file in data[0]["files"]:
        name = file["filename"].replace("\\", "/")
        for module in THRESHOLDS:
            if name == module or name.endswith("/" + module):
                measured[module] = file["summary"]["lines"]["percent"]
    failures = []
    for module, threshold in THRESHOLDS.items():
        if module not in measured:
            failures.append(f"{module}: missing coverage; required {threshold:.2f}%")
        else:
            actual = measured[module]
            print(f"{module}: {actual:.2f}% (minimum {threshold:.2f}%)")
            if actual < threshold:
                failures.append(f"{module}: {actual:.2f}% < {threshold:.2f}%")
    return failures


if __name__ == "__main__":
    failures = check(json.loads(Path(sys.argv[1]).read_text()))
    for failure in failures:
        print(failure, file=sys.stderr)
    sys.exit(bool(failures))
