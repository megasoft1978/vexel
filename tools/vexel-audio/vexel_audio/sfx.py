"""Procedural sound effects (no models). Deterministic for a given seed."""
import numpy as np

from . import SAMPLE_RATE as SR
from .dsp import (bandpass, highpass, lowpass, normalize, reverb, soft_band,
                  times, fade_edges)
from .errors import HelperError

MAX_SECONDS = 15.0
# preset -> (default seconds, adjustable)
PRESETS = {
    "whoosh": (1.5, True),
    "hum": (12.0, True),
    "launch": (6.0, False),
    "chime": (1.0, False),
    "siren": (4.0, True),
    "wind": (10.0, True),
    "comet": (3.0, False),
    "hoverbike": (4.0, False),
}

LOOPING = {"hum", "wind"}


def _hum(rng, d):
    t = times(d)

    def vib(f, depth, cycles):
        return np.sin(2 * np.pi * f * t + depth * np.sin(2 * np.pi * cycles / d * t))

    x = vib(110, 1.5, 3) + vib(165, 1.2, 4) * 0.55 + vib(220, 1.0, 5) * 0.4 + vib(220, 0.8, 7) * 0.08
    x *= 1 + 0.18 * np.sin(2 * np.pi * 2 / d * t) + 0.1 * np.sin(2 * np.pi * 9 / d * t)
    return normalize(reverb(x, rng, rt=3.5, wet=0.45), 0.7)


def _wind(rng, d):
    t = times(d)
    n = soft_band(rng.standard_normal(len(t)), 150, 1800)
    g = 1 + 0.6 * np.sin(2 * np.pi / d * t + 1) + 0.4 * np.sin(6 * np.pi / d * t + 0.3) + 0.2 * np.sin(14 * np.pi / d * t)
    return normalize(n * np.clip(g, 0.2, None), 0.7)


def _launch(rng, d):
    t = times(d)
    n = len(t)
    env = np.clip(t / 0.5, 0, 1) * np.clip((d - t) / 2.0, 0, 1) ** 1.5
    rum = lowpass(rng.standard_normal(n), 180) * 4 * env
    rum += np.sin(2 * np.pi * (45 + 10 * np.sin(2 * np.pi * 7 * t)) * t) * 0.5 * env
    ig = rng.standard_normal(n) * (t < 1.0) * np.exp(-t * 3) * (rng.random(n) < 0.15)
    ig = highpass(ig, 500) * 0.9
    fc = 300 * np.exp(np.log(7000 / 300) * np.clip(t / 4.5, 0, 1))
    ph = np.cumsum(fc) / SR
    sw = lowpass(rng.standard_normal(n) * np.sin(2 * np.pi * ph), 1500) * 4
    sw *= np.clip((t - 0.6) / 3.0, 0, 1) * np.clip((d - t) / 1.2, 0, 1)
    return normalize(rum * 1.2 + ig + sw * 0.8, 0.89)


def _hoverbike(rng, d):
    t = times(d)
    n = len(t)
    tc = d / 2
    dist = np.sqrt(1 + ((t - tc) * 2.2) ** 2)
    v = (t - tc) * 2.2 / dist
    ph = np.cumsum(95 / (1 + 0.25 * v)) / SR
    eng = np.sign(np.sin(2 * np.pi * ph)) * 0.3 + np.sin(4 * np.pi * ph) * 0.3 + np.sin(2 * np.pi * ph * 3.01) * 0.2
    eng = lowpass(eng, 1800)
    whine = np.sin(2 * np.pi * np.cumsum(560 / (1 + 0.25 * v)) / SR) * 0.12
    ns = bandpass(rng.standard_normal(n), 400, 3000) * 0.25
    out = (eng + whine + ns) / dist**1.5 * np.clip(t / 0.1, 0, 1) * np.clip((d - t) / 0.2, 0, 1)
    return normalize(out, 0.85)


def _siren(rng, d):
    t = times(d)
    seg = (t * 2).astype(int) % 2
    f = np.where(seg == 0, 720.0, 960.0)
    f = lowpass(np.tile(f, 3), 6)[len(f):2 * len(f)]
    ph = np.cumsum(f) / SR
    s = lowpass(np.sign(np.sin(2 * np.pi * ph)) * 0.5 + np.sin(2 * np.pi * ph), 3500)
    s *= np.clip(t / 0.01, 0, 1) * np.clip((d - t) / 0.01, 0, 1)
    return normalize(s, 0.45)


def _comet(rng, d):
    t = times(d)
    n = len(t)
    fc = 6000 * np.exp(np.log(400 / 6000) * np.clip(t / (d * 0.87), 0, 1))
    ph = np.cumsum(fc) / SR
    x = lowpass(rng.standard_normal(n) * np.sin(2 * np.pi * ph), 2500) * 3
    env = np.exp(-(((t - d / 3) / (d * 0.18)) ** 2)) * np.clip((d - t) / 0.4, 0, 1)
    sp = highpass(rng.standard_normal(n), 5000) * 0.1 * np.exp(-(((t - d * 0.37) / (d * 0.2)) ** 2))
    tone = np.sin(2 * np.pi * np.cumsum(900 * np.exp(-t * 1.2)) / SR) * 0.15 * env
    return normalize(x * env + sp + tone, 0.85)


def _whoosh(rng, d):
    t = times(d)
    n = len(t)
    fc = 5000 * np.exp(np.log(300 / 5000) * np.clip(t / d, 0, 1))
    ph = np.cumsum(fc) / SR
    x = lowpass(rng.standard_normal(n) * np.sin(2 * np.pi * ph), 3000) * 3
    env = np.sin(np.pi * np.clip(t / d, 0, 1)) ** 2
    return normalize(x * env, 0.85)


def _chime(rng, d):
    t = times(d)
    x = np.zeros(len(t))
    for i, f in enumerate([1760, 2217, 2637, 3520, 4186]):
        st = i * 0.07
        tt = np.clip(t - st, 0, None)
        x += np.sin(2 * np.pi * f * t) * np.exp(-tt * 6) * (t >= st) / (1 + i * 0.15)
    return normalize(x * np.clip((d - t) / 0.05, 0, 1), 0.7)


_GEN = {"whoosh": _whoosh, "hum": _hum, "launch": _launch, "chime": _chime,
        "siren": _siren, "wind": _wind, "comet": _comet, "hoverbike": _hoverbike}


def generate(params):
    preset = params.get("preset")
    if preset not in PRESETS:
        raise HelperError("bad_params", "sfx preset must be one of " + ", ".join(sorted(PRESETS)))
    default, adjustable = PRESETS[preset]
    seconds = default
    if "durationMs" in params:
        ms = params["durationMs"]
        if not isinstance(ms, (int, float)) or isinstance(ms, bool) or not 200 <= ms <= MAX_SECONDS * 1000:
            raise HelperError("bad_params", "durationMs must be 200..15000")
        if adjustable:
            seconds = ms / 1000.0
    seed = params.get("seed", 7)
    if not isinstance(seed, int) or isinstance(seed, bool) or not 0 <= seed < 2**32:
        raise HelperError("bad_params", "seed must be an integer 0..4294967295")
    rng = np.random.default_rng(seed)
    out = _GEN[preset](rng, seconds)
    return out if preset in LOOPING else fade_edges(out, 3.0)
