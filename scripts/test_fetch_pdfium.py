import hashlib
import io
from pathlib import Path
import tarfile
import tempfile
import unittest

from fetch_pdfium import unpack_runtime


class FetchPdfiumTests(unittest.TestCase):
    def archive(self, names):
        data = io.BytesIO()
        with tarfile.open(fileobj=data, mode="w:gz") as archive:
            for name in names:
                member = tarfile.TarInfo(name)
                member.size = 7
                archive.addfile(member, io.BytesIO(b"fixture"))
        return data.getvalue()

    def test_checksum_failure_creates_no_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary) / "runtime"
            with self.assertRaisesRegex(ValueError, "checksum"):
                unpack_runtime(b"corrupted", "0" * 64, destination)
            self.assertFalse(destination.exists())

    def test_traversal_is_rejected_before_any_extraction(self):
        data = self.archive(["lib/libpdfium.so", "../escape"])
        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary) / "runtime"
            with self.assertRaisesRegex(ValueError, "unsafe"):
                unpack_runtime(data, hashlib.sha256(data).hexdigest(), destination)
            self.assertFalse(destination.exists())

    def test_runtime_and_licenses_are_packaged_together(self):
        data = self.archive(["lib/libpdfium.so", "LICENSE", "licenses/third-party.txt"])
        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary) / "runtime"
            unpack_runtime(data, hashlib.sha256(data).hexdigest(), destination)
            self.assertEqual((destination / "libpdfium.so").read_bytes(), b"fixture")
            self.assertEqual((destination / "pdfium-licenses/LICENSE").read_bytes(), b"fixture")
            self.assertEqual((destination / "pdfium-licenses/licenses/third-party.txt").read_bytes(), b"fixture")


if __name__ == "__main__":
    unittest.main()
