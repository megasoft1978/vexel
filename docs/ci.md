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
