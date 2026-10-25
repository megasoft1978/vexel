"""Procedural chiptune loop (square/triangle voices and noise drums).

Ported from the Ember Call prototype. The melody is a seeded walk over the
chosen scale, so a key, tempo, mood, bar count and seed always give the same
loop. Notes that run past the end wrap around, so the file loops closed.
"""
import numpy as np

from . import SAMPLE_RATE as SR
from .dsp import highpass, normalize
from .errors import HelperError

NOTE = {"C": 0, "D": 2, "E": 4, "F": 5, "G": 7, "A": 9, "B": 11}
MAX_SECONDS = 120.0
MINOR = [0, 2, 3, 5, 7, 8, 10]
MAJOR = [0, 2, 4, 5, 7, 9, 11]
# mood -> (mode, chord roots as scale degrees (0-based), drums, arp level, bpm default)
MOODS = {
    "adventure": ("minor", [0, 5, 2, 6], True, 0.07, 100),
    "calm": ("major", [0, 4, 5, 3], False, 0.05, 80),
    "tense": ("minor", [0, 3, 0, 4], True, 0.06, 120),
    "bright": ("major", [0, 3, 4, 3], True, 0.07, 120),
}


def parse_key(key):
    key = str(key)
    if not key or key[0].upper() not in NOTE or len(key) > 2:
        raise HelperError("bad_params", "key must be a note name like A, C#, Bb")
    value = NOTE[key[0].upper()]
    if len(key) == 2:
        if key[1] == "#":
            value += 1
        elif key[1] == "b":
            value -= 1
        else:
            raise HelperError("bad_params", "key must be a note name like A, C#, Bb")
    return value % 12


def _hz(m):
    return 440.0 * 2 ** ((m - 69) / 12)


def generate(params):
    mood = params.get("mood", "adventure")
    if mood not in MOODS:
        raise HelperError("bad_params", "mood must be one of " + ", ".join(sorted(MOODS)))
    mode, prog, drums, arp_level, default_bpm = MOODS[mood]
    tonic = parse_key(params.get("key", "A"))
    bpm = params.get("tempo", default_bpm)
    bars = params.get("bars", 16)
    seed = params.get("seed", 3)
    for name, v, lo, hi in (("tempo", bpm, 50, 220), ("bars", bars, 2, 64), ("seed", seed, 0, 2**32 - 1)):
        if not isinstance(v, (int, float)) or isinstance(v, bool) or not lo <= v <= hi:
            raise HelperError("bad_params", f"{name} must be {lo}..{hi}")
    bars, seed = int(bars), int(seed)
    beat = 60.0 / bpm
    bar = 4 * beat
    if bars * bar > MAX_SECONDS:
        raise HelperError("bad_params", "loop would exceed 120 seconds; lower bars or raise tempo")
    n = int(round(bars * bar * SR))
    buf = np.zeros(n)
    rng = np.random.default_rng(seed)
    scale = MAJOR if mode == "major" else MINOR

    def add(x, start):
        i = int(round(start * SR))
        np.add.at(buf, (np.arange(len(x)) + i) % n, x)

    def tone(f, d, kind, vol, pw=0.5, atk=0.004, rel=0.05, dec=0.0):
        t = np.arange(int((d + rel) * SR)) / SR
        ph = (f * t) % 1
        w = np.where(ph < pw, 1.0, -1.0) if kind == "sq" else 4 * np.abs(ph - 0.5) - 1
        env = np.minimum(t / atk, 1) * np.where(t < d, 1, np.clip(1 - (t - d) / rel, 0, 1)) * np.exp(-dec * t)
        return w * env * vol

    def noise(d, vol, hpf=None, dec=30.0):
        t = np.arange(int(d * SR)) / SR
        x = rng.standard_normal(len(t))
        if hpf:
            x = highpass(x, hpf)
        return x * np.exp(-dec * t) * vol

    def kick(vol=0.9):
        t = np.arange(int(0.18 * SR)) / SR
        f = 45 + 120 * np.exp(-t * 30)
        return np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t * 14) * vol

    def degree(d, octave):
        return 12 * (4 + octave) + tonic + scale[d % 7] + 12 * (d // 7)

    step = beat / 2
    pos = 4
    for b in range(bars):
        t0 = b * bar
        root = prog[b % len(prog)]
        triad = [degree(root + k, 0) for k in (0, 2, 4)]
        bass = degree(root, -1)
        for k in range(8):
            add(tone(_hz(bass + (12 if k % 2 else 0)), step * 0.9, "tri", 0.30), t0 + k * step)
        arp = [triad[0] + 12, triad[1] + 12, triad[2] + 12, triad[1] + 12]
        lift = 12 if b >= (bars * 2) // 3 else 0
        for k in range(16):
            add(tone(_hz(arp[k % 4] + lift), step / 2 * 0.85, "sq", arp_level, pw=0.25, dec=6), t0 + k * step / 2)
        k = 0
        while k < 8:
            if rng.random() < 0.2:
                k += 1
                continue
            length = int(rng.choice([1, 1, 2, 2, 3]))
            length = min(length, 8 - k)
            if k == 0:
                pos = root + 4 + int(rng.integers(0, 3)) * 2  # start near a chord tone
            else:
                pos = int(np.clip(pos + int(rng.choice([-2, -1, -1, 1, 1, 2])), 2, 13))
            add(tone(_hz(degree(pos, 1)), length * step * 0.92, "sq", 0.13, atk=0.006, rel=0.06, dec=1.2), t0 + k * step)
            k += length
        if drums:
            for beat_i in range(4):
                if beat_i in (0, 2):
                    add(kick(), t0 + beat_i * beat)
                else:
                    add(noise(0.14, 0.22, hpf=1500, dec=22), t0 + beat_i * beat)
            for k in range(8):
                add(noise(0.04, 0.07 if k % 2 else 0.1, hpf=6500, dec=60), t0 + k * step)
    buf = buf - buf.mean()
    buf = np.tanh(buf * 1.1)
    return normalize(buf, 0.5)
