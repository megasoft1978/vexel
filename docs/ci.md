# Continuous integration

The workflow runs one Apple Silicon `macos-15` job for pushes, pull requests,
and manual dispatch. It checks out source with `actions/checkout` pinned to the
full v7.0.1 commit SHA, grants only `contents: read`, and runs `./dev verify`.
It has no secret references, dependency installation, cache, or other-OS job.

Locally, use:

```sh
./dev build
./dev test-fast
./dev verify
actionlint .github/workflows/ci.yml
python3 scripts/check_ci.py .github/workflows/ci.yml
```

The local fast test suite is expected to complete under 30 seconds. The hosted
workflow has not been observed in this checkout because no GitHub remote is
configured. GitHub runner labels and action revisions were checked against
their [runner documentation](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
and the [official checkout release](https://github.com/actions/checkout/releases/tag/v7.0.1).

## Sanitizer and fuzz workflow

The project supports Apple Silicon macOS 13+; both workflows use the Apple
Silicon `macos-15` runner. `.github/workflows/fuzz.yml` builds with the
ASan/UBSan profile and exercises only the CLI argument boundary. Pull requests
and manual runs use a 3-second window, while the weekday schedule uses 60
seconds. The workflow pins its checkout action by full commit SHA, grants
read-only repository permissions, has a bounded job timeout, and does not
upload artifacts or user data.

`python3 scripts/check_fuzz_workflow.py` checks the fuzz workflow's runner,
triggers, duration commands, action pins, secrets, and upload policy.
`actionlint .github/workflows/fuzz.yml` checks workflow syntax and action usage.

## Owner-step timing on hosted runners

`index-replay` fails if any owner-thread step exceeds 100 ms. Shared macOS runners
can jitter past that, so CI sets `VEXEL_OWNER_STEP_LIMIT_MS=500`. Local runs and
unset or invalid values keep the strict 100 ms gate.
