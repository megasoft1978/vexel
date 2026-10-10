# macOS packaging and release checks

Vexel's first distributable is an Apple Silicon `.app` bundle for macOS 13 and
later. The CMake project version in `CMakeLists.txt` is the source used for the
bundle's `CFBundleShortVersionString` and `CFBundleVersion`.

## Local app and disk image

Run the prerequisite check, then build the Release app and compressed disk
image:

```sh
./dev doctor
./dev bundle
./dev dmg
```

`./dev bundle` builds `build/mac-release/bin/svgworkbench` and assembles
`build/Vexel.app`. The app includes an XML `Info.plist`, a generated placeholder
icon, an SVG document association for `public.svg-image`, Retina support, the
graphics-design category, and a macOS 13 minimum. The PNG-backed ICNS icon is
drawn and assembled by the project's Python standard-library script; the build
fetches no artwork.

By default, the app is ad-hoc signed with the hardened runtime and verified
with `codesign`. `./dev dmg` stages the app beside an `/Applications` symlink,
creates `build/Vexel.dmg` using compressed UDZO format, and verifies the image.
Build outputs stay under the ignored `build/` directory.

## Developer ID signing and notarization

The packaging scripts accept these environment variables:

- `VEXEL_SIGN_IDENTITY`: a keychain signing identity. It defaults to `-` for
  ad-hoc signing. Use a Developer ID Application identity for distribution.
- `VEXEL_NOTARY_PROFILE`: an existing `notarytool` keychain profile. When set,
  `./dev dmg` submits the signed disk image, waits for notarization, staples the
  ticket, and validates the staple. It requires a non-ad-hoc signing identity.

For example, after the maintainer has installed the certificate and created a
notary profile in the local keychain:

```sh
VEXEL_SIGN_IDENTITY="Developer ID Application: Example" \
VEXEL_NOTARY_PROFILE="vexel-notary" ./dev dmg
```

The local flow does not require or create an Apple developer account. No
certificate or notarization secret is stored in the repository or CI workflow.

## Automated render and stress checks

`./dev e2e` builds the Release executable and renders a seeded matrix of 256
SVGs covering basic shapes, transforms, linear and radial gradients, local
`use`, and clipping. Every valid document is rendered twice to check PPM
structure, determinism, clean stderr, and per-render time. The seeded mutations
include byte flips, truncations, oversized numbers, deep nesting, invalid UTF-8,
and malformed XML. A failure report includes the seed and input path. Inputs,
frames, and JSON reports are written under `.cache/e2e/`.

`./dev soak [--minutes N]` defaults to five minutes and accepts 1–20 minutes.
The selected time is divided evenly between the existing XML and shape fuzz
campaigns built with AddressSanitizer and UndefinedBehaviorSanitizer. The
command also renders one fixture 10,000 times in a single Release process and
checks that RSS growth stays within 64 MiB and peak RSS stays below 512 MiB.
It runs the existing worker/snapshot test under ThreadSanitizer. The consolidated
JSON report is `.cache/soak/latest.json`; child reports and logs are kept under
`.cache/`.

## CI workflows

`.github/workflows/nightly.yml` runs the seeded e2e matrix, Debug sanitizer
suites, and bounded soak on macOS 15, then uploads the JSON campaign reports.
It supports the scheduled weekday run and manual dispatch.
`.github/workflows/release.yml` builds the bundle and DMG on a `v*` tag or
manual dispatch. It attaches the DMG to the
workflow run as an Actions artifact. The workflow does not publish a GitHub
Release asset or use signing secrets; those steps can be added after the
maintainer provisions Developer ID signing and notarization.

Both workflows use the repository's pinned-action, macOS-only CI policy:

```sh
python3 scripts/check_ci.py .github/workflows/nightly.yml
python3 scripts/check_ci.py .github/workflows/release.yml
```
