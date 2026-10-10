#!/usr/bin/env python3
"""Post real mouse events (System Events has no drag): mouse.py click X Y | drag X1 Y1 X2 Y2 (screen points)."""
import sys
import time

import Quartz


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


def main(argv):
    if len(argv) == 4 and argv[1] == "click":
        x, y = float(argv[2]), float(argv[3])
        drag(x, y, x, y, 1)
    elif len(argv) == 6 and argv[1] == "drag":
        drag(*(float(v) for v in argv[2:6]))
    else:
        raise SystemExit(__doc__)
    time.sleep(0.2)


main(sys.argv)
