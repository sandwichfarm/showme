import unittest

from check_audit import check


class AuditGateTests(unittest.TestCase):
    def report(self, identifier="RUSTSEC-2024-0436"):
        return {"vulnerabilities": {"count": 0}, "warnings": {"unmaintained": [
            {"advisory": {"id": identifier}, "package": {"name": "paste"}},
        ]}}

    def test_new_warnings_cannot_pass_without_a_reviewed_disposition(self):
        self.assertTrue(check(self.report("RUSTSEC-9999-0001"), {}))

    def test_a_reviewed_warning_needs_an_owner_and_stays_visible(self):
        reviewed = {"RUSTSEC-2024-0436": {"package": "paste", "owner": "maintainers", "disposition": "Review upstream migration"}}
        self.assertEqual(check(self.report(), reviewed), [])
        reviewed["RUSTSEC-2024-0436"]["owner"] = ""
        self.assertTrue(check(self.report(), reviewed))

    def test_dispositions_cannot_waive_vulnerabilities(self):
        self.assertTrue(check({"vulnerabilities": {"count": 1}}, {}))


if __name__ == "__main__":
    unittest.main()
