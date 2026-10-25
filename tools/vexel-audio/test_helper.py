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
        for name in (".a.vexel-audio-1.wav", ".b.vexel-audio-1.wav"):
            r = s.ask({"id": 1, "kind": "sfx", "params": {"preset": "launch", "seed": 5}, "outPath": self.out(name)})
            self.assertTrue(r["ok"], r)
            digests.append(hashlib.sha256(open(self.out(name), "rb").read()).hexdigest())
        self.assertEqual(digests[0], digests[1])
        head = open(self.out(".a.vexel-audio-1.wav"), "rb").read(44)
        self.assertEqual(head[8:12], b"WAVE")
        self.assertEqual(int.from_bytes(head[24:28], "little"), 48000)
        self.assertEqual(int.from_bytes(head[34:36], "little"), 16)
        self.assertEqual(s.close(), 0)

    def test_music_loops_and_validates(self):
        s = Session()
        r = s.ask({"id": 2, "kind": "music", "params": {"mood": "calm", "key": "C", "tempo": 120, "bars": 4}, "outPath": self.out(".m.vexel-audio-1.wav")})
        self.assertTrue(r["ok"], r)
        self.assertEqual(r["durationMs"], 8000)
        r = s.ask({"id": 3, "kind": "music", "params": {"bars": 500}, "outPath": self.out(".m2.vexel-audio-1.wav")})
        self.assertEqual(r["code"], "bad_params")
        s.close()

    def test_errors_do_not_kill_the_helper(self):
        s = Session()
        self.assertEqual(s.ask("not json")["code"], "bad_request")
        self.assertEqual(s.ask({"id": 1, "kind": "x", "params": {}, "outPath": self.out(".x.vexel-audio-1.wav")})["code"], "bad_kind")
        self.assertEqual(s.ask({"id": 1, "kind": "sfx", "params": {"preset": "chime"}, "outPath": "rel.wav"})["code"], "bad_path")
        self.assertEqual(s.ask("x" * 70000)["code"], "bad_request")
        self.assertTrue(s.ask({"id": 9, "kind": "sfx", "params": {"preset": "chime"}, "outPath": self.out(".c.vexel-audio-1.wav")})["ok"])
        s.close()

    def test_voice_without_model_is_model_missing(self):
        env = dict(os.environ, HF_HUB_CACHE=self.out("empty-cache"))
        s = Session(env)
        r = s.ask({"id": 4, "kind": "voice", "params": {"text": "hello"}, "outPath": self.out(".v.vexel-audio-1.wav")})
        self.assertFalse(r["ok"])
        self.assertEqual(r["code"], "model_missing")
        self.assertIn("--download-model", r["error"])
        self.assertFalse(os.path.exists(self.out("empty-cache")), "must not download without consent")
        s.close()

    def sfx(self, s, path):
        return s.ask({"id": 1, "kind": "sfx", "params": {"preset": "chime"}, "outPath": path})

    def test_existing_file_and_symlink_are_refused(self):
        s = Session()
        victim = self.out("victim.txt")
        open(victim, "w").write("keep")
        existing = self.out(".e.vexel-audio-1.wav")
        open(existing, "w").write("keep")
        self.assertEqual(self.sfx(s, existing)["code"], "bad_path")
        self.assertEqual(open(existing).read(), "keep")
        link = self.out(".l.vexel-audio-1.wav")
        os.symlink(victim, link)
        self.assertEqual(self.sfx(s, link)["code"], "bad_path")
        self.assertEqual(open(victim).read(), "keep")
        dangling = self.out(".d.vexel-audio-1.wav")
        os.symlink(self.out("target.wav"), dangling)
        self.assertEqual(self.sfx(s, dangling)["code"], "bad_path")
        self.assertFalse(os.path.exists(self.out("target.wav")))
        s.close()

    def test_write_wav_itself_is_create_new(self):
        sys.path.insert(0, os.path.dirname(HELPER))
        import numpy as np
        from vexel_audio import dsp
        from vexel_audio.errors import HelperError

        x = np.sin(np.arange(4800) / 10.0) * 0.5
        victim = self.out("victim.txt")
        open(victim, "w").write("keep")
        link = self.out(".l.vexel-audio-2.wav")
        os.symlink(victim, link)
        with self.assertRaises(HelperError):
            dsp.write_wav(link, x)
        with self.assertRaises(HelperError):
            dsp.write_wav(self.out("plain.wav"), x)  # not the Vexel temp name
        self.assertEqual(open(victim).read(), "keep")
        fresh = self.out(".f.vexel-audio-2.wav")
        dsp.write_wav(fresh, x)
        self.assertEqual(oct(os.stat(fresh).st_mode & 0o777), "0o600")
        with self.assertRaises(HelperError):
            dsp.write_wav(fresh, x)

    def test_non_temp_name_is_refused(self):
        s = Session()
        self.assertEqual(self.sfx(s, self.out("plain.wav"))["code"], "bad_path")
        s.close()

    def test_voice_module_forces_offline(self):
        env = dict(os.environ, HF_HUB_OFFLINE="0", TRANSFORMERS_OFFLINE="0")
        code = ("import sys; sys.path.insert(0, %r); import os; from vexel_audio import voice; "
                "print(os.environ['HF_HUB_OFFLINE'], os.environ['TRANSFORMERS_OFFLINE']); "
                "voice.force_offline(); os.environ['HF_HUB_OFFLINE']='0'; voice.force_offline(); "
                "print(os.environ['HF_HUB_OFFLINE'])") % os.path.dirname(HELPER)
        r = subprocess.run([sys.executable, "-c", code], capture_output=True, text=True, env=env)
        self.assertEqual(r.stdout.split(), ["1", "1", "1"], r.stderr)

    def test_selftest(self):
        r = subprocess.run([sys.executable, HELPER, "--selftest"], capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stderr)


if __name__ == "__main__":
    unittest.main()
