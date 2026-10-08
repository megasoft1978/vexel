# Corpus provenance and exclusions

## Included fixtures

The initial 24 fixtures are authored for Vexel in this work order. Their
`source_origin` and `source_revision` are `project-authored`, their source hash
is the exact local file hash, and their license is MPL-2.0. They are synthetic
coverage examples, not representative user documents. No external source file
was adapted. Creation timestamp, feature tags, path, content hash, license URL
and license hash are recorded per fixture in `bench/corpus/manifest.json`.

The project license text is `bench/corpus/licenses/MPL-2.0.txt`; its SHA-256 is
`3f3d9e0024b1921b067d6f7f88deb4a60cbe7a78e76c64e3f1d7fc3b779b9d04`.

## Pinned WPT license review

The reviewed upstream is Web Platform Tests revision
`da1355b3fc629f97f8194884ab113da1ba3c7990`. Its root license notice was
retrieved from
`https://raw.githubusercontent.com/web-platform-tests/wpt/da1355b3fc629f97f8194884ab113da1ba3c7990/LICENSE.md`
on 2026-10-08 and is retained verbatim as
`bench/corpus/licenses/WPT-LICENSE.md`. The file is 1,500 bytes and its
SHA-256 is
`5fac07febb0e2a97fb0d7b0def149ec08b642e1ba4b9c345283ab1cbd2af6570`.
This is the 3-Clause BSD notice for WPT at that exact revision. It remains
separate from the MPL-2.0 project fixtures. No WPT test file is included in
this first corpus, so no upstream test source hash or modification status is
claimed for a fixture.

## Exclusions

- The archived W3C SVG 1.1 test suite is excluded. Its rights page points to a
  W3C Document License and W3C Member Agreement; this work order establishes no
  per-file reuse grant. Rights statement:
  https://www.w3.org/Graphics/SVG/Test/Overview.html
- W3C test-suite material generally is excluded absent a specific applicable
  grant. Policy: https://www.w3.org/copyright/test-suites-licenses/
- WPT `svg/import/` is excluded, along with any file with unclear/conflicting
  per-file rights, external linked resources, executable script behavior,
  proprietary/user provenance, or GPL SVG-editor origin. None was copied.
- Natural/customer SVGs and large generated stress families are excluded.
  The small authored stress fixtures only exercise deterministic feature
  coverage and are not a benchmark-size tier.

The corpus is intended for development checks only. It does not establish W3C
conformance and no public performance claim should be made from this subset.
