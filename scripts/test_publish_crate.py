import hashlib
import io
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import urllib.error

from publish_crate import contents, existing_package, publish, validate_tag


def package(source=b"source", lock=b"lock"):
    data = io.BytesIO()
    with tarfile.open(fileobj=data, mode="w:gz") as archive:
        for name, value in [("src/lib.rs", source), ("Cargo.lock", lock)]:
            info = tarfile.TarInfo("showme-0.1.1/" + name)
            info.size = len(value)
            archive.addfile(info, io.BytesIO(value))
    return data.getvalue()


class PublishGateTests(unittest.TestCase):
    def test_tag_must_equal_the_package_version(self):
        manifest = {"package": {"name": "showme", "version": "0.1.1"}}
        self.assertEqual(validate_tag("v0.1.1", manifest), ("showme", "0.1.1"))
        for tag in ["v0.1.0", "0.1.1", "v0.1.1-extra"]:
            with self.assertRaises(ValueError):
                validate_tag(tag, manifest)

    def test_package_comparison_detects_source_and_lockfile_changes(self):
        candidate = contents(package(b"source", b"lock"))
        self.assertEqual(candidate, contents(package(b"source", b"lock")))
        self.assertNotEqual(candidate, contents(package(b"changed", b"lock")))
        self.assertNotEqual(candidate, contents(package(b"source", b"changed")))

    def test_only_registry_404_means_unpublished(self):
        for status in [404, 403, 429, 500]:
            error = urllib.error.HTTPError("https://crates.io/api/v1/crates/showme/0.1.1", status, "failure", None, None)
            with patch("publish_crate.urllib.request.urlopen", side_effect=error):
                if status == 404:
                    self.assertIsNone(existing_package("showme", "0.1.1"))
                else:
                    with self.assertRaises(urllib.error.HTTPError):
                        existing_package("showme", "0.1.1")

    def test_existing_archive_requires_checksum_and_non_yanked_version(self):
        archive = package()
        checksum = hashlib.sha256(archive).hexdigest()
        for yanked, digest in [(False, checksum), (False, "0" * 64), (True, checksum)]:
            metadata = io.BytesIO(json.dumps({"version": {"yanked": yanked, "checksum": digest}}).encode())
            with patch("publish_crate.urllib.request.urlopen", side_effect=[metadata, io.BytesIO(archive)]):
                if not yanked and digest == checksum:
                    self.assertEqual(existing_package("showme", "0.1.1"), archive)
                else:
                    with self.assertRaises(ValueError):
                        existing_package("showme", "0.1.1")

    def test_matching_published_package_skips_upload_and_differences_fail(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary) / "package"
            directory.mkdir()
            (directory / "showme-0.1.1.crate").write_bytes(package())
            for existing in [package(), package(source=b"different")]:
                with patch("publish_crate.existing_package", return_value=existing), patch("publish_crate.subprocess.run") as run:
                    if contents(existing) == contents(package()):
                        publish("showme", "0.1.1", temporary)
                    else:
                        with self.assertRaisesRegex(ValueError, "differs"):
                            publish("showme", "0.1.1", temporary)
                    run.assert_called_once_with(["cargo", "package", "--locked"], check=True)

    def test_unexpected_package_or_publish_failure_propagates(self):
        for failing_call in [1, 2]:
            failure = subprocess.CalledProcessError(1, "cargo")
            outcomes = [failure] if failing_call == 1 else [None, failure]
            with patch("publish_crate.existing_package", return_value=None), patch("publish_crate.subprocess.run", side_effect=outcomes) as run:
                with self.assertRaises(subprocess.CalledProcessError):
                    publish("showme", "0.1.1", "target")
                self.assertEqual(run.call_count, failing_call)


if __name__ == "__main__":
    unittest.main()
