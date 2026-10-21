#!/usr/bin/env python3
"""Require an explicit owner and disposition for every informational advisory."""

import json
from pathlib import Path
import sys


def check(report, dispositions):
    failures = []
    if report.get("vulnerabilities", {}).get("count", 0):
        failures.append("vulnerabilities remain unresolved")
    for category, warnings in report.get("warnings", {}).items():
        for warning in warnings:
            advisory = warning.get("advisory") or {}
            package = warning["package"]["name"]
            identifier = advisory.get("id", f"{category}:{package}")
            disposition = dispositions.get(identifier)
            if not disposition or disposition.get("package") != package:
                failures.append(f"{identifier} ({package}): missing reviewed disposition")
            elif not disposition.get("owner") or not disposition.get("disposition"):
                failures.append(f"{identifier}: owner and disposition must both be present")
            else:
                print(f"VISIBLE WARNING {identifier} ({package}); owner: {disposition['owner']}; {disposition['disposition']}")
    return failures


if __name__ == "__main__":
    report = json.loads(Path(sys.argv[1]).read_text())
    dispositions = json.loads((Path(__file__).resolve().parent.parent / "docs/dependency-advisories.json").read_text())
    failures = check(report, dispositions)
    for failure in failures:
        print(failure, file=sys.stderr)
    sys.exit(bool(failures))
