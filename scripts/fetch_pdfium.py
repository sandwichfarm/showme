#!/usr/bin/env python3
"""Fetch the reviewed PDFium ABI for tests and release packaging."""

import argparse
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import tarfile
import urllib.request


def unpack_runtime(data, expected_hash, destination):
    if hashlib.sha256(data).hexdigest() != expected_hash:
        raise ValueError("PDFium archive checksum mismatch")
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        members = archive.getmembers()
        for member in members:
            path = PurePosixPath(member.name)
            if path.is_absolute() or ".." in path.parts or member.issym() or member.islnk():
                raise ValueError("unsafe PDFium archive member: " + member.name)
        libraries = [member for member in members if member.isfile() and
                     PurePosixPath(member.name).name in ("libpdfium.so", "libpdfium.dylib", "pdfium.dll")]
        if len(libraries) != 1:
            raise ValueError("expected exactly one PDFium runtime library")
        destination.mkdir(parents=True, exist_ok=True)
        library = libraries[0]
        (destination / PurePosixPath(library.name).name).write_bytes(archive.extractfile(library).read())
        for member in members:
            if member.isfile() and any("license" in part.lower() for part in PurePosixPath(member.name).parts):
                path = destination / "pdfium-licenses" / member.name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(archive.extractfile(member).read())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platform", required=True, choices=["linux-x64", "linux-arm64", "mac-x64", "mac-arm64", "win-x64"])
    parser.add_argument("--directory", type=Path, required=True)
    args = parser.parse_args()
    manifest = json.loads(Path(__file__).with_name("pdfium-runtime.json").read_text())
    asset = manifest["assets"][f"pdfium-{args.platform}.tgz"]
    request = urllib.request.Request(asset["url"], headers={"User-Agent": "showme-maintenance/1.0"})
    with urllib.request.urlopen(request, timeout=120) as response:
        data = response.read()
    unpack_runtime(data, asset["sha256"], args.directory)
    print(args.directory.resolve())


if __name__ == "__main__":
    main()
