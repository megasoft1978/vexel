#![forbid(unsafe_code)]
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
compile_error!("Vexel initially supports Apple Silicon macOS 13+ only");
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
    if std::fs::create_dir_all(root.join(".cache/cargo")).is_err()
        || std::fs::create_dir_all(root.join(".cache/tmp")).is_err() {
        eprintln!("Repository-local build/cache directories could not be created");
        return ExitCode::FAILURE;
    }
    match Command::new("cargo").args(["+stable", &args[0], "--workspace", "--locked", "--offline"])
        .current_dir(&root).env("CARGO_HOME", root.join(".cache/cargo"))
        .env("CARGO_TARGET_DIR", root.join("build/rust"))
        .env("TMPDIR", root.join(".cache/tmp")).status() {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        _ => ExitCode::FAILURE,
    }
}
