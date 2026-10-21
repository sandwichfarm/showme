import unittest

from check_coverage import check


class CoverageGateTests(unittest.TestCase):
    def test_healthy_overall_coverage_cannot_hide_missing_media(self):
        report = {"data": [{"totals": {"lines": {"percent": 95}}, "files": []}]}
        failures = check(report)
        self.assertTrue(any("src/renderer.rs: missing" in failure for failure in failures))
        self.assertTrue(any("src/image.rs: missing" in failure for failure in failures))
        self.assertTrue(any("src/video.rs: missing" in failure for failure in failures))

    def test_healthy_overall_coverage_cannot_hide_a_renderer_regression(self):
        report = {"data": [{"totals": {"lines": {"percent": 95}}, "files": [
            {"filename": "/checkout/src/renderer.rs", "summary": {"lines": {"percent": 1}}},
            {"filename": "/checkout/src/image.rs", "summary": {"lines": {"percent": 90}}},
            {"filename": "/checkout/src/video.rs", "summary": {"lines": {"percent": 90}}},
        ]}]}
        self.assertEqual(check(report), ["src/renderer.rs: 1.00% < 80.00%"])


if __name__ == "__main__":
    unittest.main()
