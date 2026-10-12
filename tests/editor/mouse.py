#!/usr/bin/env python3
"""Post real mouse events and use Vexel's live test layout dump for coordinates."""
import json
from pathlib import Path
import subprocess
import sys
import time

import Quartz


WINDOW_OWNER = "svgworkbench"
CHECKED_STATE = 1 << 1


def post(kind, x, y):
    event = Quartz.CGEventCreateMouseEvent(None, kind, (x, y), Quartz.kCGMouseButtonLeft)
    Quartz.CGEventPost(Quartz.kCGHIDEventTap, event)
    time.sleep(0.02)


def drag(x1, y1, x2, y2, steps=12):
    post(Quartz.kCGEventMouseMoved, x1, y1)
    post(Quartz.kCGEventLeftMouseDown, x1, y1)
    for i in range(1, steps + 1):
        post(Quartz.kCGEventLeftMouseDragged, x1 + (x2 - x1) * i / steps, y1 + (y2 - y1) * i / steps)
    post(Quartz.kCGEventLeftMouseUp, x2, y2)


def type_text(text):
    if not text.isascii():
        raise ValueError("type-text currently accepts ASCII editor input only")
    for character in text:
        down = Quartz.CGEventCreateKeyboardEvent(None, 0, True)
        Quartz.CGEventKeyboardSetUnicodeString(down, 1, character)
        Quartz.CGEventPost(Quartz.kCGHIDEventTap, down)
        up = Quartz.CGEventCreateKeyboardEvent(None, 0, False)
        Quartz.CGEventPost(Quartz.kCGHIDEventTap, up)
        time.sleep(0.02)


def on_screen_window():
    options = Quartz.kCGWindowListOptionOnScreenOnly | \
        Quartz.kCGWindowListExcludeDesktopElements
    windows = Quartz.CGWindowListCopyWindowInfo(options, Quartz.kCGNullWindowID)
    if windows is None:
        return None
    for window in windows:
        if window.get(Quartz.kCGWindowOwnerName) != WINDOW_OWNER:
            continue
        bounds = window.get(Quartz.kCGWindowBounds)
        window_id = window.get(Quartz.kCGWindowNumber)
        if bounds is not None and window_id is not None:
            return int(window_id), bounds
    return None


def read_layout(path):
    with Path(path).open("r", encoding="utf-8") as stream:
        layout = json.load(stream)
    for key in ("window_content_origin", "backing_scale", "canvas", "document",
                "view", "nodes"):
        if key not in layout:
            raise ValueError(f"layout dump is missing {key}")
    if float(layout["backing_scale"]) <= 0 or not layout["nodes"]:
        raise ValueError("layout dump has no usable canvas or editor nodes")
    return layout


def wait_for_layout(path, timeout=10.0):
    deadline = time.monotonic() + timeout
    last_error = "layout dump is not ready"
    while time.monotonic() < deadline:
        try:
            layout = read_layout(path)
            window = on_screen_window()
            if window is not None:
                _, bounds = window
                origin = layout["window_content_origin"]
                x = float(bounds["X"])
                y = float(bounds["Y"])
                height = float(bounds["Height"])
                if abs(float(origin["x"]) - x) < 1.0 and \
                        y <= float(origin["y"]) < y + height:
                    return layout
                last_error = "layout origin has not caught up with the real window"
        except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
            last_error = str(error)
        time.sleep(0.05)
    raise TimeoutError(f"timed out waiting for {path}: {last_error}")


def screen_point(layout, local_x, local_y):
    origin = layout["window_content_origin"]
    scale = float(layout["backing_scale"])
    return (float(origin["x"]) + local_x / scale,
            float(origin["y"]) + local_y / scale)


def document_point(layout, document_x, document_y):
    view = layout["view"]
    scale = float(view["scale"])
    canvas = layout["canvas"]
    x = float(view["translate_x"]) + float(document_x) * scale
    y = float(view["translate_y"]) + float(document_y) * scale
    if x < float(canvas["x"]) or y < float(canvas["y"]) or \
            x >= float(canvas["x"] + canvas["width"]) or \
            y >= float(canvas["y"] + canvas["height"]):
        raise ValueError("document point maps outside the live canvas rectangle")
    return screen_point(layout, x, y)


def node_point(layout, label, target="bounds"):
    nodes = [node for node in layout["nodes"] if node.get("label") == label]
    if not nodes:
        labels = [node.get("label", "") for node in layout["nodes"]
                  if node.get("label")]
        raise ValueError(f"layout dump has no node labeled {label!r}; labels={labels!r}")
    node = nodes[0]
    rect = node.get("targets", {}).get(target) if target != "bounds" else None
    if rect is None:
        rect = node.get("bounds")
    if rect is None or rect["width"] <= 0 or rect["height"] <= 0:
        raise ValueError(f"node {label!r} has no hit region for {target!r}")
    local_x = float(rect["x"]) + float(rect["width"]) * 0.5
    local_y = float(rect["y"]) + float(rect["height"]) * 0.5
    return screen_point(layout, local_x, local_y)


def assert_node(layout, label, condition):
    nodes = [node for node in layout["nodes"] if node.get("label") == label]
    if condition == "present":
        if nodes or any(label in node.get("label", "") or
                        label in node.get("value", "") for node in layout["nodes"]):
            return
        raise AssertionError(f"layout has no node labeled {label!r}")
    if not nodes:
        raise AssertionError(f"layout dump has no node labeled {label!r}")
    if condition == "state=checked":
        if any(int(node.get("state", 0)) & CHECKED_STATE for node in nodes):
            return
        raise AssertionError(f"node {label!r} is not selected")
    if condition.startswith("not-contains="):
        expected = condition[len("not-contains="):]
        if all(expected not in node.get("value", "") for node in nodes):
            return
        values = [node.get("value", "") for node in nodes]
        raise AssertionError(
            f"node {label!r} values {values!r} still contain {expected!r}")
    if condition.startswith("contains="):
        expected = condition[len("contains="):]
        if any(expected in node.get("value", "") for node in nodes):
            return
        raise AssertionError(f"node {label!r} value does not contain {expected!r}")
    if condition.startswith("value="):
        expected = condition[len("value="):]
        if any(node.get("value", "") == expected for node in nodes):
            return
        values = [node.get("value", "") for node in nodes]
        raise AssertionError(f"node {label!r} has values {values!r}, expected {expected!r}")
    raise ValueError(f"unknown node condition {condition!r}")


def main(argv):
    if len(argv) == 4 and argv[1] == "click":
        x, y = float(argv[2]), float(argv[3])
        drag(x, y, x, y, 1)
    elif len(argv) == 6 and argv[1] == "drag":
        drag(*(float(v) for v in argv[2:6]))
    elif len(argv) == 3 and argv[1] == "type-text":
        type_text(argv[2])
    elif len(argv) == 5 and argv[1] == "click-doc":
        x, y = document_point(read_layout(argv[2]), float(argv[3]), float(argv[4]))
        drag(x, y, x, y, 1)
    elif len(argv) == 7 and argv[1] == "drag-doc":
        layout = read_layout(argv[2])
        start = document_point(layout, float(argv[3]), float(argv[4]))
        end = document_point(layout, float(argv[5]), float(argv[6]))
        drag(*start, *end)
    elif len(argv) in (4, 5) and argv[1] == "click-node":
        target = argv[4] if len(argv) == 5 else "bounds"
        x, y = node_point(read_layout(argv[2]), argv[3], target)
        drag(x, y, x, y, 1)
    elif len(argv) == 3 and argv[1] == "wait-layout":
        wait_for_layout(argv[2])
    elif len(argv) == 3 and argv[1] == "capture":
        capture_path = Path(argv[2])
        capture_path.parent.mkdir(parents=True, exist_ok=True)
        window = on_screen_window()
        if window is None:
            raise SystemExit("svgworkbench has no visible window to capture")
        result = subprocess.run(["screencapture", "-l", str(window[0]),
            str(capture_path)], check=False)
        if result.returncode != 0:
            raise SystemExit(f"screencapture exited with status {result.returncode}")
    elif len(argv) == 5 and argv[1] in ("assert-node", "wait-node"):
        path, label, condition = argv[2], argv[3], argv[4]
        if argv[1] == "assert-node":
            assert_node(read_layout(path), label, condition)
        else:
            deadline = time.monotonic() + 5.0
            last_error = "condition not satisfied"
            while time.monotonic() < deadline:
                try:
                    assert_node(read_layout(path), label, condition)
                    return
                except (OSError, AssertionError, ValueError, KeyError,
                        TypeError, json.JSONDecodeError) as error:
                    last_error = str(error)
                time.sleep(0.05)
            raise TimeoutError(f"timed out waiting for layout node: {last_error}")
    else:
        raise SystemExit(__doc__)
    time.sleep(0.2)


main(sys.argv)
