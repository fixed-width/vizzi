"""Checks for the qualification evidence collector, independent of performance."""

import unittest

from measure import binding, check_window, compact, summarise


class EvidenceTests(unittest.TestCase):
    def test_literal_file_is_selected_when_path_filter_has_other_matches(self):
        class Host:
            def request(self, route, params):
                if route == "/api/window":
                    return 200, {"outline": [{"name": "target", "kind": "function", "id": "tracking"}], "outline_total": 1}, 0, 0
                exact = params.get("file") == "target-file"
                return 200, {"snapshot": "snapshot", "files": [{"path": "mm/mmap.c", "id": "target-file"}],
                             "source": {"path": "mm/mmap.c" if exact else "arch/arc/mm/mmap.c",
                                        "version": "target-version" if exact else "wrong-version"}}, 0, 0

        request, _, _ = binding(Host(), "mm/mmap.c", "target")
        self.assertEqual(request, {"snapshot": "snapshot", "file": "target-file",
                                  "version": "target-version", "tracking": "tracking"})

    def test_diff_line_ranges_survive_response_compaction(self):
        edit = {"lines": {"start": 5, "end": 9}, "bytes": {"start": 20, "end": 80}}
        self.assertEqual(compact(edit), edit)

    def test_refusals_are_separate_from_successful_latency(self):
        rows = [
            {"operation": "history", "condition": "first", "status": 200, "ms": 12},
            {"operation": "history", "condition": "first", "status": 503, "ms": 900},
            {"operation": "history", "condition": "first", "status": 200, "ms": 8},
            {"operation": "history", "condition": "warm", "status": 409, "ms": 3},
        ]
        result = summarise(rows)
        self.assertEqual(result["history/first"]["successes"], 2)
        self.assertEqual(result["history/first"]["median_ms"], 10)
        self.assertEqual(result["history/first"]["max_ms"], 12)
        self.assertEqual(result["history/first"]["refusals"], {"503": 1})
        self.assertIsNone(result["history/warm"]["median_ms"])
        self.assertEqual(result["history/warm"]["refusals"], {"409": 1})

    def test_source_oracle_checks_literal_git_bytes_and_exact_pins(self):
        pins = {"snapshot": "s", "file": "f", "version": "v"}
        page = {**pins, "lines": [
            {"number": 2, "column": 2, "text": "λ()", "ending": "\r\n", "right": False},
        ]}
        check_window(page, pins, b"before\n  \xce\xbb()\r\nafter\n")
        for field in pins:
            with self.subTest(field=field), self.assertRaises(AssertionError):
                check_window({**page, field: "foreign"}, pins, b"before\n  \xce\xbb()\r\nafter\n")
        with self.assertRaises(AssertionError):
            check_window(page, pins, b"before\n  changed()\r\nafter\n")

    def test_partial_lines_do_not_claim_an_ending(self):
        pins = {"snapshot": "s", "file": "f", "version": "v"}
        page = {**pins, "lines": [
            {"number": 1, "column": 1, "text": "b", "ending": "", "right": True},
        ]}
        check_window(page, pins, b"abc\n")
        page["lines"][0]["ending"] = "\n"
        with self.assertRaises(AssertionError):
            check_window(page, pins, b"abc\n")

    def test_columns_count_utf8_bytes(self):
        pins = {"snapshot": "s", "file": "f", "version": "v"}
        page = {**pins, "lines": [
            {"number": 1, "column": 3, "text": "call()", "ending": "\n", "right": False},
        ]}
        check_window(page, pins, "λ call()\n".encode())


if __name__ == "__main__":
    unittest.main()
