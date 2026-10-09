# Contributing

vexel is early-stage and maintained by one person; issues and discussion are
welcome, and please open an issue before starting a large change.

## Ground rules

- Apple Silicon macOS 13+ only. Do not add other-OS code, shims or CI jobs.
- C17, MPL-2.0, minimal dependencies. New dependencies need an ADR and pinned
  provenance in `docs/provenance.md`.
- Follow `docs/CODING_STANDARD.md`: explicit ownership, pointer+length slices,
  checked size arithmetic, bounded input, no VLAs, no unsafe string functions.
- Preserve unknown SVG/XML source. Do not fetch external SVG resources.
- Never read, copy or port code from GPL SVG editors; use specifications only.
- Keep source files under 800 lines and add tests with every change.
- Do not weaken golden images, oracle baselines or pixel thresholds.

## Before opening a pull request

```sh
./dev verify
./dev asan && ./dev ubsan
```

By contributing you agree that your contribution is licensed under MPL-2.0.
