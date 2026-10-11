"""Regression checks for the W15 accessibility and localization surfaces."""

from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class AccessibilityLocalizationExtractionTests(unittest.TestCase):
    def test_platform_localization_seam_and_resources_exist(self) -> None:
        header = ROOT / "src/platform/platform_localization.h"
        implementation = ROOT / "src/platform/macos/platform_localization.m"
        resource = ROOT / "resources/en.lproj/Localizable.strings"
        self.assertTrue(header.is_file(), "platform localization API is missing")
        self.assertTrue(implementation.is_file(), "macOS localization adapter is missing")
        self.assertTrue(resource.is_file(), "English string table is missing")
        text = header.read_text(encoding="utf-8")
        for symbol in (
            "platform_localization_copy",
            "platform_localization_copy_default",
            "platform_localization_format_number",
            "platform_localization_format_count_message",
            "platform_localization_format_one",
            "platform_localization_test_set_pseudo_locale",
        ):
            self.assertIn(symbol, text)

    def test_editor_labels_are_extracted_and_pseudo_locale_is_exercised(self) -> None:
        resource = ROOT / "resources/en.lproj/Localizable.strings"
        script = ROOT / "tests/editor/verify_accessibility_localization.py"
        self.assertTrue(resource.is_file(), "English string table is missing")
        self.assertTrue(script.is_file(), "pseudo-locale extraction test is missing")
        strings = resource.read_text(encoding="utf-8")
        required_keys = (
            "editor.label.layers",
            "editor.label.inspector",
            "editor.action.hide_layer",
            "editor.action.show_layer",
            "editor.action.lock_layer",
            "editor.action.unlock_layer",
            "editor.accessibility.layer_visibility",
            "editor.accessibility.layer_lock",
            "menu.view.interface_text_size",
            "dialog.export.scale",
            "welcome.title",
            "settings.accessibility.text_size_help",
        )
        for key in required_keys:
            self.assertRegex(strings, re.escape(key) + r'"\s*=')
        defaults = (ROOT / "src/platform/platform_localization_defaults.c").read_text(
            encoding="utf-8"
        )
        registered = set(re.findall(r'\{"([^"\n]+)"\s*,\s*"', defaults))
        extracted = set(re.findall(r'^\s*"([^"\n]+)"\s*=', strings, re.M))
        self.assertTrue(registered <= extracted, sorted(registered - extracted))
        fallback_text = set(re.findall(
            r'\{"[^"\n]+"\s*,\s*"([^"\n]*)"\s*,', defaults
        ))
        dynamic_prefixes = [value.split("%@", 1)[0] for value in fallback_text
                            if "%@" in value]
        for source_path in (ROOT / "src/app").glob("*.c"):
            source = source_path.read_text(encoding="utf-8")
            for message in re.findall(
                r'(?:editor_status_ok|viewer_editor_set_status)\(\s*'
                r'(?:editor|app->editor),\s*"([^"\n]*)"', source
            ):
                self.assertTrue(message in fallback_text or any(
                    message.startswith(prefix) for prefix in dynamic_prefixes
                ), f"unextracted status literal in {source_path.name}: {message}")
        self.assertTrue(script.is_file(), "headless text-size golden verifier is missing")
        test = (ROOT / "tests/editor/accessibility_localization_test.c").read_text(
            encoding="utf-8"
        )
        self.assertIn("platform_localization_test_set_pseudo_locale", test)
        self.assertIn("RTL ordering", test)
        self.assertIn("\\u202b", (ROOT / "src/platform/macos/platform_localization.m").read_text(
            encoding="utf-8"
        ))

    def test_voiceover_bridge_exposes_inline_layer_actions(self) -> None:
        bridge = (ROOT / "src/platform/macos/platform_accessibility.m").read_text(
            encoding="utf-8"
        )
        for marker in (
            "accessibilityHelp",
            "layer_action_element",
            "NSAccessibilityCheckBoxRole",
            "NSAccessibilityAnnouncementRequestedNotification",
            "UI_STATE_HIDDEN",
            "UI_STATE_LOCKED",
        ):
            self.assertIn(marker, bridge)

    def test_keyboard_navigation_tracks_focused_outline_rows(self) -> None:
        keyboard = ROOT / "src/app/viewer_editor_keyboard.c"
        self.assertTrue(keyboard.is_file(), "focus-aware editor key handling is missing")
        source = keyboard.read_text(encoding="utf-8")
        self.assertIn("editor_handle_focused_key", source)
        self.assertIn("editor_focus_outline_row", source)
        self.assertIn("focused_text_key", source)
        input_source = (ROOT / "src/app/viewer_editor_input.c").read_text(
            encoding="utf-8"
        )
        self.assertIn("ui_focus_next", input_source)
        structure = (ROOT / "src/app/viewer_chrome_structure.c").read_text(
            encoding="utf-8"
        )
        self.assertIn("UI_STATE_SELECTED", structure)
        self.assertIn("Selected on canvas", structure)
        bridge = (ROOT / "src/platform/macos/platform_accessibility.m").read_text(
            encoding="utf-8"
        )
        self.assertIn("UI_STATE_CHECKED |", bridge)
        self.assertIn("UI_STATE_SELECTED", bridge)

    def test_dialog_text_uses_registered_fallbacks(self) -> None:
        dialog = (ROOT / "src/platform/macos/platform_dialog.m").read_text(
            encoding="utf-8"
        )
        self.assertIn("dialog_message_string", dialog)
        self.assertIn("platform_localization_copy_default", dialog)
        resources = (ROOT / "resources/en.lproj/Localizable.strings").read_text(
            encoding="utf-8"
        )
        extracted = set(re.findall(r'^\s*"([^"\n]+)"\s*=', resources, re.M))
        for relative in (
            "src/platform/macos/platform_menus.m",
            "src/platform/macos/platform_dialog.m",
            "src/platform/macos/platform_accessibility.m",
        ):
            source = (ROOT / relative).read_text(encoding="utf-8")
            referenced = set(re.findall(
                r'(?:platform_localization_string|menu_string)\(\s*@"([^"\n]+)"',
                source,
            ))
            self.assertTrue(referenced <= extracted,
                f"unextracted native UI keys in {relative}: {sorted(referenced - extracted)}")
        packaging = (ROOT / "packaging/vexel_packaging.py").read_text(
            encoding="utf-8"
        )
        self.assertIn('resources" / "en.lproj"', packaging)

    def test_interface_text_size_reaches_two_hundred_percent_goldens(self) -> None:
        api = ROOT / "src/platform/platform_text_ui.h"
        menu = ROOT / "src/platform/macos/platform_menus.m"
        renderer = ROOT / "app/editor_chrome_render.c"
        verify = ROOT / "tests/editor/verify_accessibility_localization.py"
        self.assertTrue(api.is_file())
        self.assertTrue(menu.is_file())
        self.assertTrue(renderer.is_file())
        self.assertTrue(verify.is_file())
        self.assertIn("200U", api.read_text(encoding="utf-8"))
        self.assertIn("actionSetInterfaceTextSize", menu.read_text(encoding="utf-8"))
        self.assertIn("viewer_editor_set_interface_scale", renderer.read_text(encoding="utf-8"))
        verify_source = verify.read_text(encoding="utf-8")
        self.assertIn("INTERFACE_SCALE = 200", verify_source)
        self.assertIn('("dark", "light")', verify_source)


if __name__ == "__main__":
    unittest.main()
