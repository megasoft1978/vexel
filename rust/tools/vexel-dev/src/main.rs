#![forbid(unsafe_code)]
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
compile_error!("Vexel initially supports Apple Silicon macOS 13+ only");
mod args;
mod bundle;
mod ci_policy;
mod plist;
mod release;
mod runner;

use args::{Command, USAGE};
use runner::{Invocation, Runner, SystemRunner, run_checked};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

fn root() -> PathBuf {
    let raw = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    fs::canonicalize(&raw).unwrap_or(raw)
}

fn verify_toolchain(runner: &mut dyn Runner) -> bool {
    let probe = Invocation::new("rustc").args(["+stable", "--version"]);
    match runner.run(&probe) {
        Ok(result) => result.success && result.stdout.starts_with("rustc 1.99.0 "),
        Err(_) => false,
    }
}

fn cargo_env(root: &Path) -> bundle::CargoEnv {
    bundle::cargo_env(
        root,
        env::var("CARGO_HOME").ok().as_deref(),
        env::var("CARGO_TARGET_DIR").ok().as_deref(),
    )
}

/// Resolves `--out` (relative to the current directory) or `build/release`.
fn out_dir(root: &Path, out: Option<PathBuf>) -> Result<PathBuf, String> {
    let dir = match out {
        Some(path) if path.is_absolute() => path,
        Some(path) => env::current_dir()
            .map_err(|e| format!("current directory: {e}"))?
            .join(path),
        None => root.join("build/release"),
    };
    fs::create_dir_all(&dir).map_err(|e| format!("create `{}`: {e}", dir.display()))?;
    Ok(dir)
}

fn prepare_cache(cargo: &bundle::CargoEnv) -> Result<(), String> {
    for dir in [&cargo.home, &cargo.tmp] {
        fs::create_dir_all(dir).map_err(|_| {
            "Repository-local build/cache directories could not be created".to_string()
        })?;
    }
    Ok(())
}

fn run(command: Command, runner: &mut dyn Runner) -> Result<(), String> {
    let root = root();
    match command {
        Command::Info => {
            println!("Vexel Rust foundation: Apple Silicon macOS13+, Rust1.99.0/2024, MPL2");
            println!(
                "Workspace scaffolding; application/module parity pending. See docs/product/rust-port-goal-and-plan.md"
            );
            Ok(())
        }
        Command::CheckCi { workflow } => check_ci_policy(&workflow),
        Command::Build | Command::Check => {
            let verb = if command == Command::Build {
                "build"
            } else {
                "check"
            };
            toolchain_and_cache(runner, &root)?;
            let invocation = bundle::cargo_invocation(
                &[verb, "--workspace", "--locked", "--offline"],
                &root,
                &cargo_env(&root),
            );
            run_checked(runner, &invocation).map(|_| ())
        }
        Command::Bundle {
            out,
            icon,
            bundle_id,
            display_name,
        } => {
            let cargo = toolchain_and_cache(runner, &root)?;
            let out = out_dir(&root, out)?;
            let icon = icon.unwrap_or_else(|| root.join(bundle::ICON_SOURCE));
            bundle::require_icon(&icon)?;
            let plan = bundle::plan(&out, &icon)?;
            let bundle_id_env = env::var("VEXEL_BUNDLE_ID").ok();
            let display_name_env = env::var("VEXEL_DISPLAY_NAME").ok();
            run_checked(runner, &bundle::release_build(&root, &cargo))?;
            bundle::assemble(
                runner,
                &plan,
                &bundle::built_binary(&cargo),
                &root.join("resources/en.lproj/Localizable.strings"),
                &icon,
                env!("CARGO_PKG_VERSION"),
                bundle_id
                    .as_deref()
                    .or(bundle_id_env.as_deref())
                    .unwrap_or(crate::plist::BUNDLE_ID),
                display_name
                    .as_deref()
                    .or(display_name_env.as_deref())
                    .unwrap_or("Vexel"),
            )?;
            println!("bundle: {}", plan.app.display());
            Ok(())
        }
        Command::Dmg { out } => {
            let dmg = release::make_dmg(runner, &out_dir(&root, out)?)?;
            println!("dmg: {}", dmg.display());
            Ok(())
        }
        Command::Sbom { out } => {
            let sbom = release::make_sbom(runner, &root, &out_dir(&root, out)?)?;
            println!("sbom: {}", sbom.display());
            Ok(())
        }
        Command::InstallSmoke { out } => {
            release::install_smoke(runner, &out_dir(&root, out)?)?;
            println!("install-smoke: pass");
            Ok(())
        }
        Command::Notarize { path, execute } => release::notarize(
            &path,
            execute,
            env::var("VEXEL_SIGN_IDENTITY").ok().as_deref(),
            env::var("VEXEL_NOTARY_PROFILE").ok().as_deref(),
            runner,
        ),
    }
}

fn check_ci_policy(workflow: &Path) -> Result<(), String> {
    let text =
        fs::read_to_string(workflow).map_err(|error| format!("{}: {error}", workflow.display()))?;
    ci_policy::check_policy(&normalize_workflow_newlines(&text))
}

fn normalize_workflow_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn toolchain_and_cache(runner: &mut dyn Runner, root: &Path) -> Result<bundle::CargoEnv, String> {
    if !verify_toolchain(runner) {
        return Err("Exact installed Rust1.99.0 required; no install attempted".to_string());
    }
    let cargo = cargo_env(root);
    prepare_cache(&cargo)?;
    Ok(cargo)
}

fn main() -> ExitCode {
    let raw: Vec<String> = env::args().skip(1).collect();
    let command = match args::parse(&raw) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("vexel-dev: {error}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    if let Command::CheckCi { ref workflow } = command {
        return match check_ci_policy(workflow) {
            Ok(()) => {
                println!("macOS-only CI policy: pass");
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("CI policy error: {message}");
                ExitCode::FAILURE
            }
        };
    }
    match run(command, &mut SystemRunner) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("vexel-dev: {message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_workflow_newlines;

    #[test]
    fn workflow_newlines_normalize_lf_crlf_and_cr() {
        assert_eq!(normalize_workflow_newlines("a\nb\nc\n"), "a\nb\nc\n");
        assert_eq!(normalize_workflow_newlines("a\r\nb\r\nc\r\n"), "a\nb\nc\n");
        assert_eq!(normalize_workflow_newlines("a\rb\rc\r"), "a\nb\nc\n");
    }
}
