#!/usr/bin/env python3
"""Package a binary with its matching PDFium runtime and license files."""

import argparse
import os
from pathlib import Path
import shutil
import tarfile
import zipfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--asset", required=True)
    parser.add_argument("--directory", type=Path, default=Path("dist"))
    args = parser.parse_args()
    root = args.directory / args.asset
    root.mkdir(parents=True, exist_ok=False)
    shutil.copy2(args.binary, root / args.binary.name)
    shutil.copy2(Path(__file__).parent.parent / "docs/runtime.md", root / "README.md")
    native = Path(os.environ["SHOWME_PDFIUM_DIR"])
    libraries = [path for path in native.iterdir() if path.name in ("libpdfium.so", "libpdfium.dylib", "pdfium.dll")]
    if len(libraries) != 1:
        raise ValueError("expected one matching PDFium runtime")
    shutil.copy2(libraries[0], root / libraries[0].name)
    shutil.copytree(native / "pdfium-licenses", root / "pdfium-licenses")
    if args.binary.suffix == ".exe":
        archive = args.directory / (args.asset + ".zip")
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as output:
            for path in sorted(root.rglob("*")):
                if path.is_file():
                    output.write(path, path.relative_to(args.directory))
    else:
        archive = args.directory / (args.asset + ".tar.gz")
        with tarfile.open(archive, "w:gz") as output:
            output.add(root, arcname=args.asset)
    print(archive)


if __name__ == "__main__":
    main()
