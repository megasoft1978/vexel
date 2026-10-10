"""Unit tests for the seeded headless SVG render harness."""

from __future__ import annotations

import unittest

from scripts import e2e_svg


class SvgGeneratorTests(unittest.TestCase):
    def test_generated_matrix_covers_locked_svg_features(self) -> None:
        document = "".join(e2e_svg.generate_svg(20261010, index).decode("ascii")
                           for index in range(6))
        for token in ("<rect", "<circle", "<ellipse", "<line", "<polygon", "<path",
                      "transform=", "linearGradient", "radialGradient", "<use", "clipPath"):
            with self.subTest(token=token):
                self.assertIn(token, document)

    def test_seeded_generation_is_repeatable_and_seed_sensitive(self) -> None:
        self.assertEqual(e2e_svg.generate_svg(42, 7), e2e_svg.generate_svg(42, 7))
        self.assertNotEqual(e2e_svg.generate_svg(42, 7), e2e_svg.generate_svg(43, 7))
        self.assertNotEqual(e2e_svg.generate_svg(42, 7), e2e_svg.generate_svg(42, 8))

    def test_mutation_set_has_each_required_corruption_family(self) -> None:
        mutations = dict(e2e_svg.mutation_inputs(123, e2e_svg.generate_svg(123, 0)))
        self.assertEqual(set(mutations), {"byte-flip", "truncate-half", "truncate-tail", "huge-number", "invalid-utf8"})
        self.assertIn(b"9" * 200, mutations["huge-number"])
        self.assertIn(b"\xff\xfe", mutations["invalid-utf8"])
        self.assertLess(len(e2e_svg.deep_nesting_svg()), 20_000)

    def test_ppm_validation_checks_dimensions_and_exact_pixel_payload(self) -> None:
        header = b"P6\n2 1\n255\n"
        self.assertTrue(e2e_svg.validate_ppm(header + bytes(6), 2, 1))
        self.assertFalse(e2e_svg.validate_ppm(header + bytes(5), 2, 1))
        self.assertFalse(e2e_svg.validate_ppm(header + bytes(6), 1, 2))
        self.assertFalse(e2e_svg.validate_ppm(b"P3\n2 1\n255\n" + bytes(6), 2, 1))

    def test_argument_validation_rejects_out_of_range_values(self) -> None:
        for args in (("--seed", "-1"), ("--documents", "0"),
                     ("--documents", "2001"), ("--timeout-seconds", "61"),
                     ("--output", "/tmp/e2e-report.json")):
            with self.subTest(args=args), self.assertRaises(SystemExit):
                e2e_svg.parse_args(list(args))

    def test_report_schema_rejects_boolean_count_and_missing_failure_path(self) -> None:
        report = {
            "schema_version": 1, "kind": "svg_render_e2e", "seed": 7, "status": "pass",
            "generated_documents": 1, "mutated_inputs": 7, "render_invocations": 9,
            "valid_ppm_count": 2, "deterministic_checks": 1, "clean_stderr_checks": 2,
            "crashes": 0, "timeouts": 0, "timeout_seconds": 5.0,
            "maximum_case_seconds": 0.02, "elapsed_seconds": 0.2, "input_root": ".cache/e2e",
            "failures": [],
        }
        e2e_svg.validate_report(report)
        report["generated_documents"] = True
        with self.assertRaisesRegex(e2e_svg.E2EError, "nonnegative integer"):
            e2e_svg.validate_report(report)
        report["generated_documents"] = 1
        report["status"] = "failure"
        report["failures"] = [{"seed": 7, "reason": "missing path"}]
        with self.assertRaisesRegex(e2e_svg.E2EError, "failure item"):
            e2e_svg.validate_report(report)


if __name__ == "__main__":
    unittest.main()
