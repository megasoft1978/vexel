# vexel-svg-parity

Standalone, offline component harness for accepted SVG paint caching and clip definition planning. From the repository root:

```sh
CARGO_HOME="$PWD/.cache/cargo-home" CARGO_TARGET_DIR="$PWD/.cache/target-parity" cargo +stable check --manifest-path rust/tools/vexel-svg-parity/Cargo.toml --locked --offline
CARGO_HOME="$PWD/.cache/cargo-home" CARGO_TARGET_DIR="$PWD/.cache/target-parity" cargo +stable test --manifest-path rust/tools/vexel-svg-parity/Cargo.toml --locked --offline
cargo +stable fmt --manifest-path rust/tools/vexel-svg-parity/Cargo.toml -- --check
```

The tested harness package is `vexel-svg-parity` **0.1.0**. Its direct production dependencies are the repository's **0.1.0** packages at these relative paths from this manifest:

| Package | Path |
| --- | --- |
| `vexel-base` | `../../crates/vexel-base` |
| `vexel-geometry` | `../../crates/vexel-geometry` |
| `vexel-scene` | `../../crates/vexel-scene` |
| `vexel-xml` | `../../crates/vexel-xml` |
| `vexel-svg-css` | `../../crates/vexel-svg-css` |
| `vexel-svg-reference` | `../../crates/vexel-svg-reference` |
| `vexel-svg-paint` | `../../crates/vexel-svg-paint` |
| `vexel-svg-definitions` | `../../crates/vexel-svg-definitions` |
| `vexel-svg-animation` | `../../crates/vexel-svg-animation` |
| `vexel-svg-path` | `../../crates/vexel-svg-path` |
| `vexel-svg-values` | `../../crates/vexel-svg-values` |

The locked transitive dependency graph also uses local path crates. There are no external or dev dependencies. This is component evidence only, not decoder or product acceptance.
