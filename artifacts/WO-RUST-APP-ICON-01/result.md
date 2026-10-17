# WO-RUST-APP-ICON-01 result

## Design
- First-party icon drawn by `scripts/make_icon.py` (Pillow, dev-only, run by hand; not a product dependency).
- Rounded square: 10% transparent margin per side, corner radius 22.37% of the 824 px shape.
- Fill: diagonal gradient, cobalt (38,92,226) top-left to ink (14,20,36) bottom-right.
- Mark: white Bezier V with square anchors (white, cobalt outline), circular handle dots on accent lines (dark accent #5EA3FA from docs/ui/design-direction-2026-10.md), and one accent keyframe diamond on the left stroke with a cobalt knockout ring.
- Iconset sizes 16 to 1024 px (plus @2x) are drawn at 4x supersampling and Lanczos-reduced. At 64 px and below, handles and anchors are dropped and the stroke is heavier.

## Sizes and artifacts
- `resources/AppIcon.icns` (443 KB), built with `iconutil -c icns`.
- Round-trip with `iconutil -c iconset` gives icon_16x16 through icon_512x512@2x at the expected pixel sizes (16, 32, 32, 64, 128, 256, 256, 512, 512, 1024), confirmed with `sips -g pixelWidth`.
- Preview: `docs/archive/live/2026-10-17-app-icon-1024.png`.
- The .iconset PNGs are written to `build/icon/AppIcon.iconset` (git-ignored) and are not committed.

## Commands
- `python3 scripts/make_icon.py`: writes the iconset and `resources/AppIcon.icns`.
- `cargo run -q --offline --locked -p vexel-dev -- bundle --out /private/tmp/vexel-app-icon-01`: release build, copies binary, copies icon to `Contents/Resources/AppIcon.icns`, writes Info.plist with `CFBundleIconFile=AppIcon`, ad-hoc codesign, `codesign --verify --strict`. Exit 0.
- `plutil -lint Vexel.app/Contents/Info.plist`: OK.
- `codesign --verify --strict Vexel.app`: passes.
- `vexel-dev bundle --icon <missing>.icns`: exits 1 before building ("app icon ... not found; generate it with `python3 scripts/make_icon.py`").
- `vexel-dev dmg --out /private/tmp/vexel-app-icon-01`: hdiutil verify VALID.
- `vexel-dev install-smoke --out /private/tmp/vexel-app-icon-01`: pass.

## Code changes (rust/tools/vexel-dev)
- bundle.rs: `ICON_SOURCE = resources/AppIcon.icns` is the default. `--icon` overrides the source. The destination is always `Contents/Resources/AppIcon.icns`. `require_icon` fails clearly before any write or build. `assemble` takes a required icon source.
- plist.rs: `CFBundleIconFile` is always rendered as `AppIcon`; the Option parameter is removed.
- main.rs: resolves and checks the icon before the release build.
- Tests: plan places the icon at the fixed name; a missing icon fails before anything is written; the copy lands at the fixed name; plist key set and value are checked.

## Test results
- `cargo test --offline --locked -p vexel-dev`: 25 passed, no warnings.
- `cargo fmt -p vexel-dev -- --check`: clean. `python3 scripts/check_rust_fmt.py`: pass (18 clean-listed crates).

## Owner-gated (not done here)
- Bundle identifier `app.vexel.Vexel` remains a placeholder; not changed.
- Signing is ad hoc only. Developer ID identity and notarization are owner-gated and not used.
- The icon is a first-party draft; final brand review is the owner's call.
