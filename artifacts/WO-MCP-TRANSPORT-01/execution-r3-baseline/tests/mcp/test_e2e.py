from __future__ import annotations

import base64
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import signal
import select
import socket
import subprocess
import struct
import tempfile
import time
import unittest
import zlib


ROOT = Path(__file__).resolve().parents[2]
BIN = ROOT / "build/mac-debug/bin"
PROTOCOL_VERSION = "2026-07-28"


class McpStdioRelayTests(unittest.TestCase):
    """Public executable transport checks; the fixture implements no MCP tools."""

    MAX_BYTES = 2 * 1024 * 1024
    WAIT_SECONDS = 5.0

    def setUp(self) -> None:
        # Keep sun_path short even when the host's TMPDIR has a long prefix.
        temporary = tempfile.TemporaryDirectory(prefix="vxm-", dir="/tmp")
        self.addCleanup(temporary.cleanup)
        self.socket_path = Path(temporary.name) / "m.sock"
        listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.addCleanup(listener.close)
        try:
            listener.bind(str(self.socket_path))
        except PermissionError:
            self.skipTest("execution sandbox denies Unix-domain socket bind (EPERM)")
        self.socket_path.chmod(0o600)
        listener.listen(1)
        listener.settimeout(self.WAIT_SECONDS)
        environment = dict(os.environ, VEXEL_MCP_SOCKET=str(self.socket_path))
        self.server = subprocess.Popen(
            [str(BIN / "vexel-mcp")], cwd=ROOT, env=environment,
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            bufsize=0)
        self.addCleanup(self._stop_server)
        self.broker, _ = listener.accept()
        self.broker.settimeout(self.WAIT_SECONDS)
        self.addCleanup(self.broker.close)
        self.pool = ThreadPoolExecutor(max_workers=1)
        self.addCleanup(self.pool.shutdown, wait=True)
        self.stdout_tail = bytearray()
        self.broker_tail = bytearray()

    def _stop_server(self) -> None:
        if self.server.poll() is None:
            self.server.terminate()
            try:
                self.server.wait(timeout=self.WAIT_SECONDS)
            except subprocess.TimeoutExpired:
                self.server.kill()
                self.server.wait(timeout=self.WAIT_SECONDS)
        for pipe in (self.server.stdin, self.server.stdout, self.server.stderr):
            pipe.close()

    @staticmethod
    def wire(message: dict[str, object]) -> bytes:
        return json.dumps(message, separators=(",", ":")).encode() + b"\n"

    def send_client(self, data: bytes) -> None:
        view = memoryview(data)
        deadline = time.monotonic() + self.WAIT_SECONDS
        descriptor = self.server.stdin.fileno()
        os.set_blocking(descriptor, False)
        try:
            while view:
                remaining = deadline - time.monotonic()
                if remaining <= 0 or not select.select([], [descriptor], [], remaining)[1]:
                    raise AssertionError("relay stopped accepting client input")
                try:
                    count = os.write(descriptor, view[:8192])
                except BlockingIOError:
                    continue
                view = view[count:]
        finally:
            os.set_blocking(descriptor, True)

    def read_line(self, descriptor: int, tail: bytearray) -> bytes:
        deadline = time.monotonic() + self.WAIT_SECONDS
        while b"\n" not in tail:
            remaining = deadline - time.monotonic()
            self.assertGreater(remaining, 0, "timed out waiting for relay line")
            self.assertTrue(select.select([descriptor], [], [], remaining)[0],
                            "timed out waiting for relay line")
            chunk = os.read(descriptor, 8192)
            self.assertTrue(chunk, "relay closed before a complete line")
            tail.extend(chunk)
        end = tail.index(b"\n") + 1
        line = bytes(tail[:end])
        del tail[:end]
        return line

    def client_line(self) -> bytes:
        return self.read_line(self.server.stdout.fileno(), self.stdout_tail)

    def broker_line(self) -> bytes:
        return self.read_line(self.broker.fileno(), self.broker_tail)

    def finish(self, expected_code: int = 0) -> bytes:
        self.server.stdin.close()
        self.broker.shutdown(socket.SHUT_WR)
        self.assertEqual(self.server.wait(timeout=self.WAIT_SECONDS), expected_code)
        rest = bytes(self.stdout_tail) + self.server.stdout.read()
        self.stdout_tail.clear()
        error = self.server.stderr.read()
        if expected_code == 0:
            self.assertEqual(error, b"")
        else:
            self.assertIn(b"bounded stdio/socket relay failed", error)
        return rest

    def test_partial_coalesced_progress_and_out_of_order_response_ids(self) -> None:
        first = self.wire({"jsonrpc": "2.0", "id": "first", "method": "slow"})
        second = self.wire({"jsonrpc": "2.0", "id": 42, "method": "later"})
        self.send_client(first[:13])
        self.send_client(first[13:] + second)
        self.assertEqual(self.broker_line(), first)
        self.assertEqual(self.broker_line(), second)
        progress = self.wire({"jsonrpc": "2.0", "method": "notifications/progress",
                              "params": {"progressToken": "p", "progress": 1}})
        later = self.wire({"jsonrpc": "2.0", "id": 42, "result": {}})
        earlier = self.wire({"jsonrpc": "2.0", "id": "first", "result": {}})
        self.broker.sendall(progress[:17])
        # Round trip a notification while the broker line is incomplete.
        barrier = self.wire({"jsonrpc": "2.0", "method": "barrier"})
        self.send_client(barrier)
        self.assertEqual(self.broker_line(), barrier)
        self.assertFalse(self.stdout_tail)
        self.assertFalse(select.select([self.server.stdout], [], [], 0)[0])
        self.broker.sendall(progress[17:] + later + earlier)
        self.assertEqual([self.client_line() for _ in range(3)], [progress, later, earlier])
        self.assertEqual(self.finish(), b"")

    def test_cancellation_while_pending_has_no_synthetic_response(self) -> None:
        original = self.wire({"jsonrpc": "2.0", "id": "cancel-me", "method": "slow"})
        self.send_client(original)
        self.assertEqual(self.broker_line(), original)
        progress = self.wire({"jsonrpc": "2.0", "method": "notifications/progress",
                              "params": {"progressToken": "pending", "progress": 1}})
        self.broker.sendall(progress)
        self.assertEqual(self.client_line(), progress)
        cancelled = self.wire({"jsonrpc": "2.0", "method": "notifications/cancelled",
                               "params": {"requestId": "cancel-me", "reason": "test"}})
        later = self.wire({"jsonrpc": "2.0", "id": "next", "method": "ping"})
        self.send_client(cancelled + later)
        self.assertEqual(self.broker_line(), cancelled)
        self.assertEqual(self.broker_line(), later)
        response = self.wire({"jsonrpc": "2.0", "id": "next", "result": {}})
        self.broker.sendall(response)
        self.assertEqual(self.client_line(), response)
        # Closure proves the entire stdout contains no reply for cancel-me.
        self.assertEqual(self.finish(), b"")

    def test_stdin_eof_half_closes_and_drains_broker(self) -> None:
        request = self.wire({"jsonrpc": "2.0", "id": 7, "method": "last"})
        self.send_client(request[:-1])
        self.server.stdin.close()
        self.assertEqual(self.broker_line(), request)
        self.assertEqual(self.broker.recv(1), b"", "broker input was not half-closed")
        progress = self.wire({"jsonrpc": "2.0", "method": "notifications/progress"})
        response = self.wire({"jsonrpc": "2.0", "id": 7, "result": {}})
        self.broker.sendall(progress + response[:11])
        self.assertEqual(self.client_line(), progress)
        self.broker.sendall(response[11:])
        self.assertEqual(self.client_line(), response)
        self.assertEqual(self.finish(), b"")

    def test_oversized_stdin_discards_once_and_recovers(self) -> None:
        self.send_client(b"x" * (self.MAX_BYTES + 8192))
        # Delimiter plus next line in one write proves discard-tail retention.
        valid = self.wire({"jsonrpc": "2.0", "id": 9, "method": "ping"})
        self.send_client(b"ignored\n" + valid.replace(b"\n", b"\r\n"))
        error = json.loads(self.client_line())
        self.assertEqual(error["id"], None)
        self.assertEqual(error["error"]["code"], -32700)
        self.assertEqual(self.broker_line(), valid)
        response = self.wire({"jsonrpc": "2.0", "id": 9, "result": {}})
        self.broker.sendall(response)
        self.assertEqual(self.client_line(), response)
        self.assertEqual(self.finish(), b"")

    def test_oversized_stdin_at_eof_emits_one_parse_error(self) -> None:
        self.send_client(b"x" * (self.MAX_BYTES + 1))
        self.server.stdin.close()
        self.assertEqual(json.loads(self.client_line())["error"]["code"], -32700)
        self.assertEqual(self.broker.recv(1), b"")
        self.assertEqual(self.finish(), b"")

    def test_oversized_broker_line_fails_without_partial_json(self) -> None:
        def send_oversized() -> None:
            try:
                self.broker.sendall(b"x" * (self.MAX_BYTES + 1) + b"\n")
            except BrokenPipeError:
                pass  # Relay may close as soon as the first excess byte arrives.
        future = self.pool.submit(send_oversized)
        self.assertEqual(self.server.wait(timeout=self.WAIT_SECONDS), 1)
        future.result(timeout=self.WAIT_SECONDS)
        self.assertEqual(self.server.stdout.read(), b"")
        self.assertIn(b"bounded stdio/socket relay failed", self.server.stderr.read())

    def test_truncated_broker_line_fails_without_partial_json(self) -> None:
        self.broker.sendall(b'{"jsonrpc":"2.0","id":1')
        self.broker.shutdown(socket.SHUT_WR)
        self.assertEqual(self.server.wait(timeout=self.WAIT_SECONDS), 1)
        self.assertEqual(self.server.stdout.read(), b"")
        self.assertIn(b"bounded stdio/socket relay failed", self.server.stderr.read())

    def test_exact_limit_lines_are_preserved_in_both_directions(self) -> None:
        prefix = b'{"jsonrpc":"2.0","id":1,"result":"'
        line = prefix + b"a" * (self.MAX_BYTES - len(prefix) - 2) + b'"}\n'
        future = self.pool.submit(self.send_client, line)
        self.assertEqual(self.broker_line(), line)
        future.result(timeout=self.WAIT_SECONDS)
        future = self.pool.submit(self.broker.sendall, line)
        self.assertEqual(self.client_line(), line)
        future.result(timeout=self.WAIT_SECONDS)
        self.assertEqual(self.finish(), b"")

    def test_socket_backpressure_still_forwards_broker_progress(self) -> None:
        prefix = b'{"jsonrpc":"2.0","id":1,"method":"slow","params":{"pad":"'
        request = prefix + b"p" * (self.MAX_BYTES - len(prefix) - 3) + b'"}}\n'
        future = self.pool.submit(self.send_client, request)
        future.result(timeout=self.WAIT_SECONDS)
        # No broker reads yet: a 2 MiB request exceeds the socket send buffer.
        progress = self.wire({"jsonrpc": "2.0", "method": "notifications/progress",
                              "params": {"progressToken": "p", "progress": 1}})
        self.broker.sendall(progress)
        self.assertEqual(self.client_line(), progress)
        self.assertEqual(self.broker_line(), request)
        self.assertEqual(self.finish(), b"")

    def test_local_parse_error_never_splits_broker_stdout_line(self) -> None:
        prefix = b'{"jsonrpc":"2.0","id":1,"result":"'
        response = prefix + b"p" * (self.MAX_BYTES - len(prefix) - 2) + b'"}\n'
        future = self.pool.submit(self.broker.sendall, response)
        future.result(timeout=self.WAIT_SECONDS)
        self.send_client(b"x" * (self.MAX_BYTES + 1) + b"\n")
        lines = [self.client_line(), self.client_line()]
        self.assertIn(response, lines)
        error = json.loads(next(line for line in lines if line != response))
        self.assertEqual(error["error"]["code"], -32700)
        self.assertEqual(self.finish(), b"")

    def test_stdout_backpressure_still_forwards_cancellation(self) -> None:
        original = self.wire({"jsonrpc": "2.0", "id": 1, "method": "slow"})
        self.send_client(original)
        self.assertEqual(self.broker_line(), original)
        prefix = b'{"jsonrpc":"2.0","method":"notifications/progress","params":{"pad":"'
        progress = prefix + b"p" * (self.MAX_BYTES - len(prefix) - 3) + b'"}}\n'
        future = self.pool.submit(self.broker.sendall, progress)
        future.result(timeout=self.WAIT_SECONDS)
        # The maximum-size line cannot fit in the unread stdout pipe.
        # Don't consume stdout until the cancellation has reached the broker.
        cancelled = self.wire({"jsonrpc": "2.0", "method": "notifications/cancelled",
                               "params": {"requestId": 1}})
        self.send_client(cancelled)
        self.assertEqual(self.broker_line(), cancelled)
        self.assertEqual(self.client_line(), progress)
        future.result(timeout=self.WAIT_SECONDS)
        self.assertEqual(self.finish(), b"")


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
        return response["result"]

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
            "element_get", "element_query", "attribute_set", "attribute_remove", "text_set",
            "element_add", "element_add_fragment", "element_delete", "element_duplicate", "element_reorder",
            "element_group", "element_ungroup", "transaction", "undo", "redo", "save",
            "save_as", "render_png", "animation_list", "animation_add", "animation_set",
            "animation_remove", "export_animation", "selection_get", "selection_set", "export_svg",
            "native_timeline_get", "native_timeline_add_track",
            "native_timeline_add_drawing_asset", "native_timeline_add_property_track",
            "native_timeline_set_property_key", "native_timeline_set_cel",
            "native_timeline_remove_cel", "native_timeline_move_item",
            "native_timeline_set_loop", "native_timeline_undo", "native_timeline_redo",
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
