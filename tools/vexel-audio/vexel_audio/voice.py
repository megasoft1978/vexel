"""Speech with Kokoro-82M (4-bit, MLX) plus an optional effect chain.

Needs `mlx_audio` and the cached model. Nothing is downloaded unless the
request carries `consent: true`.
"""
import importlib.util
import os
import re

import numpy as np

from . import SAMPLE_RATE as SR
from .dsp import highpass, lowpass, resample
from .errors import HelperError

MODEL = "mlx-community/Kokoro-82M-4bit"
KOKORO_RATE = 24000
MAX_TEXT = 1000
VOICES = ["af_heart", "af_bella", "af_nova", "af_sky", "af_kore", "af_jessica",
          "am_onyx", "am_fenrir", "am_michael", "bf_emma", "bm_george"]
VOICE_RE = re.compile(r"^[abefhijpz][fm]_[a-z]{2,16}$")
PRESETS = {
    "pip": {"pitchSemitones": 4.9, "bitcrushBits": 10, "tremoloHz": 22, "tremoloDepth": 0.25,
            "ringModHz": 95, "ringModMix": 0.45},
    "whisper": {"highpassHz": 150, "lowpassHz": 7000},
    "soft": {"lowpassHz": 5000},
}
NUM = {"pitchSemitones": (-12, 12), "bitcrushBits": (4, 16), "tremoloHz": (0.1, 60),
       "tremoloDepth": (0, 1), "ringModHz": (10, 2000), "ringModMix": (0, 1),
       "highpassHz": (20, 2000), "lowpassHz": (500, 20000)}

INSTALL = ("Voice needs the Kokoro model. Install once: "
           "`python3 -m pip install mlx-audio`, then download the model with "
           "`python3 \"{helper}\" --download-model` (about 100 MB from huggingface.co/" + MODEL +
           "), or re-send this request with consent=true to let Vexel do that now.")


def cache_dir():
    hub = os.environ.get("HF_HUB_CACHE")
    if not hub:
        home = os.environ.get("HF_HOME") or os.path.join(os.path.expanduser("~"), ".cache", "huggingface")
        hub = os.path.join(home, "hub")
    return os.path.join(hub, "models--" + MODEL.replace("/", "--"))


def model_cached():
    return os.path.isdir(os.path.join(cache_dir(), "snapshots"))


def have_runtime():
    return importlib.util.find_spec("mlx_audio") is not None


def missing(helper):
    return HelperError("model_missing", INSTALL.format(helper=helper))


def force_offline():
    os.environ["HF_HUB_OFFLINE"] = "1"
    os.environ["TRANSFORMERS_OFFLINE"] = "1"


force_offline()  # the network is only ever reached by download() below


def download():
    """The one place that may reach the network (explicit action with consent)."""
    os.environ.pop("HF_HUB_OFFLINE", None)
    os.environ.pop("TRANSFORMERS_OFFLINE", None)
    try:
        from huggingface_hub import constants, snapshot_download  # installed with mlx-audio

        constants.HF_HUB_OFFLINE = False
        try:
            snapshot_download(MODEL)
        finally:
            constants.HF_HUB_OFFLINE = True
    finally:
        force_offline()


def phase_vocoder_stretch(x, rate, n_fft=1024, hop=256):
    """Time-stretches by `rate` (>1 = faster) keeping pitch."""
    win = np.hanning(n_fft)
    pad = np.pad(x, (n_fft, n_fft))
    frames = 1 + (len(pad) - n_fft) // hop
    spec = np.stack([np.fft.rfft(pad[i * hop:i * hop + n_fft] * win) for i in range(frames)])
    steps = np.arange(0, frames - 1, rate)
    omega = 2 * np.pi * hop * np.arange(spec.shape[1]) / n_fft
    phase = np.angle(spec[0])
    out = np.zeros((len(steps), spec.shape[1]), dtype=complex)
    for k, s in enumerate(steps):
        i = int(s)
        frac = s - i
        a, b = spec[i], spec[i + 1]
        mag = (1 - frac) * np.abs(a) + frac * np.abs(b)
        out[k] = mag * np.exp(1j * phase)
        delta = np.angle(b) - np.angle(a) - omega
        delta -= 2 * np.pi * np.round(delta / (2 * np.pi))
        phase = phase + omega + delta
    y = np.zeros(hop * (len(steps) - 1) + n_fft)
    norm = np.zeros_like(y)
    for k in range(len(steps)):
        seg = np.fft.irfft(out[k], n_fft) * win
        y[k * hop:k * hop + n_fft] += seg
        norm[k * hop:k * hop + n_fft] += win**2
    y /= np.maximum(norm, 1e-6)
    return y[n_fft:-n_fft] if len(y) > 2 * n_fft else y


def pitch_shift(x, semitones, sr):
    ratio = 2 ** (semitones / 12)
    stretched = phase_vocoder_stretch(x, 1 / ratio)  # longer by ratio, same pitch
    return resample(stretched, int(round(sr * ratio)), sr)  # play back faster: pitch up


def effects(x, fx, sr):
    if fx.get("pitchSemitones"):
        x = pitch_shift(x, fx["pitchSemitones"], sr)
    if fx.get("highpassHz"):
        x = highpass(x, fx["highpassHz"], sr)
    if fx.get("lowpassHz"):
        x = lowpass(x, fx["lowpassHz"], sr)
    if fx.get("bitcrushBits"):
        levels = 2 ** (int(fx["bitcrushBits"]) - 1)
        x = np.round(x * levels) / levels
    if fx.get("tremoloHz"):
        depth = fx.get("tremoloDepth", 0.25)
        t = np.arange(len(x)) / sr
        x = x * (1 - depth / 2 + depth / 2 * np.sin(2 * np.pi * fx["tremoloHz"] * t))
    if fx.get("ringModHz"):
        mix = fx.get("ringModMix", 0.45)
        t = np.arange(len(x)) / sr
        x = x + mix * x * np.sin(2 * np.pi * fx["ringModHz"] * t)
    return x


def chain(params):
    fx = {}
    name = params.get("effect")
    if name is not None:
        if name not in PRESETS:
            raise HelperError("bad_params", "effect must be one of " + ", ".join(sorted(PRESETS)))
        fx.update(PRESETS[name])
    for key, (lo, hi) in NUM.items():
        if key in params:
            v = params[key]
            if not isinstance(v, (int, float)) or isinstance(v, bool) or not lo <= v <= hi:
                raise HelperError("bad_params", f"{key} must be {lo}..{hi}")
            fx[key] = v
    return fx


def validate(params):
    text = params.get("text")
    if not isinstance(text, str) or not text.strip() or len(text) > MAX_TEXT:
        raise HelperError("bad_params", f"text must be 1..{MAX_TEXT} characters")
    voice = params.get("voice", "af_heart")
    if not isinstance(voice, str) or not VOICE_RE.match(voice):
        raise HelperError("bad_params", "voice must be a Kokoro id such as " + ", ".join(VOICES[:4]))
    speed = params.get("speed", 1.0)
    if not isinstance(speed, (int, float)) or isinstance(speed, bool) or not 0.5 <= speed <= 2.0:
        raise HelperError("bad_params", "speed must be 0.5..2.0")
    return text, voice, float(speed), chain(params)


def generate(params, helper, consent=False):
    text, voice, speed, fx = validate(params)  # validate before touching models
    if not have_runtime() or (not model_cached() and not consent):
        raise missing(helper)
    if not model_cached():
        try:
            download()
        except Exception as exc:  # network or disk failure
            raise HelperError("model_missing", f"model download failed ({exc}). " + INSTALL.format(helper=helper))
    force_offline()
    from mlx_audio.tts.utils import load_model

    model = load_model(MODEL)
    parts = [np.array(r.audio).reshape(-1)
             for r in model.generate(text=text, voice=voice, speed=speed, lang_code=voice[0])]
    if not parts:
        raise HelperError("generation_failed", "the model produced no audio")
    x = np.concatenate([np.zeros(int(0.05 * KOKORO_RATE))] + parts + [np.zeros(int(0.12 * KOKORO_RATE))])
    x = effects(x.astype(np.float64), fx, KOKORO_RATE)
    x = resample(x, KOKORO_RATE, SR)
    peak = np.abs(x).max()
    return x / peak * 0.89 if peak > 0.89 else x
