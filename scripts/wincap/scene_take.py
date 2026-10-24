#!/usr/bin/env python3
"""Record a paced MCP scene build in the live Vexel window with wincap.

usage: scene_take.py <scene_walk_home|scene_logo|scene_loader> <out.mp4> [grant_dir]
Patches scene_common so the app it spawns writes into <grant_dir>, resizes only
that app's window, starts wincap (window filter by pid) before the first tool
call, and stops it after the scene script finishes.
"""
import os
import runpy
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
scene, out = sys.argv[1], sys.argv[2]
grant = sys.argv[3] if len(sys.argv) > 3 else "/private/tmp/vexel-video/grant"
WINCAP = os.environ.get("WINCAP", "/private/tmp/vexel-video/wincap")
os.makedirs(grant, exist_ok=True)
sys.argv = [scene + ".py", "--paced"]
sys.path.insert(0, os.path.join(REPO, "scripts"))
import scene_common as C  # noqa: E402

C.OUT = grant
cap = None
orig = C.Mcp.rpc
state = {"started": False}


def rpc(self, method, params=None, cost=1.0):
    global cap
    if method == "tools/call" and not state["started"]:
        state["started"] = True
        pid = self.p.pid
        scr = ('tell application "System Events" to tell (first process whose unix id is %d)\n'
               'set frontmost to true\nrepeat 40 times\nif (count of windows) > 0 then exit repeat\n'
               'delay 0.25\nend repeat\nset position of window 1 to {60, 60}\n'
               'set size of window 1 to {1440, 900}\nend tell' % pid)
        subprocess.run(["osascript", "-e", scr], check=False)
        time.sleep(1.0)
        cap = subprocess.Popen([WINCAP, "--pid", str(pid), "--out", out, "--fps", "60"],
                               stdin=subprocess.PIPE, text=True)
        time.sleep(2.5)
    return orig(self, method, params, cost)


C.Mcp.rpc = rpc
try:
    runpy.run_path(os.path.join(REPO, "scripts", scene + ".py"), run_name="__main__")
finally:
    time.sleep(1.5)
    if cap:
        try:
            cap.stdin.write("q\n")
            cap.stdin.flush()
        except (BrokenPipeError, OSError):
            pass  # wincap already finalized itself (window closed)
        cap.wait(timeout=30)
