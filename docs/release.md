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
`--out DIR`, `--icon FILE.icns`, `--bundle-id`, `--display-name` and
`--sign-identity`; `release` accepts the same flags; `package`, `dmg`, `sbom`
and `install-smoke` accept only `--out DIR`. Relative `--out` paths are relative to the current working
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
`io.github.megasoft1978.Vexel`, with macOS 13 as its minimum version. The supplied icon is
packaged as `AppIcon`; the app icon artwork and final bundle identity remain
owner decisions. Without a signing identity the command ad-hoc signs the
bundle and verifies that signature; with `--sign-identity` (or
`VEXEL_SIGN_IDENTITY`) it signs with Developer ID, see below.

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

### Developer ID signing and the `release` command

`bundle --sign-identity <name|sha1>` (or `VEXEL_SIGN_IDENTITY`) signs the
executable, then the bundle, with `codesign --force --options runtime
--timestamp` (inner first, no `--deep` when signing), then runs
`codesign --verify --strict --deep --verbose=2` and `spctl --assess --type
execute -vv`. The `spctl` result is informational and is expected to read
`rejected` / `Unnotarized Developer ID` until notarization. No entitlements are
used: the app is a native hardened-runtime binary with no JIT, unsigned-memory
or network-server needs. Add an entitlements file only if that changes.

`release` runs bundle, sign, DMG (signed with the same identity), then, only if
`VEXEL_NOTARY_PROFILE` names a keychain profile that `xcrun notarytool history`
accepts, `notarytool submit --wait`, `stapler staple` on the DMG and the app,
and `spctl`. It always writes `build/release/Vexel-<version>.dmg`, `.zip`
(`ditto -c -k --keepParent`) and `.sha256` (version from the workspace
`Cargo.toml`), and prints `signed, NOT notarized` when it stopped before
notarization. `package` re-creates those three files from an existing bundle
and DMG. `notarize --path X.app|X.dmg [--execute]` is the standalone step: an
`.app` is zipped first because notarytool does not accept bare bundles; dry run
prints the commands without running them.

Identity in use: `Developer ID Application: Antonietta Rizzo (JT3BY4G6KR)`,
SHA-1 `8EAF62F6CF51815385DC954E1E4380DB1730DB1B`. The tool never reads or
prints credentials.

### Notarization: one-time owner setup and finishing command

Notarization needs the owner's Apple ID app-specific password, which tooling
must never request. The owner runs once:

```sh
xcrun notarytool store-credentials vexel-notary --apple-id <apple id> \
  --team-id JT3BY4G6KR --password <app-specific-password>
```

Then the single finishing command (bundle, sign, DMG, notarize, staple, zip,
checksums):

```sh
VEXEL_SIGN_IDENTITY="Developer ID Application: Antonietta Rizzo (JT3BY4G6KR)" \
VEXEL_NOTARY_PROFILE=vexel-notary cargo run -p vexel-dev -- release
```

The first time, the keychain may ask to let `codesign` use the private key;
choose Always Allow.

### Current status

- Done and verified locally: Developer ID signed `Vexel.app` (Authority
  Developer ID Application, TeamIdentifier JT3BY4G6KR, hardened runtime flag
  `runtime`), signed DMG, zip and SHA-256 file. Gatekeeper reports
  `Unnotarized Developer ID` for them.
- Pending: notarization and stapling (owner must create the `vexel-notary`
  profile). Nothing has been uploaded anywhere.
- Still owner decisions or reviews: final app icon artwork, VoiceOver and IME
  review on the release build, hosted golden regeneration and review. A live
  GUI launch of the signed build has not been performed.

The current CI workflow is `.github/workflows/ci.yml`; it does not publish a
release or attach a distribution artifact. A manual CI packaging smoke is
configured but its execution on GitHub is not established by the workflow
definition. The earlier release work-order evidence recorded an ad-hoc packaging run; the
Developer ID signing above was added afterwards, notarization is still pending. No release readiness, full migration, or GUI verification claim
follows from those checks.

To check the repository's macOS-only CI policy for the active workflow:

```sh
cargo run -p vexel-dev --locked -- check-ci .github/workflows/ci.yml
```
