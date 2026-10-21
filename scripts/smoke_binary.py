#!/usr/bin/env python3
"""Run actual image/PDF/video previews from a built binary or release archive."""

import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def smoke(binary, video, directory):
    fixtures = Path(__file__).resolve().parent.parent / "tests/fixtures"
    environment = os.environ.copy()
    # Archive checks must discover the bundled runtime beside the executable.
    environment.pop("SHOWME_PDFIUM_PATH", None)
    environment.pop("SHOWME_PDFIUM_DIR", None)
    cases = ["red.ppm", "two-pages.pdf"]
    if video:
        cases.append("short.mkv")
    for fixture in cases:
        output = directory / (fixture + ".txt")
        result = subprocess.run([str(binary), "--quiet", "--backend", "unicode", "--frames", "1", "--width", "4", "--height", "2", "--output", str(output), str(fixtures / fixture)],
                                cwd=directory, env=environment, capture_output=True, timeout=30)
        if result.returncode or result.stdout or not output.exists() or not output.stat().st_size:
            raise RuntimeError(f"{fixture}: exit={result.returncode}, stdout={result.stdout!r}, stderr={result.stderr.decode(errors='replace')}")
        print(f"PASS {fixture}: {output.stat().st_size} output bytes")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--binary", type=Path)
    source.add_argument("--archive", type=Path)
    parser.add_argument("--video", choices=["true", "false"], required=True)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="showme-runtime-") as temporary:
        directory = Path(temporary).resolve()
        if args.archive:
            # This is the archive just produced by our own packaging job.
            shutil.unpack_archive(args.archive.resolve(), directory)
            binaries = [path for path in directory.rglob("*") if path.name in ("showme", "showme.exe") and path.is_file()]
            if len(binaries) != 1:
                raise ValueError("expected one release binary")
            binary = binaries[0]
        else:
            binary = args.binary.resolve()
        smoke(binary, args.video == "true", directory)


if __name__ == "__main__":
    main()
