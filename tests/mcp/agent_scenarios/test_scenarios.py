from __future__ import annotations

import base64
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import time
import unittest


ROOT = Path(__file__).resolve().parents[3]
BIN = Path(os.environ.get("VEXEL_MCP_BIN_DIR", ROOT / "build/mac-debug/bin"))
PROTOCOL_VERSION = "2026-07-28"
SCENARIO_DIR = Path(__file__).resolve().parent
GOLDEN_DIR = SCENARIO_DIR / "goldens"
EMPTY_SVG = (
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
    "viewBox='0 0 64 64'></svg>"
)
REFERENCE_SOURCES = {
    "flat-style-icon": (
        "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' viewBox='0 0 64 64'>"
        "<rect id='background' x='0' y='0' width='64' height='64' fill='#f7f7f2'/>"
        "<circle id='disc' cx='32' cy='32' r='20' fill='#345995'/>"
        "<path id='glyph' d='M22 32 L28 38 L42 24' fill='none' stroke='#ffffff' "
        "stroke-width='4' stroke-linecap='round' stroke-linejoin='round'/></svg>"
    ),
    "gradient-sunset": (
        "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' viewBox='0 0 64 64'>"
        "<defs><linearGradient id='sunset' x1='0' y1='0' x2='0' y2='64'>"
        "<stop offset='0' stop-color='#f06b51'/><stop offset='1' stop-color='#1d3557'/>"
        "</linearGradient></defs><rect id='sky' x='0' y='0' width='64' height='64' "
        "fill='url(#sunset)'/><circle id='sun' cx='32' cy='36' r='10' fill='#ffd166'/>"
        "<rect id='horizon' x='0' y='50' width='64' height='14' fill='#e76f51'/></svg>"
    ),
    "bouncing-ball-smil": (
        "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' viewBox='0 0 64 64'>"
        "<rect id='floor' x='0' y='54' width='64' height='10' fill='#344e41'/>"
        "<circle id='ball' cx='32' cy='12' r='6' fill='#2ec4b6'>"
        "<animate id='bounce' attributeName='cy' values='12;48;12' keyTimes='0;0.5;1' "
        "dur='2s' repeatCount='indefinite'/></circle></svg>"
    ),
    "spinner-smil": (
        "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' viewBox='0 0 64 64'>"
        "<circle id='track' cx='32' cy='32' r='19' fill='none' stroke='#d9d9d9' stroke-width='5'/>"
        "<rect id='spinner-hand' x='29' y='8' width='6' height='20' rx='3' fill='#335cbb'>"
        "<animateTransform id='spin' attributeName='transform' type='rotate' "
        "from='0 32 32' to='360 32 32' dur='1s' repeatCount='indefinite'/>"
        "</rect></svg>"
    ),
    "morph-free-logo-reveal": (
        "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' viewBox='0 0 64 64'>"
        "<rect id='logo-base' x='8' y='8' width='48' height='48' rx='12' fill='#172554'/>"
        "<circle id='logo-mark' cx='32' cy='32' r='14' fill='#fbbf24' opacity='0'>"
        "<animate id='reveal' attributeName='opacity' from='0' to='1' dur='1s' fill='freeze'/>"
        "</circle></svg>"
    ),
}


class McpAgentScenarioTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="vexel-mcp-agent-")
        self.root = Path(self.temporary.name)
        self.granted = self.root / "granted"
        self.granted.mkdir(mode=0o700)
        self.socket_path = self.root / "m.sock"
        self.current_path = self.granted / "current.svg"
        self.current_path.write_text(EMPTY_SVG, encoding="utf-8")
        environment = os.environ.copy()
        environment["VEXEL_MCP_SOCKET"] = str(self.socket_path)
        environment["VEXEL_MCP_ALLOWED_DIRS"] = str(self.granted)
        self.broker = subprocess.Popen(
            [str(BIN / "mcp_headless_broker"), str(self.current_path)],
            cwd=ROOT, env=environment, stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        deadline = time.monotonic() + 5.0
        while time.monotonic() < deadline and not self.socket_path.exists():
            if self.broker.poll() is not None:
                error = self.broker.stderr.read() if self.broker.stderr else b""
                if b"bind failed: Operation not permitted" in error:
                    self.temporary.cleanup()
                    self.skipTest(
                        "execution sandbox denies Unix-domain socket bind (EPERM)")
                self.fail(f"headless broker exited before socket startup: {error!r}")
            time.sleep(0.01)
        if not self.socket_path.exists():
            self.fail("headless broker did not create its Unix socket")
        self.server = subprocess.Popen(
            [str(BIN / "vexel-mcp")], cwd=ROOT, env=environment,
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            bufsize=0)
        self.request_id = 0
        discovered = self.rpc("server/discover", {"_meta": self.meta()})
        self.assertEqual(discovered["result"]["supportedVersions"], [PROTOCOL_VERSION])
        tool_rows = self.rpc("tools/list", {})["result"]["tools"]
        self.available_tools = {item["name"] for item in tool_rows}

    def tearDown(self) -> None:
        for process_name in ("server", "broker"):
            process = getattr(self, process_name, None)
            if process is None:
                continue
            if process.poll() is None:
                process.send_signal(signal.SIGTERM)
                try:
                    process.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=3)
            for stream_name in ("stdin", "stdout", "stderr"):
                stream = getattr(process, stream_name, None)
                if stream is not None:
                    stream.close()
        self.temporary.cleanup()

    @staticmethod
    def meta() -> dict[str, object]:
        return {"io.modelcontextprotocol/protocolVersion": PROTOCOL_VERSION,
                "io.modelcontextprotocol/clientCapabilities": {}}

    def rpc(self, method: str, params: dict[str, object]) -> dict[str, object]:
        self.request_id += 1
        request = {"jsonrpc": "2.0", "id": self.request_id,
                   "method": method, "params": dict(params)}
        if method != "server/discover":
            request["params"]["_meta"] = self.meta()
        assert self.server.stdin is not None and self.server.stdout is not None
        self.server.stdin.write(
            json.dumps(request, separators=(",", ":")).encode() + b"\n")
        self.server.stdin.flush()
        line = self.server.stdout.readline()
        if not line:
            stderr = self.server.stderr.read() if self.server.stderr else b""
            self.fail(f"vexel-mcp exited without a response: {stderr!r}")
        response = json.loads(line)
        self.assertEqual(response.get("id"), self.request_id)
        return response

    def call_tool(self, name: str,
                  arguments: dict[str, object]) -> dict[str, object]:
        response = self.rpc("tools/call", {"name": name, "arguments": arguments})
        self.assertIn("result", response)
        result = response["result"]
        self.assertFalse(result.get("isError"), result)
        self.assertEqual(result["structuredContent"].get("mode"), "headless", result)
        return result["structuredContent"]

    def add_shape(self, document_id: str, root_id: str, template: str,
                  attributes: dict[str, str]) -> None:
        self.call_tool("element_add", {
            "documentId": document_id, "parentId": root_id,
            "template": template, "attributes": attributes,
        })

    def build_icon(self, document_id: str, root_id: str) -> None:
        self.add_shape(document_id, root_id, "rect", {
            "id": "background", "x": "0", "y": "0", "width": "64",
            "height": "64", "fill": "#f7f7f2",
        })
        self.add_shape(document_id, root_id, "circle", {
            "id": "disc", "cx": "32", "cy": "32", "r": "20", "fill": "blue",
        })
        self.call_tool("attribute_set", {
            "documentId": document_id, "elementId": "disc",
            "name": "fill", "value": "#345995",
        })
        self.add_shape(document_id, root_id, "path", {
            "id": "glyph", "d": "M22 32 L28 38 L42 24", "fill": "none",
            "stroke": "white", "stroke-width": "4", "stroke-linecap": "round",
            "stroke-linejoin": "round",
        })
        self.call_tool("attribute_set", {
            "documentId": document_id, "elementId": "glyph",
            "name": "stroke", "value": "#ffffff",
        })

    def build_sunset(self, document_id: str, root_id: str) -> None:
        self.call_tool("element_add_fragment", {
            "documentId": document_id, "parentId": root_id,
            "xml": "<defs><linearGradient id='sunset' x1='0' y1='0' x2='0' y2='64'>"
                   "<stop offset='0' stop-color='#f06b51'/>"
                   "<stop offset='1' stop-color='#1d3557'/>"
                   "</linearGradient></defs>",
        })
        self.add_shape(document_id, root_id, "rect", {
            "id": "sky", "x": "0", "y": "0", "width": "64", "height": "64",
            "fill": "red",
        })
        self.call_tool("attribute_set", {
            "documentId": document_id, "elementId": "sky",
            "name": "fill", "value": "url(#sunset)",
        })
        self.add_shape(document_id, root_id, "circle", {
            "id": "sun", "cx": "32", "cy": "36", "r": "10", "fill": "#ffd166",
        })
        self.add_shape(document_id, root_id, "rect", {
            "id": "horizon", "x": "0", "y": "50", "width": "64",
            "height": "14", "fill": "#e76f51",
        })

    def build_bouncing_ball(self, document_id: str, root_id: str) -> None:
        self.add_shape(document_id, root_id, "rect", {
            "id": "floor", "x": "0", "y": "54", "width": "64",
            "height": "10", "fill": "#344e41",
        })
        self.add_shape(document_id, root_id, "circle", {
            "id": "ball", "cx": "32", "cy": "12", "r": "6", "fill": "#2ec4b6",
        })
        self.call_tool("animation_add", {
            "documentId": document_id, "targetId": "ball",
            "xml": "<animate id='bounce' attributeName='cy' values='12;48;12' "
                   "keyTimes='0;0.5;1' dur='2s' repeatCount='indefinite'/>",
        })

    def build_spinner(self, document_id: str, root_id: str) -> None:
        self.add_shape(document_id, root_id, "circle", {
            "id": "track", "cx": "32", "cy": "32", "r": "19", "fill": "none",
            "stroke": "#d9d9d9", "stroke-width": "5",
        })
        self.add_shape(document_id, root_id, "rect", {
            "id": "spinner-hand", "x": "29", "y": "8", "width": "6",
            "height": "20", "rx": "3", "fill": "#335cbb",
        })
        self.call_tool("animation_add", {
            "documentId": document_id, "targetId": "spinner-hand",
            "xml": "<animateTransform id='spin' attributeName='transform' "
                   "type='rotate' from='0 32 32' to='360 32 32' dur='1s' "
                   "repeatCount='indefinite'/>",
        })

    def build_logo_reveal(self, document_id: str, root_id: str) -> None:
        self.add_shape(document_id, root_id, "rect", {
            "id": "logo-base", "x": "8", "y": "8", "width": "48", "height": "48",
            "rx": "12", "fill": "#172554",
        })
        self.add_shape(document_id, root_id, "circle", {
            "id": "logo-mark", "cx": "32", "cy": "32", "r": "14",
            "fill": "#fbbf24", "opacity": "0",
        })
        self.call_tool("animation_add", {
            "documentId": document_id, "targetId": "logo-mark",
            "xml": "<animate id='reveal' attributeName='opacity' from='0' to='1' "
                   "dur='1s' fill='freeze'/>",
        })

    def test_stepwise_drawings_animations_and_pixels(self) -> None:
        manifest = json.loads((SCENARIO_DIR / "manifest.json").read_text())
        builders = {
            "flat-style-icon": self.build_icon,
            "gradient-sunset": self.build_sunset,
            "bouncing-ball-smil": self.build_bouncing_ball,
            "spinner-smil": self.build_spinner,
            "morph-free-logo-reveal": self.build_logo_reveal,
        }
        missing_by_scenario: dict[str, list[str]] = {}
        ran = 0
        for scenario in manifest["scenarios"]:
            name = scenario["name"]
            missing = sorted(set(scenario["requiresTools"]) - self.available_tools)
            if missing:
                missing_by_scenario[name] = missing
                continue
            path = self.granted / f"{name}.svg"
            path.write_text(EMPTY_SVG, encoding="utf-8")
            opened = self.call_tool("document_open", {"path": str(path)})
            self.assertEqual(opened.get("mode"), "headless", opened)
            document_id = opened["documentId"]
            root_page = self.call_tool("tree_get", {
                "documentId": document_id, "cursor": 0, "limit": 1,
            })
            root_id = root_page["elements"][0]["elementId"]
            builders[name](document_id, root_id)

            time_seconds = scenario.get("time", 0)
            response = self.rpc("tools/call", {
                "name": "render_png", "arguments": {
                    "documentId": document_id,
                    "region": {"x": 0, "y": 0, "width": 64, "height": 64},
                    "time": time_seconds,
                },
            })
            self.assertFalse(response["result"].get("isError"), response)
            result = response["result"]["structuredContent"]
            self.assertEqual((result["width"], result["height"]), (64, 64))
            image_block = next(item for item in response["result"]["content"]
                               if item["type"] == "image")
            png = base64.b64decode(image_block["data"], validate=True)
            actual_path = self.root / f"{name}.png"
            actual_path.write_bytes(png)
            golden_path = GOLDEN_DIR / scenario["golden"]
            pixdiff = subprocess.run(
                [str(ROOT / "dev"), "pixdiff", str(golden_path),
                 str(actual_path), "--tolerance", "0"],
                cwd=ROOT, text=True, capture_output=True, timeout=30, check=False)
            self.assertEqual(pixdiff.returncode, 0,
                             f"{name} pixel golden failed:\n{pixdiff.stdout}{pixdiff.stderr}")

            if "animation_list" in scenario["requiresTools"]:
                tracks = self.call_tool("animation_list", {
                    "documentId": document_id,
                })["tracks"]
                self.assertGreaterEqual(len(tracks), 1, name)
                expected_animation = {
                    "bouncing-ball-smil": "bounce",
                    "spinner-smil": "spin",
                    "morph-free-logo-reveal": "reveal",
                }[name]
                self.assertEqual(tracks[0]["animationId"], expected_animation)
            expected_id = {
                "flat-style-icon": "glyph",
                "gradient-sunset": "sun",
                "bouncing-ball-smil": "ball",
                "spinner-smil": "spinner-hand",
                "morph-free-logo-reveal": "logo-mark",
            }[name]
            element = self.call_tool("element_get", {
                "documentId": document_id, "elementId": expected_id,
            })
            self.assertEqual(element["elementId"], expected_id)
            source = self.call_tool("export_svg", {
                "documentId": document_id,
            })["svg"]
            self.assertTrue(f'id="{expected_id}"' in source or
                            f"id='{expected_id}'" in source)
            if name == "gradient-sunset":
                self.assertIn("linearGradient", source)
                self.assertIn("url(#sunset)", source)
            if name == "morph-free-logo-reveal":
                self.assertIn("attributeName='opacity'", source)
                self.assertNotIn("attributeName='d'", source)
            self.call_tool("save", {"documentId": document_id})
            saved_bytes = path.read_bytes()
            self.assertEqual(saved_bytes, source.encode("utf-8"), name)
            info = self.call_tool("document_info", {
                "documentId": document_id,
            })
            self.assertFalse(info["dirty"], name)
            ran += 1
        if ran == 0:
            self.skipTest(f"feature probes found missing MCP tools: {missing_by_scenario}")
        print(f"agent scenarios passed: {ran}; feature-gated: {missing_by_scenario}")


if __name__ == "__main__":
    unittest.main()
