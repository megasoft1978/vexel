# Audio generation helper

Vexel generates voices, sound effects and music through an optional helper
process, `tools/vexel-audio` (Python). The app stays small, models can be
swapped, nothing is fetched at render time, and a model download only happens
after explicit consent. Timeline audio tracks, the export mux and
`audio_envelope` need no helper.

## Install

- Sound effects and music: `python3` with `numpy` (`python3 -m pip install numpy`).
  No model. Check with `tools/vexel-audio/vexel-audio --selftest`.
- Voice: additionally `python3 -m pip install mlx-audio` (Apple Silicon) and the
  Kokoro-82M 4-bit model (about 100 MB, `mlx-community/Kokoro-82M-4bit`).

## Where Vexel looks for the helper

1. `VEXEL_AUDIO_HELPER` (path to the `vexel-audio` script).
2. `vexel-audio` next to the app binary, or `../Resources/vexel-audio` in an app bundle.
3. Debug builds only: `tools/vexel-audio/vexel-audio` of the source checkout.

There is no search of parent directories. The file must be a regular file (not a
symlink), owned by you or root, and not writable by group or others. The helper
runs without a shell, with only `PATH`, `HOME`, `VEXEL_*`, `HF_*` and `MLX*`
environment variables, and exits when Vexel closes its pipe.

## Model cache and consent

The model lives in the Hugging Face cache (`~/.cache/huggingface/hub`, or
`HF_HUB_CACHE` / `HF_HOME`). Voice requests never download: without the model they
fail with `model_missing` and these steps. Download once yourself with
`tools/vexel-audio/vexel-audio --download-model`, or have the agent ask you and
then call `audio_generate` with `consent: true`.

## Voices

Kokoro ids such as `af_heart` (default, bright), `af_bella`, `af_nova`, `af_sky`,
`af_kore`, `af_jessica`, `am_onyx` (deep), `am_fenrir`, `am_michael`, `bf_emma`,
`bm_george`. `speed` is 0.5 to 2. Effects: `effect` presets `pip` (pitch +4.9
semitones, 10-bit crush, 22 Hz tremolo, ring-mod blend), `whisper`, `soft`, or
explicit `pitchSemitones`, `bitcrushBits`, `tremoloHz`, `tremoloDepth`,
`ringModHz`, `ringModMix`, `highpassHz`, `lowpassHz`.

## Protocol

One JSON object per line on stdin, one reply per line on stdout (replies over 16
KiB are rejected):

```
{"id":1,"kind":"sfx","params":{"preset":"chime","seed":7},"outPath":"/abs/x.wav","consent":false}
{"id":1,"ok":true,"path":"/abs/x.wav","durationMs":1000}
{"id":1,"ok":false,"code":"model_missing","error":"..."}
```

Output is mono PCM16 WAV at 48 kHz. Error codes: `bad_params`, `bad_path`,
`bad_kind`, `bad_request`, `model_missing`, `dependency_missing`, `too_long`,
`generation_failed`. sfx and music are deterministic for the same params.

## Tests

`python3 -m unittest tools/vexel-audio/test_helper.py`; Rust:
`cargo test -p vexel-app --test mcp_audio_gen` (fake helper; the real-helper test
skips without numpy). The real Kokoro run is `-- --ignored real_kokoro_voice`.
