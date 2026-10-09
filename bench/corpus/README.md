# SVG development corpus

This is a small, deterministic development corpus for early Vexel SVG tooling.
It is not a representative sample of user documents and is not a conformance
suite or a source of public performance claims. It contains 24 original
project-authored fixtures spanning shapes, paths, metadata/namespaces, styling,
text, a data-URI embedded image, transforms, and stress-oriented small inputs.

All fixture records and their hashes are in `manifest.json`; its exact digest is
in `manifest.sha256`. The project fixtures are MPL-2.0 and are covered by
`licenses/MPL-2.0.txt`. The WPT BSD license notice is retained at
`licenses/WPT-LICENSE.md` as the pinned source review record; no WPT fixture is
included in this initial corpus. See `docs/corpus/PROVENANCE.md` for decisions
and exclusions.

Run `python3 scripts/fetch_corpus.py --verify-only` for offline verification.
`--fetch` is a separate developer command and only obtains records whose
manifest source is on the pinned WPT raw-file allowlist. Opening SVG documents
in the application never invokes this tool or loads external resources.
