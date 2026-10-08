# External source and fixture provenance

The M0 SVG development corpus is documented in
[`corpus/PROVENANCE.md`](corpus/PROVENANCE.md). All included SVG files are
project-authored and MPL-2.0. The pinned Web Platform Tests BSD license notice
is retained for the exact reviewed revision, but no WPT fixture data is
included. No third-party algorithms or renderer code were added.

WO-M0-02 uses the locally installed `rsvg-convert` 2.62.3 only to create a
one-time independent PNG test baseline from the project-authored MPL-2.0 M0
fixtures. Homebrew package metadata identifies librsvg as LGPL-2.1-or-later.
The renderer binary is not a build or test dependency; no renderer source was
fetched, inspected, copied, or vendored. The lock records linked component
versions, OS/architecture, source hashes, PNG hashes, and fixed 256x256 raster
dimensions. See `tests/oracle/metadata.json` and
`tests/golden/manifest.lock`. This baseline is not an SVG conformance claim or
a Vexel performance result.
