"""Unit tests for macOS app metadata, icon generation, and packaging arguments."""

from __future__ import annotations

import plistlib
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from packaging import vexel_packaging


class PackagingMetadataTests(unittest.TestCase):
    def test_project_version_is_read_from_the_single_cmake_source(self) -> None:
        self.assertEqual(vexel_packaging.project_version(), "0.0.1")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "CMakeLists.txt").write_text("project(vexel VERSION 3.4.5 LANGUAGES C OBJC)\n")
            self.assertEqual(vexel_packaging.project_version(root), "3.4.5")

    def test_info_plist_has_document_registration_and_macos_contract(self) -> None:
        info = vexel_packaging.info_plist("2.3.4")
        self.assertEqual(info["CFBundleIdentifier"], "com.vexel.svgworkbench")
        self.assertEqual(info["CFBundleShortVersionString"], info["CFBundleVersion"])
        self.assertEqual(info["LSMinimumSystemVersion"], "13.0")
        self.assertTrue(info["NSHighResolutionCapable"])
        self.assertEqual(info["LSApplicationCategoryType"], "public.app-category.graphics-design")
        document_type = info["CFBundleDocumentTypes"][0]
        self.assertIn("public.svg-image", document_type["LSItemContentTypes"])
        self.assertIn("svg", document_type["CFBundleTypeExtensions"])
        self.assertEqual(plistlib.loads(plistlib.dumps(info))["CFBundleExecutable"], "svgworkbench")

    def test_invalid_versions_are_rejected(self) -> None:
        for version in ("1.2", "1.2.3-beta", "v1.2.3", "1.2.3.4"):
            with self.subTest(version=version), self.assertRaises(vexel_packaging.PackagingError):
                vexel_packaging.info_plist(version)

    def test_generated_iconset_has_standard_sizes_and_valid_png_headers(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            iconset = Path(temporary) / "Vexel.iconset"
            vexel_packaging.generate_iconset(iconset)
            self.assertEqual(len(list(iconset.glob("*.png"))), 10)
            for points in vexel_packaging.ICON_POINTS:
                for scale, suffix in ((1, ".png"), (2, "@2x.png")):
                    path = iconset / f"icon_{points}x{points}{suffix}"
                    data = path.read_bytes()
                    expected_size = points * scale
                    self.assertEqual(data[:8], b"\x89PNG\r\n\x1a\n")
                    self.assertEqual(data[12:16], b"IHDR")
                    self.assertEqual(int.from_bytes(data[16:20], "big"), expected_size)
                    self.assertEqual(int.from_bytes(data[20:24], "big"), expected_size)

    def test_programmatic_icns_container_has_png_backed_representations(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "Vexel.icns"
            vexel_packaging.generate_icns(path)
            data = path.read_bytes()
            self.assertEqual(data[:4], b"icns")
            self.assertEqual(int.from_bytes(data[4:8], "big"), len(data))
            cursor = 8
            found = []
            while cursor < len(data):
                chunk_type = data[cursor:cursor + 4]
                chunk_length = int.from_bytes(data[cursor + 4:cursor + 8], "big")
                self.assertGreaterEqual(chunk_length, 8)
                chunk = data[cursor + 8:cursor + chunk_length]
                self.assertEqual(chunk[:8], b"\x89PNG\r\n\x1a\n")
                found.append(chunk_type)
                cursor += chunk_length
            self.assertEqual(cursor, len(data))
            self.assertEqual(found, [kind for _, kind in vexel_packaging.ICNS_REPRESENTATIONS])

    def test_signing_identity_validation_and_command_arguments(self) -> None:
        self.assertEqual(vexel_packaging._sign_identity({}), "-")
        self.assertEqual(vexel_packaging._sign_identity({"VEXEL_SIGN_IDENTITY": "Developer ID"}), "Developer ID")
        with self.assertRaisesRegex(vexel_packaging.PackagingError, "VEXEL_SIGN_IDENTITY"):
            vexel_packaging._sign_identity({"VEXEL_SIGN_IDENTITY": "  "})
        self.assertEqual(vexel_packaging.parse_args(["bundle"]).command, "bundle")
        self.assertEqual(vexel_packaging.parse_args(["dmg"]).command, "dmg")
        with self.assertRaises(SystemExit):
            vexel_packaging.parse_args([])
        with self.assertRaises(SystemExit):
            vexel_packaging.parse_args(["bundle", "--unexpected"])

    def test_dmg_staging_contains_the_applications_link_and_compressed_image(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            build = root / "build"
            build.mkdir()
            app = build / "Vexel.app"
            (app / "Contents").mkdir(parents=True)
            (app / "Contents/Info.plist").write_bytes(b"mock plist")
            seen_stage: list[Path] = []
            calls: list[list[str]] = []

            def fake_run(argv, *, capture=False):
                command = list(argv)
                calls.append(command)
                if command[:2] == ["hdiutil", "create"]:
                    stage = Path(command[command.index("-srcfolder") + 1])
                    seen_stage.append(stage)
                    self.assertTrue((stage / "Applications").is_symlink())
                    self.assertEqual((stage / "Applications").resolve(), Path("/Applications"))
                    self.assertTrue((stage / "Vexel.app/Contents/Info.plist").is_file())
                    Path(command[-1]).write_bytes(b"mock compressed dmg")
                return subprocess.CompletedProcess(command, 0, "", "")

            with patch.object(vexel_packaging, "build_bundle", return_value=app), \
                    patch.object(vexel_packaging, "_run", side_effect=fake_run):
                result = vexel_packaging.build_dmg(root, {})

            self.assertEqual(result, build / "Vexel.dmg")
            self.assertTrue(result.is_file())
            self.assertEqual(len(seen_stage), 1)
            create = next(command for command in calls if command[:2] == ["hdiutil", "create"])
            self.assertEqual(create[create.index("-format") + 1], "UDZO")
            self.assertIn(["hdiutil", "verify", str(result)], calls)

    def test_notarization_requires_a_non_adhoc_identity(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "build").mkdir()
            with patch.object(vexel_packaging, "build_bundle") as build_bundle:
                with self.assertRaisesRegex(vexel_packaging.PackagingError, "requires a Developer ID"):
                    vexel_packaging.build_dmg(root, {"VEXEL_SIGN_IDENTITY": "-",
                                                      "VEXEL_NOTARY_PROFILE": "vexel"})
                build_bundle.assert_not_called()

    def test_notary_profile_submits_staples_and_validates_the_disk_image(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            build = root / "build"
            build.mkdir()
            app = build / "Vexel.app"
            app.mkdir()
            calls: list[list[str]] = []

            def fake_run(argv, *, capture=False):
                command = list(argv)
                calls.append(command)
                if command[:2] == ["hdiutil", "create"]:
                    Path(command[-1]).write_bytes(b"mock disk image")
                return subprocess.CompletedProcess(command, 0, "", "")

            env = {"VEXEL_SIGN_IDENTITY": "Developer ID Application: Example",
                   "VEXEL_NOTARY_PROFILE": "vexel-notary"}
            with patch.object(vexel_packaging, "build_bundle", return_value=app), \
                    patch.object(vexel_packaging, "_run", side_effect=fake_run):
                vexel_packaging.build_dmg(root, env)

            self.assertIn(["xcrun", "notarytool", "submit", str(build / "Vexel.dmg"),
                           "--keychain-profile", "vexel-notary", "--wait"], calls)
            self.assertIn(["xcrun", "stapler", "staple", str(build / "Vexel.dmg")], calls)
            self.assertIn(["xcrun", "stapler", "validate", str(build / "Vexel.dmg")], calls)

    def test_failed_dmg_creation_removes_a_partial_output(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            build = root / "build"
            build.mkdir()
            app = build / "Vexel.app"
            app.mkdir()

            def fail_create(argv, *, capture=False):
                command = list(argv)
                if command[:2] == ["hdiutil", "create"]:
                    Path(command[-1]).write_bytes(b"partial")
                    raise vexel_packaging.PackagingError("simulated hdiutil failure")
                return subprocess.CompletedProcess(command, 0, "", "")

            with patch.object(vexel_packaging, "build_bundle", return_value=app), \
                    patch.object(vexel_packaging, "_run", side_effect=fail_create):
                with self.assertRaisesRegex(vexel_packaging.PackagingError, "simulated"):
                    vexel_packaging.build_dmg(root, {})
            self.assertFalse((build / "Vexel.dmg").exists())


if __name__ == "__main__":
    unittest.main()
