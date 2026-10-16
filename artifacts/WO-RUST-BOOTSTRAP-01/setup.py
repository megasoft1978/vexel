from pathlib import Path
import json

root = Path(__file__).resolve().parents[2]

def write(path, text):
    target = root / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(text)

names = ['base', 'geometry', 'xml', 'json', 'project', 'raster']
members = [f'rust/crates/vexel-{name}' for name in names] + ['rust/tools/vexel-dev']
write('Cargo.toml', '[workspace]\nresolver = "3"\nmembers = ' + json.dumps(members, indent=2) + '\n\n[workspace.package]\nversion = "0.1.0"\nedition = "2024"\nrust-version = "1.99"\nlicense = "MPL-2.0"\npublish = false\n')
write('rust-toolchain.toml', '[toolchain]\nchannel = "1.99.0"\nprofile = "minimal"\n')
write('.cargo/config.toml', '[build]\ntarget = "aarch64-apple-darwin"\ntarget-dir = "build/rust"\njobs = 2\n\n[env]\nMACOSX_DEPLOYMENT_TARGET = { value = "13.0", force = true }\n')
for name in names:
    manifest = f'[package]\nname = "vexel-{name}"\nversion.workspace = true\nedition.workspace = true\nrust-version.workspace = true\nlicense.workspace = true\npublish.workspace = true\n'
    if name != 'base':
        manifest += '\n[dependencies]\nvexel-base = { path = "../vexel-base" }\n'
    write(f'rust/crates/vexel-{name}/Cargo.toml', manifest)
    write(f'rust/crates/vexel-{name}/src/lib.rs', '#![forbid(unsafe_code)]\n//! Foundation contract; legacy behavior awaits its scoped port order.\n')
write('rust/crates/vexel-base/src/lib.rs', '''#![forbid(unsafe_code)]
//! Checked byte-span foundations for the owned Vexel Rust port.
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
compile_error!("Vexel initially supports Apple Silicon macOS 13+ only");
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseStatus { Invalid, Overflow, Limit, OutOfMemory }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span { pub start: usize, pub end: usize }
impl Span {
    pub fn from_offset_length(offset: usize, length: usize) -> Option<Self> {
        Some(Self { start: offset, end: offset.checked_add(length)? })
    }
    pub fn slice(self, source: &[u8]) -> Option<&[u8]> {
        source.get(self.start..self.end)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn span_valid_empty_bounds() {
        assert_eq!(Span::from_offset_length(1, 2).and_then(|s| s.slice(b"abcd")), Some(&b"bc"[..]));
        assert_eq!(Span::from_offset_length(4, 0).and_then(|s| s.slice(b"abcd")), Some(&b""[..]));
        assert!(Span { start: 3, end: 2 }.slice(b"abcd").is_none());
        assert!(Span { start: 0, end: 5 }.slice(b"abcd").is_none());
    }
    #[test] fn span_overflow() { assert!(Span::from_offset_length(usize::MAX, 1).is_none()); }
}
''')
write('rust/crates/vexel-geometry/src/lib.rs', '''#![forbid(unsafe_code)]
//! SVG affine contracts; functions await the geometry port order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point { pub x: f64, pub y: f64 }
/// SVG [a c e; b d f; 0 0 1]; compose maps right first.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine { pub a: f64, pub b: f64, pub c: f64, pub d: f64, pub e: f64, pub f: f64 }
''')
write('rust/tools/vexel-dev/Cargo.toml', '[package]\nname = "vexel-dev"\nversion.workspace = true\nedition.workspace = true\nrust-version.workspace = true\nlicense.workspace = true\npublish.workspace = true\n\n[dependencies]\nvexel-base = { path = "../../crates/vexel-base" }\n')
write('rust/tools/vexel-dev/src/main.rs', '''#![forbid(unsafe_code)]
use std::{env, path::PathBuf, process::{Command, ExitCode}};
fn root() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..") }
fn verify_toolchain() -> bool {
    match Command::new("rustc").args(["+stable", "--version"]).output() {
        Ok(result) => result.status.success() && result.stdout.starts_with(b"rustc 1.99.0 "),
        Err(_) => false,
    }
}
fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 1 {
        eprintln!("Usage: vexel-dev info|build|check"); return ExitCode::from(2);
    }
    if args[0] == "info" {
        println!("Vexel Rust foundation: Apple Silicon macOS13+, Rust1.99.0/2024, MPL2");
        println!("Workspace scaffolding; application/module parity pending. See docs/product/rust-port-goal-and-plan.md");
        return ExitCode::SUCCESS;
    }
    if args[0] != "build" && args[0] != "check" { return ExitCode::from(2); }
    if !verify_toolchain() { eprintln!("Exact installed Rust1.99.0 required; no install attempted"); return ExitCode::FAILURE; }
    let root = root();
    match Command::new("cargo").args(["+stable", &args[0], "--workspace", "--locked", "--offline"])
        .current_dir(&root).env("CARGO_HOME", root.join(".cache/cargo")).env("TMPDIR", root.join(".cache/tmp")).status() {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}
''')
write('rust/README.md', '''# Vexel Rust workspace

Foundation scaffolding only; desktop app and full legacy parity are pending.
Rust1.99.0/2024, MPL2, std-only/internal dependencies, AppleSilicon macOS13+ only.
Existing C checkout remains migration reference.

This host has Rust1.99.0 under stable, not a version-named installed alias.
Use cargo +stable after verifying rustc +stable --version is exactly1.99.0;
this avoids a toolchain download outside the project. Set CARGO_HOME/TMPDIR
to repo-local .cache/cargo and .cache/tmp; create them first. Cargo output stays
in ignored build/rust. Work orders name focused Rust tests; no broad UI runs.
The Rust developer CLI supports info/build/check and calls Rust Cargo only.
Legacy ./dev stays reference until its complete Rust command port is accepted.
''')
write('docs/adr/0023-rust-foundation-workspace.md', '''# ADR0023: Rust foundation workspace

Accepted2026-10-14 under the full-port owner direction. Rust1.99.0/2024/resolver3,
MPL2, std-only/internal dependencies, macOS13 AppleSilicon only. Base/geometry/
xml/json/project/raster and developer CLI members are scaffolding for ports.
Domain crates forbid unsafe; native FFI belongs to a later audited platform crate.
No external dependency adoption. Shared Span(start,end) uses checked arithmetic
and slices. Point/Affine are f64 SVG geometry, compose maps right first.
Coordinator owns manifests/lock/contracts. Build success is not product parity.

Exact compiler pin1.99.0 is recorded. The host stable alias resolves to it, so
checks use +stable after exact-version verification without global installs.
Build/cache/tmp output repo-local. Six disjoint foundation owners port modules
behind these interfaces, with focused checks and original fixture preservation.
''')
(root / '.cache/cargo').mkdir(parents=True, exist_ok=True)
(root / '.cache/tmp').mkdir(parents=True, exist_ok=True)
