"""Helper protocol tests (no model needed): python3 -m unittest tools/vexel-audio/test_helper.py"""
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import unittest

HELPER = os.path.join(os.path.dirname(os.path.abspath(__file__)), "vexel-audio")


class Session:
    def __init__(self, env=None):
        self.p = subprocess.Popen([sys.executable, HELPER], stdin=subprocess.PIPE,
                                  stdout=subprocess.PIPE, text=True, env=env)

    def ask(self, req):
        self.p.stdin.write((req if isinstance(req, str) else json.dumps(req)) + "\n")
        self.p.stdin.flush()
        return json.loads(self.p.stdout.readline())

    def close(self):
        self.p.stdin.close()
        return self.p.wait(10)


class HelperTests(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.addCleanup(self.dir.cleanup)

    def out(self, name):
        return os.path.join(self.dir.name, name)

    def test_sfx_is_deterministic_pcm16_48k(self):
        s = Session()
        digests = []
        for name in ("a.wav", "b.wav"):
            r = s.ask({"id": 1, "kind": "sfx", "params": {"preset": "launch", "seed": 5}, "outPath": self.out(name)})
            self.assertTrue(r["ok"], r)
            digests.append(hashlib.sha256(open(self.out(name), "rb").read()).hexdigest())
        self.assertEqual(digests[0], digests[1])
        head = open(self.out("a.wav"), "rb").read(44)
        self.assertEqual(head[8:12], b"WAVE")
        self.assertEqual(int.from_bytes(head[24:28], "little"), 48000)
        self.assertEqual(int.from_bytes(head[34:36], "little"), 16)
        self.assertEqual(s.close(), 0)

    def test_music_loops_and_validates(self):
        s = Session()
        r = s.ask({"id": 2, "kind": "music", "params": {"mood": "calm", "key": "C", "tempo": 120, "bars": 4}, "outPath": self.out("m.wav")})
        self.assertTrue(r["ok"], r)
        self.assertEqual(r["durationMs"], 8000)
        r = s.ask({"id": 3, "kind": "music", "params": {"bars": 500}, "outPath": self.out("m2.wav")})
        self.assertEqual(r["code"], "bad_params")
        s.close()

    def test_errors_do_not_kill_the_helper(self):
        s = Session()
        self.assertEqual(s.ask("not json")["code"], "bad_request")
        self.assertEqual(s.ask({"id": 1, "kind": "x", "params": {}, "outPath": self.out("x.wav")})["code"], "bad_kind")
        self.assertEqual(s.ask({"id": 1, "kind": "sfx", "params": {"preset": "chime"}, "outPath": "rel.wav"})["code"], "bad_path")
        self.assertEqual(s.ask("x" * 70000)["code"], "bad_request")
        self.assertTrue(s.ask({"id": 9, "kind": "sfx", "params": {"preset": "chime"}, "outPath": self.out("c.wav")})["ok"])
        s.close()

    def test_voice_without_model_is_model_missing(self):
        env = dict(os.environ, HF_HUB_CACHE=self.out("empty-cache"))
        s = Session(env)
        r = s.ask({"id": 4, "kind": "voice", "params": {"text": "hello"}, "outPath": self.out("v.wav")})
        self.assertFalse(r["ok"])
        self.assertEqual(r["code"], "model_missing")
        self.assertIn("--download-model", r["error"])
        self.assertFalse(os.path.exists(self.out("empty-cache")), "must not download without consent")
        s.close()

    def test_selftest(self):
        r = subprocess.run([sys.executable, HELPER, "--selftest"], capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stderr)


if __name__ == "__main__":
    unittest.main()
