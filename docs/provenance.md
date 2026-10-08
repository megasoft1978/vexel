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

WO-M0-03 adds only process-invocation measurement infrastructure. The adapter
boundary invokes configured released binaries as argv arrays and does not
inspect, fetch, install, or copy renderer source. No third-party algorithm or
renderer dependency was added. The offline test renderer is project-authored
Python test code and emits a deterministic PNG; it is not a competitor result.
Adapter processes use macOS's built-in `sandbox-exec` with process forks and
network access denied. Pinned SVG fixtures are parsed before use; external
resource references, stylesheet imports, and XML external declarations are
rejected. Configured render commands must receive the input, output, width, and
height placeholders. Each trial can write only its designated `.cache` PNG,
with a 2 MiB file-size limit; configuration is capped at eight adapters. No
package or runtime dependency was added; adapters that require subprocess
helpers or external resources report failed trials explicitly.
