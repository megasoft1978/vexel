from __future__ import annotations

import base64
import json
import os
from pathlib import Path
import signal
import subprocess
import struct
import tempfile
import time
import unittest
import zlib


ROOT = Path(__file__).resolve().parents[2]
BIN = ROOT / "build/mac-debug/bin"
PROTOCOL_VERSION = "2026-07-28"


class McpStdioEndToEndTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="vexel-mcp-test-")
        self.root = Path(self.temporary.name)
        self.granted = self.root / "granted"
        self.granted.mkdir(mode=0o700)
        self.socket_path = self.root / "m.sock"
        self.current_path = self.granted / "current.svg"
        self.other_path = self.granted / "other.svg"
        self.save_path = self.granted / "save-target.svg"
        self.save_new_path = self.granted / "save-new.svg"
        image_data = self._tiny_png_data_uri()
        self.current_path.write_text(
            '<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100">'
            '<rect id="r1" x="4" y="5" width="20" height="12" fill="red">'
            '<animate id="grow" attributeName="width" from="20" to="40" dur="2s" fill="freeze"/>'
            '</rect>'
            '<circle id="c1" cx="60" cy="40" r="8" fill="blue"/>'
            '<title id="title">Old</title>'
            '<text id="media-text" x="3" y="55" font-size="12">Media</text>'
            f'<image id="media-image" x="70" y="70" width="8" height="8" href="{image_data}"/>'
            '</svg>', encoding="utf-8")
        self.other_path.write_text(
            '<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20">'
            '<rect id="other" width="10" height="10" fill="green"/></svg>', encoding="utf-8")
        self.save_path.write_text('<svg xmlns="http://www.w3.org/2000/svg"/>', encoding="utf-8")
        environment = os.environ.copy()
        environment["VEXEL_MCP_SOCKET"] = str(self.socket_path)
        environment["VEXEL_MCP_ALLOWED_DIRS"] = str(self.granted)
        self.broker = subprocess.Popen(
            [str(BIN / "mcp_headless_broker"), str(self.current_path)],
            cwd=ROOT, env=environment, stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=False)
        deadline = time.monotonic() + 5.0
        while time.monotonic() < deadline and not self.socket_path.exists():
            if self.broker.poll() is not None:
                error = self.broker.stderr.read()
                if b"bind failed: Operation not permitted" in error:
                    self.temporary.cleanup()
                    self.skipTest("execution sandbox denies Unix-domain socket bind (EPERM)")
                raise AssertionError(f"headless broker exited early: {error!r}")
            time.sleep(0.01)
        self.assertTrue(self.socket_path.exists(), "headless broker did not create its UDS")
        self.server = subprocess.Popen(
            [str(BIN / "vexel-mcp")], cwd=ROOT, env=environment,
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            bufsize=0)
        self.request_id = 0
        discovery = self.rpc("server/discover", {"_meta": self.meta()})
        self.assertEqual(discovery["result"]["supportedVersions"], [PROTOCOL_VERSION])

    def tearDown(self) -> None:
        if getattr(self, "server", None) is not None:
            if self.server.poll() is None:
                self.server.terminate()
                try:
                    self.server.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    self.server.kill()
                    self.server.wait(timeout=3)
        if getattr(self, "broker", None) is not None:
            if self.broker.poll() is None:
                self.broker.terminate()
                try:
                    self.broker.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    self.broker.kill()
                    self.broker.wait(timeout=3)
        self.temporary.cleanup()

    def meta(self) -> dict[str, object]:
        return {"io.modelcontextprotocol/protocolVersion": PROTOCOL_VERSION,
                "io.modelcontextprotocol/clientCapabilities": {}}

    def rpc(self, method: str, params: dict[str, object]) -> dict[str, object]:
        self.request_id += 1
        request = {"jsonrpc": "2.0", "id": self.request_id, "method": method,
                   "params": dict(params)}
        if method != "server/discover":
            request["params"]["_meta"] = self.meta()
        assert self.server.stdin is not None and self.server.stdout is not None
        self.server.stdin.write(json.dumps(request, separators=(",", ":")).encode() + b"\n")
        self.server.stdin.flush()
        response_line = self.server.stdout.readline()
        if not response_line:
            stderr = self.server.stderr.read() if self.server.stderr is not None else b""
            raise AssertionError(f"vexel-mcp exited without a response: {stderr!r}")
        response = json.loads(response_line)
        self.assertEqual(response.get("id"), self.request_id)
        return response

    def call_tool(self, name: str, arguments: dict[str, object]) -> dict[str, object]:
        response = self.rpc("tools/call", {"name": name, "arguments": arguments})
        self.assertIn("result", response)
        result = response["result"]
        error = result.get("structuredContent", {}).get("error", {})
        if not (error.get("code") == "invalid_arguments" and
                error.get("message") == "unexpected argument"):
            self.assertEqual(result["structuredContent"].get("mode"), "headless", result)
        return result

    def expect_tool_error(self, name: str, arguments: dict[str, object], code: str) -> None:
        result = self.call_tool(name, arguments)
        self.assertTrue(result.get("isError"), result)
        self.assertEqual(result["structuredContent"]["error"]["code"], code)
        self.assertTrue(result["content"][0]["text"])

    def test_current_protocol_full_tool_surface_and_raw_stdio(self) -> None:
        listed = self.rpc("tools/list", {})["result"]["tools"]
        tool_names = {item["name"] for item in listed}
        expected = {
            "document_open", "document_close", "document_list", "document_info", "tree_get",
            "element_tree", "element_get", "element_query", "attribute_set", "attribute_remove", "text_set",
            "element_add", "element_add_fragment", "element_delete", "element_duplicate", "element_reorder",
            "element_create", "element_set", "element_group", "element_ungroup", "transform_set",
            "transform_apply", "style_fill", "style_stroke", "style_opacity", "gradient_create",
            "gradient_stop_add", "gradient_stop_set", "gradient_stop_remove", "path_from_commands",
            "path_append", "path_edit_node", "element_align", "element_distribute", "batch",
            "transaction", "undo", "redo", "save",
            "save_as", "render_png", "animation_list", "animation_add", "animation_set",
            "animation_remove", "export_animation", "selection_get", "selection_set", "export_svg",
        }
        self.assertEqual(tool_names, expected)
        for tool in listed:
            self.assertFalse(tool["inputSchema"].get("additionalProperties", True))

        documents = self.call_tool("document_list", {})["structuredContent"]["documents"]
        self.assertEqual(len(documents), 1)
        document_id = documents[0]["documentId"]
        info = self.call_tool("document_info", {"documentId": document_id})["structuredContent"]
        self.assertEqual(info["sourceBytes"], self.current_path.stat().st_size)
        self.assertFalse(info["dirty"])

        tree = self.call_tool("tree_get", {"documentId": document_id, "limit": 2})["structuredContent"]
        self.assertEqual(len(tree["elements"]), 2)
        self.assertFalse(tree["complete"])
        root_id = tree["elements"][0]["elementId"]
        page2 = self.call_tool("tree_get", {"documentId": document_id,
                                                "cursor": tree["cursor"], "limit": 8})["structuredContent"]
        self.assertTrue(page2["complete"])
        element = self.call_tool("element_get", {"documentId": document_id,
                                                   "elementId": "r1"})["structuredContent"]
        self.assertEqual(element["tag"], "rect")
        self.assertTrue(any(item["name"] == "fill" and item["value"] == "red"
                            for item in element["attributes"]))
        query = self.call_tool("element_query", {"documentId": document_id,
                                                    "tag": "circle"})["structuredContent"]
        self.assertEqual(query["elements"][0]["elementId"], "c1")
        self.assertEqual(query["mode"], "headless")
        detailed_tree = self.call_tool("element_tree", {"documentId": document_id,
            "includeAttributes": True, "includeBounds": True, "limit": 32})["structuredContent"]
        self.assertEqual(detailed_tree["mode"], "headless")
        self.assertTrue(any(row["elementId"] == "r1" and row["bounds"] is not None
                            for row in detailed_tree["elements"]))
        selector = self.call_tool("element_query", {"documentId": document_id,
            "selector": "#c1"})["structuredContent"]
        self.assertEqual(selector["elements"][0]["elementId"], "c1")
        by_id = self.call_tool("element_query", {"documentId": document_id,
            "id": "r1"})["structuredContent"]
        self.assertEqual(by_id["elements"][0]["elementId"], "r1")
        hit = self.call_tool("element_query", {"documentId": document_id,
            "hitTest": {"x": 10, "y": 10}})["structuredContent"]
        self.assertIn("r1", [row["elementId"] for row in hit["elements"]])

        created_scene = self.call_tool("element_create", {"documentId": document_id,
            "parentId": root_id, "tag": "rect", "index": 0,
            "attributes": {"id": "scene-rect", "x": "2", "y": "3",
                           "width": "8", "height": "6"},
            "style": {"fill": "red", "stroke": "black"}})["structuredContent"]
        self.assertEqual(created_scene["elementId"], "scene-rect")
        self.assertEqual(created_scene["mode"], "headless")
        self.expect_tool_error("element_create", {"documentId": document_id,
            "parentId": root_id, "tag": "rect", "index": 999}, "invalid_argument")
        self.call_tool("style_stroke", {"documentId": document_id,
            "elementId": "scene-rect", "value": "blue"})
        self.call_tool("style_opacity", {"documentId": document_id,
            "elementId": "scene-rect", "value": "0.7"})
        self.call_tool("style_fill", {"documentId": document_id,
            "elementId": "scene-rect", "value": "orange"})
        self.call_tool("element_set", {"documentId": document_id,
            "elementId": "scene-rect", "attributes": {"opacity": "0.8"},
            "style": {"stroke": "navy", "stroke-width": "2"}})
        self.call_tool("transform_apply", {"documentId": document_id,
            "elementId": "scene-rect", "operation": "rotate", "angle": 15,
            "cx": 6, "cy": 6})
        scene_svg = self.call_tool("export_svg", {"documentId": document_id})["structuredContent"]["svg"]
        self.assertIn("rotate(15 6 6)", scene_svg)
        self.call_tool("transform_set", {"documentId": document_id,
            "elementId": "scene-rect", "value": "translate(1 2) scale(1.25)"})

        path = self.call_tool("path_from_commands", {"documentId": document_id,
            "parentId": root_id, "commands": "M 1 1 L 5 5 Z"})["structuredContent"]
        path_id = path["elementId"]
        self.call_tool("path_edit_node", {"documentId": document_id,
            "elementId": path_id, "nodeIndex": 1, "values": "7 9"})
        self.call_tool("path_append", {"documentId": document_id,
            "elementId": path_id, "commands": "L 10 12"})
        defs_id = self.call_tool("element_create", {"documentId": document_id,
            "parentId": root_id, "tag": "defs", "attributes": {"id": "scene-defs"}})["structuredContent"]["elementId"]
        gradient = self.call_tool("gradient_create", {"documentId": document_id,
            "parentId": defs_id, "kind": "linearGradient", "index": 0,
            "attributes": {"x1": "0%", "x2": "100%"}})["structuredContent"]
        gradient_id = gradient["elementId"]
        self.call_tool("gradient_stop_add", {"documentId": document_id,
            "gradientId": gradient_id, "index": 0, "offset": "0%", "color": "white"})
        self.call_tool("gradient_stop_add", {"documentId": document_id,
            "gradientId": gradient_id, "index": 1, "offset": "1", "color": "black"})
        self.call_tool("gradient_stop_add", {"documentId": document_id,
            "gradientId": gradient_id, "index": 2, "offset": "1", "color": "red"})
        self.call_tool("gradient_stop_set", {"documentId": document_id,
            "gradientId": gradient_id, "index": 1, "color": "blue"})
        self.call_tool("gradient_stop_remove", {"documentId": document_id,
            "gradientId": gradient_id, "index": 2})
        self.call_tool("style_fill", {"documentId": document_id,
            "elementId": "scene-rect", "value": f"url(#{gradient_id})"})
        filter_id = self.call_tool("element_create", {"documentId": document_id,
            "parentId": defs_id, "tag": "filter"})["structuredContent"]["elementId"]
        self.call_tool("element_create", {"documentId": document_id,
            "parentId": filter_id, "tag": "feGaussianBlur",
            "attributes": {"stdDeviation": "1"}})
        self.call_tool("element_create", {"documentId": document_id,
            "parentId": root_id, "tag": "image", "attributes": {
                "x": "80", "y": "80", "width": "8", "height": "8",
                "href": self._tiny_png_data_uri()}})
        for element_id, x in (("align-a", "4"), ("align-b", "30"), ("align-c", "60")):
            self.call_tool("element_create", {"documentId": document_id,
                "parentId": root_id, "tag": "rect", "attributes": {"id": element_id,
                "x": x, "y": "20", "width": "5", "height": "5"}})
        self.call_tool("element_align", {"documentId": document_id,
            "elementIds": ["align-a", "align-b"], "axis": "x", "alignment": "start"})
        self.call_tool("element_distribute", {"documentId": document_id,
            "elementIds": ["align-a", "align-b", "align-c"], "axis": "x"})
        batch = self.call_tool("batch", {"documentId": document_id, "operations": [
            {"op": "element_set", "elementId": "scene-rect", "attributes": {"fill": "purple"}},
            {"op": "element_set", "elementId": "scene-rect", "attributes": {"x": "12"}},
        ]})["structuredContent"]
        self.assertEqual(batch["undoSteps"], 1)
        self.assertEqual([row["applied"] for row in batch["results"]], [True, True])
        self.call_tool("undo", {"documentId": document_id})
        scene_after_undo = self.call_tool("element_get", {"documentId": document_id,
            "elementId": "scene-rect"})["structuredContent"]
        scene_attrs = {item["name"]: item["value"] for item in scene_after_undo["attributes"]}
        self.assertIn(f"url(#{gradient_id})", scene_attrs["style"])
        self.call_tool("redo", {"documentId": document_id})
        before_rollback = self.call_tool("export_svg", {"documentId": document_id})["structuredContent"]["svg"]
        self.expect_tool_error("batch", {"documentId": document_id, "operations": [
            {"op": "element_set", "elementId": "scene-rect", "attributes": {"fill": "cyan"}},
            {"op": "not_supported", "elementId": "scene-rect"},
        ]}, "batch_rolled_back")
        after_rollback = self.call_tool("export_svg", {"documentId": document_id})["structuredContent"]["svg"]
        self.assertEqual(after_rollback, before_rollback)

        animations = self.call_tool("animation_list", {"documentId": document_id})["structuredContent"]
        self.assertEqual(animations["tracks"][0]["animationId"], "grow")
        self.assertEqual(animations["tracks"][0]["targetId"], "r1")
        self.assertEqual(animations["tracks"][0]["attribute"], "width")
        self.assertGreater(animations["tracks"][0]["sourceSpan"]["length"], 0)

        self.call_tool("animation_set", {"documentId": document_id,
            "animationId": "grow", "name": "to", "value": "50"})
        changed = self.call_tool("animation_list", {"documentId": document_id})["structuredContent"]
        self.assertEqual(changed["tracks"][0]["keys"][-1]["value"]["value"], 50)

        rendered = self.call_tool("render_png", {"documentId": document_id,
            "region": {"x": 0, "y": 0, "width": 100, "height": 100}, "scale": 1, "time": 1})
        image_block = next(item for item in rendered["content"] if item["type"] == "image")
        png = base64.b64decode(image_block["data"], validate=True)
        self.assertEqual(png[:8], b"\x89PNG\r\n\x1a\n")

        self.call_tool("selection_set", {"documentId": document_id, "elementId": "r1"})
        self.assertEqual(self.call_tool("selection_get", {"documentId": document_id})
                         ["structuredContent"]["elementId"], "r1")
        self.call_tool("attribute_set", {"documentId": document_id, "elementId": "r1",
                                           "name": "stroke", "value": "black"})
        self.call_tool("attribute_remove", {"documentId": document_id, "elementId": "r1",
                                             "name": "stroke"})
        self.call_tool("text_set", {"documentId": document_id, "elementId": "title",
                                     "text": "Updated & safe"})
        created = self.call_tool("element_add", {"documentId": document_id, "parentId": root_id,
            "template": "rect", "attributes": {"id": "added", "x": "30", "y": "30",
            "width": "8", "height": "8", "fill": "green"}})["structuredContent"]
        self.assertEqual(created["elementId"], "added")
        duplicate = self.call_tool("element_duplicate", {"documentId": document_id,
                                                            "elementId": "r1"})["structuredContent"]
        self.assertTrue(duplicate["created"])
        animation_id_query = self.call_tool("element_query", {"documentId": document_id,
            "id": "grow"})["structuredContent"]
        self.assertEqual(len(animation_id_query["elements"]), 1)
        self.call_tool("element_delete", {"documentId": document_id,
                                           "elementId": duplicate["elementId"]})
        self.call_tool("element_group", {"documentId": document_id, "elementId": "added"})
        group_rows = self.call_tool("element_query", {"documentId": document_id, "tag": "g"})["structuredContent"]["elements"]
        self.assertTrue(group_rows)
        self.call_tool("element_ungroup", {"documentId": document_id,
                                            "elementId": group_rows[-1]["elementId"]})
        self.call_tool("element_reorder", {"documentId": document_id, "elementId": "r1",
                                            "targetId": "c1", "before": True})
        transaction = self.call_tool("transaction", {"documentId": document_id, "edits": [
            {"op": "attribute_set", "elementId": "r1", "name": "fill", "value": "purple"},
            {"op": "attribute_set", "elementId": "r1", "name": "stroke", "value": "black"},
        ]})["structuredContent"]
        self.assertTrue(transaction["atomic"])
        self.assertEqual(transaction["applied"], 2)
        self.expect_tool_error("transaction", {"documentId": document_id, "edits": [
            {"op": "attribute_set", "elementId": "r1", "name": "fill", "value": "red"},
            {"op": "not_supported", "elementId": "r1"},
        ]}, "transaction_rolled_back")
        self.call_tool("undo", {"documentId": document_id})
        self.call_tool("redo", {"documentId": document_id})
        added_text = self.call_tool("element_add", {"documentId": document_id,
            "parentId": root_id, "template": "text",
            "attributes": {"id": "new-text", "x": "2", "y": "80"}})["structuredContent"]
        self.assertEqual(added_text["elementId"], "new-text")
        self.call_tool("text_set", {"documentId": document_id,
            "elementId": "new-text", "text": "MCP <&>"})
        added_fragment = self.call_tool("element_add_fragment", {
            "documentId": document_id, "parentId": root_id,
            "xml": "<g id='fragment'><text id='fragment-text'>Fragment</text></g>"
        })["structuredContent"]
        self.assertEqual(added_fragment["elementId"], "fragment")
        self.expect_tool_error("element_add_fragment", {
            "documentId": document_id, "parentId": root_id,
            "xml": "<g><text></g>"}, "malformed_xml")
        self.expect_tool_error("element_add_fragment", {
            "documentId": document_id, "parentId": root_id,
            "xml": "<rect/><circle/>"}, "invalid_operation")
        self.expect_tool_error("element_delete", {"documentId": document_id,
            "elementId": root_id}, "root_delete")

        rendered = self.call_tool("render_png", {"documentId": document_id,
            "region": {"x": 0, "y": 0, "width": 100, "height": 100}, "scale": 1})
        image_block = next(item for item in rendered["content"] if item["type"] == "image")
        png = base64.b64decode(image_block["data"], validate=True)
        self.assertEqual(png[:8], b"\x89PNG\r\n\x1a\n")
        self.assertEqual(rendered["structuredContent"]["width"], 100)
        self.assertTrue(zlib.decompress(self._png_idat(png)))

        exported = self.call_tool("export_svg", {"documentId": document_id})["structuredContent"]["svg"]
        self.assertIn('id="r1"', exported)
        self.assertIn("Updated &amp; safe", exported)
        self.assertIn("MCP &lt;&amp;&gt;", exported)
        self.assertIn("<text id='fragment-text'>Fragment</text>", exported)
        resources = self.rpc("resources/list", {})["result"]["resources"]
        self.assertEqual(len(resources), 3)
        source_resource = self.rpc("resources/read", {"uri": resources[0]["uri"]})["result"]["contents"][0]
        self.assertIn('id="r1"', source_resource["text"])
        tree_resource = self.rpc("resources/read", {"uri": resources[1]["uri"]})["result"]["contents"][0]
        self.assertIn('"elements"', tree_resource["text"])
        render_resource = self.rpc("resources/read", {"uri": resources[2]["uri"]})["result"]["contents"][0]
        self.assertEqual(base64.b64decode(render_resource["blob"])[:8], b"\x89PNG\r\n\x1a\n")
        prompts = self.rpc("prompts/list", {})["result"]["prompts"]
        self.assertEqual(prompts[0]["name"], "inspect-document")
        prompt = self.rpc("prompts/get", {"name": "inspect-document", "arguments": {}})["result"]
        self.assertIn("messages", prompt)

        self.expect_tool_error("document_open", {"path": str(self.root / "outside.svg")}, "path_not_granted")
        opened = self.call_tool("document_open", {"path": str(self.other_path)})["structuredContent"]
        other_id = opened["documentId"]
        self.assertNotEqual(other_id, document_id)
        self.call_tool("attribute_set", {"documentId": other_id, "elementId": "other",
                                          "name": "stroke", "value": "black"})
        self.expect_tool_error("document_close", {"documentId": other_id}, "unsaved_changes")
        self.call_tool("undo", {"documentId": other_id})
        self.expect_tool_error("save_as", {"documentId": other_id,
            "path": str(self.root / "outside-new.svg")}, "path_not_granted")
        self.call_tool("save_as", {"documentId": other_id, "path": str(self.save_path)})
        self.assertTrue(self.save_path.read_text(encoding="utf-8").startswith("<svg"))
        self.assertFalse(self.save_new_path.exists())
        saved_new = self.call_tool("save_as", {"documentId": other_id,
            "path": str(self.save_new_path)})["structuredContent"]
        self.assertEqual(os.path.realpath(saved_new["path"]), os.path.realpath(self.save_new_path))
        self.assertTrue(self.save_new_path.is_file())
        self.assertTrue(self.save_new_path.read_text(encoding="utf-8").startswith("<svg"))
        self.assertTrue(self.call_tool("document_close", {"documentId": other_id})
                        ["structuredContent"]["closed"])
        self.expect_tool_error("document_info", {"documentId": other_id}, "document_not_found")
        self.expect_tool_error("document_close", {"documentId": document_id}, "active_document")
        self.call_tool("save", {"documentId": document_id})
        self.assertFalse(self.call_tool("document_info", {"documentId": document_id})
                         ["structuredContent"]["dirty"])

        self.expect_tool_error("attribute_set", {"documentId": document_id,
            "elementId": "r1", "name": "fill", "value": "red", "extra": 1}, "invalid_arguments")
        malformed = self.server.stdin
        assert malformed is not None
        malformed.write(b"{bad}\n")
        malformed.flush()
        parse_error = json.loads(self.server.stdout.readline())
        self.assertEqual(parse_error["error"]["code"], -32700)

        piped = []
        for method, params in (("ping", {}), ("tools/list", {}), ("resources/list", {})):
            self.request_id += 1
            piped.append({"jsonrpc": "2.0", "id": self.request_id, "method": method,
                          "params": {**params, "_meta": self.meta()}})
        self.server.stdin.write(b"".join(json.dumps(item).encode() + b"\n" for item in piped))
        self.server.stdin.flush()
        replies = [json.loads(self.server.stdout.readline()) for _ in piped]
        self.assertEqual([reply["id"] for reply in replies], [item["id"] for item in piped])

        self.server.stdin.write(b"x" * (2 * 1024 * 1024 + 1) + b"\n")
        self.server.stdin.flush()
        oversized = json.loads(self.server.stdout.readline())
        self.assertEqual(oversized["error"]["code"], -32700)

    @staticmethod
    def _png_idat(png: bytes) -> bytes:
        cursor = 8
        result = bytearray()
        while cursor < len(png):
            length = int.from_bytes(png[cursor:cursor + 4], "big")
            kind = png[cursor + 4:cursor + 8]
            payload = png[cursor + 8:cursor + 8 + length]
            if kind == b"IDAT":
                result.extend(payload)
            cursor += 12 + length
            if kind == b"IEND":
                break
        return bytes(result)

    @staticmethod
    def _tiny_png_data_uri() -> str:
        def chunk(kind: bytes, payload: bytes) -> bytes:
            content = kind + payload
            return (struct.pack(">I", len(payload)) + content +
                    struct.pack(">I", zlib.crc32(content) & 0xFFFFFFFF))

        png = (b"\x89PNG\r\n\x1a\n" +
               chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, 6, 0, 0, 0)) +
               chunk(b"IDAT", zlib.compress(b"\x00\xff\x00\x00\xff")) +
               chunk(b"IEND", b""))
        return "data:image/png;base64," + base64.b64encode(png).decode("ascii")


if __name__ == "__main__":
    unittest.main()
