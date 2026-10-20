# macOS packaging and release checks

Vexel's current release tooling is the Rust `vexel-dev` command, supported on
Apple Silicon macOS 13 or later. The `build`, `check`, and `bundle` commands
require the exact installed Rust 1.99.0 toolchain and do not install it. CI
installs Rust 1.99.0 and runs on `macos-15`.

## Development and CI checks

Run these from the repository root. The workspace checks use the committed
lockfile; CI performs them on every push, pull request, and manual dispatch:

```sh
python3 scripts/check_rust_fmt.py
cargo check --workspace --locked
cargo test --workspace --locked -- --test-threads=1
```

CI also attempts Clippy when available, as an informational step with
`continue-on-error`; it is not a release gate. The bundle, DMG, SBOM, and
install smoke run only on `workflow_dispatch`:

```sh
cargo run -p vexel-dev --locked -- bundle
cargo run -p vexel-dev --locked -- dmg
cargo run -p vexel-dev --locked -- sbom
cargo run -p vexel-dev --locked -- install-smoke
```

The manual CI job first fetches locked dependencies. For a local offline bundle
build, dependencies must already be available to Cargo. `vexel-dev` uses
`.cache/cargo` and `build/rust` under the repository by default, with
`CARGO_HOME` and `CARGO_TARGET_DIR` environment variables taking precedence.
Its commands use Cargo's `+stable` toolchain selector and require that selected
toolchain to report Rust 1.99.0.

## Local bundle and packaging sequence

`vexel-dev` is run through Cargo from the repository root. Its supported
commands are `info`, `build`, `check`, `check-ci WORKFLOW.yml`, `bundle`,
`dmg`, `sbom`, and `install-smoke`. `info`, `build`, `check`, and `check-ci`
take no flags; `check-ci` requires exactly one workflow path. `bundle` accepts
`--out DIR` and `--icon FILE.icns`; the other three packaging commands accept
only `--out DIR`. Relative `--out` paths are relative to the current working
directory. The default output directory is `build/release`.

The bundle step requires `resources/AppIcon.icns` (or a supplied `.icns` file),
and builds the Release Apple Silicon app before assembling it:

```sh
cargo run -p vexel-dev --locked -- bundle
```

The result is `build/release/Vexel.app`. The executable is at
`Contents/MacOS/vexel`; the bundle includes `Info.plist`, the English
localization strings, and `Contents/Resources/AppIcon.icns`. The plist uses
the workspace package version and the placeholder identifier
`app.vexel.Vexel`, with macOS 13 as its minimum version. The supplied icon is
packaged as `AppIcon`; the app icon artwork and final bundle identity remain
owner decisions. The command ad-hoc signs the bundle and verifies that
signature.

After a bundle exists, create and verify the compressed disk image, then create
the CycloneDX SBOM for the bundled executable:

```sh
cargo run -p vexel-dev --locked -- dmg
cargo run -p vexel-dev --locked -- sbom
```

These produce `build/release/Vexel.dmg` and
`build/release/sbom.cyclonedx.json`. The DMG contains `Vexel.app` and an
`Applications` symlink. `dmg` verifies the image after creation. `sbom` records
the executable's SHA-256 and linked libraries. Both commands require the bundle
to exist first.

The final smoke step mounts the DMG read-only, copies the app into a temporary
Applications directory, and checks the plist, signature, arm64 architecture,
and executable permission. It detaches the mounted image and does **not** launch
the GUI. It requires the DMG from the previous step:

```sh
cargo run -p vexel-dev --locked -- install-smoke
```

The smoke reports `install-smoke: pass` on success. This is a packaging check,
not evidence of a clean-install GUI launch or product release readiness.

## Signing, notarization, and release evidence

### Owner-gated release checklist

`bundle` accepts `--bundle-id` and `--display-name`, with `VEXEL_BUNDLE_ID`
and `VEXEL_DISPLAY_NAME` environment defaults. Without either override it
retains the placeholder identifier `app.vexel.Vexel` and display name `Vexel`.
`vexel-dev notarize --path build/release/Vexel.app` validates the path and
prints the signing, notary submission, and stapling commands. It is a dry run
unless `--execute` is supplied; execution additionally requires
`VEXEL_SIGN_IDENTITY` and `VEXEL_NOTARY_PROFILE`. The tool does not inspect or
print credential contents.

Before distribution, the owner must supply or approve:

- A Developer ID Application certificate and its signing identity.
- A configured notarytool keychain profile.
- The final bundle identifier and production display name.
- Final app icon artwork.
- VoiceOver and IME review on the release build.
- Hosted golden regeneration and review.

Vexel code supports local bundle creation, configurable plist identity and
name, and explicit signing/notarization command execution. It cannot supply
the certificate, credentials/profile, product identity decisions, icon
artwork, accessibility/input-method approval, or hosted golden results.

The current Rust packaging command always signs with the ad-hoc identity `-`
and verifies that signature. It has no CLI or environment-variable option for
Developer ID signing or notarization. Those actions remain owner-gated; do not
use the ad-hoc bundle as evidence of Developer ID distribution or notarization.
The placeholder bundle identifier and final artwork also need owner decisions.

The current CI workflow is `.github/workflows/ci.yml`; it does not publish a
release or attach a distribution artifact. A manual CI packaging smoke is
configured but its execution on GitHub is not established by the workflow
definition. The release work-order evidence records a local packaging run, but
explicitly says Developer ID signing, notarization, and live GUI launch were
not performed. No release readiness, full migration, or GUI verification claim
follows from those checks.

To check the repository's macOS-only CI policy for the active workflow:

```sh
cargo run -p vexel-dev --locked -- check-ci .github/workflows/ci.yml
```
