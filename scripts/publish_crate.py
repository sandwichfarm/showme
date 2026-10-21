#!/usr/bin/env python3
"""Validate a tag and publish, accepting only an identical existing package."""

import argparse
import hashlib
import io
import json
from pathlib import Path
import subprocess
import tarfile
import urllib.error
import urllib.request


def validate_tag(tag, manifest):
    package = manifest["package"]
    if tag != "v" + package["version"]:
        raise ValueError(f"tag {tag!r} must match Cargo.toml version v{package['version']}")
    return package["name"], package["version"]


def contents(data):
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        # VCS provenance can differ for identical package source; all other
        # packaged files, including Cargo.lock, must match exactly.
        return {member.name.split("/", 1)[1]: archive.extractfile(member).read()
                for member in archive.getmembers() if member.isfile()
                and not member.name.endswith("/.cargo_vcs_info.json")}


def existing_package(name, version):
    headers = {"User-Agent": "showme-release/1.0"}
    request = urllib.request.Request(f"https://crates.io/api/v1/crates/{name}/{version}", headers=headers)
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            published = json.load(response)["version"]
    except urllib.error.HTTPError as error:
        # Only the registry API's 404 establishes absence. The archive CDN
        # returns 403 for missing objects, which must not authorize publication.
        if error.code == 404:
            return None
        raise
    if published["yanked"]:
        raise ValueError("already-published version is yanked; manual review required")
    request = urllib.request.Request(f"https://static.crates.io/crates/{name}/{name}-{version}.crate", headers=headers)
    with urllib.request.urlopen(request, timeout=60) as response:
        archive = response.read()
    if hashlib.sha256(archive).hexdigest() != published["checksum"]:
        raise ValueError("already-published crate checksum mismatch")
    return archive


def publish(name, version, target_directory):
    subprocess.run(["cargo", "package", "--locked"], check=True)
    candidate = Path(target_directory) / "package" / f"{name}-{version}.crate"
    existing = existing_package(name, version)
    if existing is not None:
        if contents(existing) != contents(candidate.read_bytes()):
            raise ValueError("already-published crate differs from this tagged package")
        print("Already-published crate matches this package; skipping upload")
        return
    subprocess.run(["cargo", "publish", "--locked"], check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--publish", action="store_true")
    args = parser.parse_args()
    metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"]))
    manifest_path = Path("Cargo.toml").resolve()
    package = next(package for package in metadata["packages"] if Path(package["manifest_path"]).resolve() == manifest_path)
    name, version = validate_tag(args.tag, {"package": package})
    if not args.publish:
        print(f"Tag matches {name} {version}")
        return
    publish(name, version, metadata["target_directory"])


if __name__ == "__main__":
    main()
