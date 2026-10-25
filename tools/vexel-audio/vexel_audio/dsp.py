"""Small numpy-only DSP kit: FFT filters, envelopes, resampling, WAV output."""
import os
import wave

import numpy as np

from . import SAMPLE_RATE as SR
from .errors import HelperError


def times(seconds, sr=SR):
    return np.arange(int(seconds * sr)) / sr


def _mag(freqs, cutoff, order, kind):
    ratio = np.maximum(freqs, 1e-9) / cutoff
    low = 1.0 / np.sqrt(1.0 + ratio ** (2 * order))
    return low if kind == "low" else np.sqrt(np.clip(1.0 - low**2, 0.0, 1.0))


def lowpass(x, cutoff, sr=SR, order=2):
    spec = np.fft.rfft(x) * _mag(np.fft.rfftfreq(len(x), 1 / sr), cutoff, order, "low")
    return np.fft.irfft(spec, len(x))


def highpass(x, cutoff, sr=SR, order=2):
    spec = np.fft.rfft(x) * _mag(np.fft.rfftfreq(len(x), 1 / sr), cutoff, order, "high")
    return np.fft.irfft(spec, len(x))


def bandpass(x, lo, hi, sr=SR):
    return highpass(lowpass(x, hi, sr), lo, sr)


def soft_band(x, lo, hi, sr=SR):
    """Brick-ish band with soft edges (used for wind)."""
    spec = np.fft.rfft(x)
    f = np.fft.rfftfreq(len(x), 1 / sr)
    mask = np.clip(np.minimum((f - lo) / lo * 2, (hi - f) / hi * 2), 0, 1)
    return np.fft.irfft(spec * mask, len(x))


def reverb(x, rng, rt=3.0, wet=0.5, sr=SR):
    n = len(x)
    t = np.arange(n) / sr
    ir = lowpass(rng.standard_normal(n) * np.exp(-t * 6.9 / rt), 3000, sr)
    ir[0] = 0
    ir = ir / np.sqrt((ir**2).sum() + 1e-12)
    y = np.fft.irfft(np.fft.rfft(x) * np.fft.rfft(ir), n)
    return x * (1 - wet) + y * wet * 1.6


def resample(x, src, dst):
    if src == dst:
        return x
    n = int(round(len(x) * dst / src))
    spec = np.fft.rfft(x)
    out = np.zeros(n // 2 + 1, dtype=complex)
    keep = min(len(spec), len(out))
    out[:keep] = spec[:keep]
    return np.fft.irfft(out, n) * (n / len(x))


def normalize(x, peak=0.89):
    return x / (np.abs(x).max() + 1e-9) * peak


def fade_edges(x, ms=5.0, sr=SR):
    n = min(int(ms * sr / 1000), len(x) // 2)
    if n > 0:
        ramp = np.linspace(0.0, 1.0, n)
        x = x.copy()
        x[:n] *= ramp
        x[-n:] *= ramp[::-1]
    return x


def write_wav(path, x, sr=SR):
    """Writes mono PCM16; returns the duration in milliseconds."""
    if not np.all(np.isfinite(x)) or len(x) == 0:
        raise HelperError("generation_failed", "generator produced no usable samples")
    pcm = (np.clip(x, -1.0, 1.0) * 32767).astype("<i2")
    name = os.path.basename(path)
    if not name.startswith(".") or ".vexel-audio-" not in name:
        raise HelperError("bad_path", "outPath must be the temporary name chosen by Vexel")
    try:  # create-new only: never truncate, never follow a symlink
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    except FileExistsError:
        raise HelperError("bad_path", "outPath already exists; refusing to overwrite")
    except OSError as e:
        raise HelperError("bad_path", f"cannot create outPath: {e.strerror}")
    with os.fdopen(fd, "wb") as f, wave.open(f, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(sr)
        w.writeframes(pcm.tobytes())
    return int(len(pcm) * 1000 // sr)
