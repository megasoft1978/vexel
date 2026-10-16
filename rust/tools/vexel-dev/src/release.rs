//! DMG creation, SBOM generation and the install smoke (never launches the GUI).
use crate::bundle::APP_NAME;
use crate::plist::EXECUTABLE;
use crate::runner::{Invocation, Runner, run_checked};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub const DMG_NAME: &str = "Vexel.dmg";
pub const SBOM_NAME: &str = "sbom.cyclonedx.json";

pub fn dmg_path(out: &Path) -> PathBuf {
    out.join(DMG_NAME)
}

/// Creates `Vexel.dmg` (UDZO) from `Vexel.app` plus an `Applications` link, then
/// verifies it. The staging folder is removed on every path.
pub fn make_dmg(runner: &mut dyn Runner, out: &Path) -> Result<PathBuf, String> {
    let app = out.join(APP_NAME);
    if !app.is_dir() {
        return Err(format!(
            "`{}` not found; run `vexel-dev bundle` first",
            app.display()
        ));
    }
    let staging = out.join("dmg-staging");
    let _ = fs::remove_dir_all(&staging);
    fs::create_dir_all(&staging).map_err(|e| format!("create `{}`: {e}", staging.display()))?;
    let dmg = dmg_path(out);
    let result = stage_and_create(runner, &app, &staging, &dmg);
    let _ = fs::remove_dir_all(&staging);
    result?;
    run_checked(
        runner,
        &Invocation::new("hdiutil").arg_str("verify").path_arg(&dmg),
    )?;
    Ok(dmg)
}

fn stage_and_create(
    runner: &mut dyn Runner,
    app: &Path,
    staging: &Path,
    dmg: &Path,
) -> Result<(), String> {
    run_checked(
        runner,
        &Invocation::new("ditto")
            .path_arg(app)
            .path_arg(&staging.join(APP_NAME)),
    )?;
    std::os::unix::fs::symlink("/Applications", staging.join("Applications"))
        .map_err(|e| format!("link Applications: {e}"))?;
    run_checked(
        runner,
        &Invocation::new("hdiutil")
            .args(["create", "-volname", "Vexel", "-srcfolder"])
            .path_arg(staging)
            .args(["-format", "UDZO", "-ov"])
            .path_arg(dmg),
    )?;
    Ok(())
}

/// Runs `scripts/make_sbom.py` against the bundled executable.
pub fn make_sbom(runner: &mut dyn Runner, root: &Path, out: &Path) -> Result<PathBuf, String> {
    let binary = out.join(APP_NAME).join("Contents/MacOS").join(EXECUTABLE);
    let sbom = out.join(SBOM_NAME);
    run_checked(
        runner,
        &Invocation::new("python3")
            .arg_str("scripts/make_sbom.py")
            .arg_str("--binary")
            .path_arg(&binary)
            .arg_str("--out")
            .path_arg(&sbom)
            .in_dir(root),
    )?;
    Ok(sbom)
}

/// Mounts the DMG read-only, "installs" the app into a temporary Applications
/// directory and checks plist, signature, architecture and executable bit.
/// The volume is detached on every path after a successful attach.
pub fn install_smoke(runner: &mut dyn Runner, out: &Path) -> Result<(), String> {
    let dmg = dmg_path(out);
    if !dmg.is_file() {
        return Err(format!(
            "`{}` not found; run `vexel-dev dmg` first",
            dmg.display()
        ));
    }
    let work = out.join("install-smoke");
    let (mount, applications) = (work.join("mnt"), work.join("Applications"));
    let _ = fs::remove_dir_all(&work);
    for dir in [&mount, &applications] {
        fs::create_dir_all(dir).map_err(|e| format!("create `{}`: {e}", dir.display()))?;
    }
    let attach = Invocation::new("hdiutil")
        .args([
            "attach",
            "-readonly",
            "-nobrowse",
            "-noverify",
            "-mountpoint",
        ])
        .path_arg(&mount)
        .path_arg(&dmg);
    if let Err(error) = run_checked(runner, &attach) {
        let _ = fs::remove_dir_all(&work);
        return Err(error);
    }
    let checks = smoke_checks(runner, &mount, &applications);
    let detached = detach(runner, &mount);
    if detached.is_ok() {
        let _ = fs::remove_dir_all(&work);
    }
    checks.and(detached)
}

fn smoke_checks(runner: &mut dyn Runner, mount: &Path, applications: &Path) -> Result<(), String> {
    let installed = applications.join(APP_NAME);
    run_checked(
        runner,
        &Invocation::new("ditto")
            .path_arg(&mount.join(APP_NAME))
            .path_arg(&installed),
    )?;
    run_checked(
        runner,
        &Invocation::new("plutil")
            .arg_str("-lint")
            .path_arg(&installed.join("Contents/Info.plist")),
    )?;
    run_checked(
        runner,
        &Invocation::new("codesign")
            .args(["--verify", "--strict", "--verbose=2"])
            .path_arg(&installed),
    )?;
    let executable = installed.join("Contents/MacOS").join(EXECUTABLE);
    let archs = run_checked(
        runner,
        &Invocation::new("lipo")
            .arg_str("-archs")
            .path_arg(&executable),
    )?;
    if !archs.stdout.split_whitespace().any(|a| a == "arm64") {
        return Err(format!(
            "executable archs `{}` lack arm64",
            archs.stdout.trim()
        ));
    }
    let mode = fs::metadata(&executable)
        .map_err(|e| format!("stat `{}`: {e}", executable.display()))?
        .permissions()
        .mode();
    if mode & 0o111 == 0 {
        return Err(format!("`{}` is not executable", executable.display()));
    }
    Ok(())
}

fn detach(runner: &mut dyn Runner, mount: &Path) -> Result<(), String> {
    let plain = Invocation::new("hdiutil").arg_str("detach").path_arg(mount);
    if run_checked(runner, &plain).is_ok() {
        return Ok(());
    }
    let forced = Invocation::new("hdiutil")
        .args(["detach", "-force"])
        .path_arg(mount);
    run_checked(runner, &forced).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::Output;
    use crate::runner::fake::{RecordingRunner, fail, ok};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("vexel-dev-{tag}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Fake `ditto` that materialises an installed app with an executable file.
    fn fake_ditto(inv: &Invocation) {
        if inv.program == "ditto" {
            let bin = Path::new(&inv.args[1]).join("Contents/MacOS");
            fs::create_dir_all(&bin).unwrap();
            let exe = bin.join(EXECUTABLE);
            fs::write(&exe, b"x").unwrap();
            fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    fn smoke_dir(tag: &str) -> PathBuf {
        let out = temp(tag);
        fs::write(dmg_path(&out), b"dmg").unwrap();
        out
    }

    #[test]
    fn install_smoke_runs_the_full_sequence_and_detaches_last() {
        let out = smoke_dir("smoke-ok");
        let mut runner = RecordingRunner::new(|inv| {
            fake_ditto(inv);
            (inv.program == "lipo").then(|| ok("arm64\n"))
        });
        install_smoke(&mut runner, &out).unwrap();
        let expected = [
            "hdiutil attach -readonly -nobrowse -noverify -mountpoint ",
            "ditto ",
            "plutil -lint ",
            "codesign --verify --strict --verbose=2 ",
            "lipo -archs ",
            "hdiutil detach ",
        ];
        assert_eq!(runner.calls.len(), expected.len(), "{:?}", runner.calls);
        for (call, prefix) in runner.calls.iter().zip(expected) {
            assert!(
                call.starts_with(prefix),
                "{call} should start with {prefix}"
            );
        }
        assert!(!out.join("install-smoke").exists(), "work dir cleaned");
        fs::remove_dir_all(&out).unwrap();
    }

    #[test]
    fn forced_failure_still_detaches() {
        let out = smoke_dir("smoke-fail");
        let mut runner = RecordingRunner::new(|inv| {
            fake_ditto(inv);
            (inv.program == "plutil").then(|| fail("bad plist"))
        });
        let error = install_smoke(&mut runner, &out).unwrap_err();
        assert!(error.contains("bad plist"), "{error}");
        let last = runner.calls.last().unwrap();
        assert!(last.starts_with("hdiutil detach "), "{last}");
        assert!(
            !runner.calls.iter().any(|c| c.starts_with("lipo")),
            "stops at first failure"
        );
        fs::remove_dir_all(&out).unwrap();
    }

    #[test]
    fn missing_arm64_fails_and_detaches() {
        let out = smoke_dir("smoke-arch");
        let mut runner = RecordingRunner::new(|inv| {
            fake_ditto(inv);
            (inv.program == "lipo").then(|| ok("x86_64\n"))
        });
        let error = install_smoke(&mut runner, &out).unwrap_err();
        assert!(error.contains("lack arm64"), "{error}");
        assert!(runner.calls.last().unwrap().starts_with("hdiutil detach "));
        fs::remove_dir_all(&out).unwrap();
    }

    #[test]
    fn failed_detach_is_retried_with_force_and_reported_if_it_persists() {
        let out = smoke_dir("smoke-detach");
        let mut runner = RecordingRunner::new(|inv| {
            fake_ditto(inv);
            if inv.program == "lipo" {
                return Some(ok("arm64"));
            }
            (inv.program == "hdiutil" && inv.args[0] == "detach").then(|| fail("busy"))
        });
        let error = install_smoke(&mut runner, &out).unwrap_err();
        assert!(error.contains("busy"));
        let tail: Vec<&String> = runner.calls.iter().rev().take(2).collect();
        assert!(tail[0].starts_with("hdiutil detach -force "), "{tail:?}");
        assert!(tail[1].starts_with("hdiutil detach /"), "{tail:?}");
        fs::remove_dir_all(&out).unwrap();
    }

    #[test]
    fn failed_attach_never_detaches() {
        let out = smoke_dir("smoke-attach");
        let mut runner = RecordingRunner::new(|inv| {
            (inv.args.first().is_some_and(|a| a == "attach")).then(|| fail("no disk"))
        });
        assert!(install_smoke(&mut runner, &out).is_err());
        assert_eq!(runner.calls.len(), 1);
        fs::remove_dir_all(&out).unwrap();
    }

    #[test]
    fn exec_bit_is_required() {
        let out = smoke_dir("smoke-exec");
        let mut runner = RecordingRunner::new(|inv| {
            fake_ditto(inv);
            if inv.program == "ditto" {
                let exe = Path::new(&inv.args[1])
                    .join("Contents/MacOS")
                    .join(EXECUTABLE);
                fs::set_permissions(&exe, fs::Permissions::from_mode(0o644)).unwrap();
            }
            (inv.program == "lipo").then(|| ok("arm64"))
        });
        assert!(
            install_smoke(&mut runner, &out)
                .unwrap_err()
                .contains("not executable")
        );
        assert!(runner.calls.last().unwrap().starts_with("hdiutil detach "));
        fs::remove_dir_all(&out).unwrap();
    }

    #[test]
    fn dmg_stages_creates_then_verifies() {
        let out = temp("dmg");
        fs::create_dir_all(out.join(APP_NAME)).unwrap();
        let mut runner = RecordingRunner::new(|_| None::<Output>);
        let dmg = make_dmg(&mut runner, &out).unwrap();
        assert_eq!(dmg, out.join(DMG_NAME));
        assert_eq!(runner.calls.len(), 3);
        assert!(runner.calls[0].starts_with("ditto "));
        assert!(runner.calls[1].starts_with("hdiutil create -volname Vexel -srcfolder "));
        assert!(runner.calls[1].contains(" -format UDZO -ov "));
        assert!(runner.calls[2].starts_with("hdiutil verify "));
        assert!(!out.join("dmg-staging").exists());
        fs::remove_dir_all(&out).unwrap();
    }

    #[test]
    fn dmg_requires_a_bundle_and_cleans_staging_on_failure() {
        let out = temp("dmg-fail");
        let mut runner = RecordingRunner::new(|_| None);
        assert!(
            make_dmg(&mut runner, &out)
                .unwrap_err()
                .contains("vexel-dev bundle")
        );
        fs::create_dir_all(out.join(APP_NAME)).unwrap();
        let mut failing = RecordingRunner::new(|inv| {
            (inv.args.first().is_some_and(|a| a == "create")).then(|| fail("full"))
        });
        assert!(make_dmg(&mut failing, &out).is_err());
        assert!(!out.join("dmg-staging").exists());
        fs::remove_dir_all(&out).unwrap();
    }

    #[test]
    fn sbom_invokes_the_script_in_the_repo_root() {
        let mut runner = RecordingRunner::new(|_| None);
        let sbom = make_sbom(&mut runner, Path::new("/r"), Path::new("/o")).unwrap();
        assert_eq!(sbom, Path::new("/o/sbom.cyclonedx.json"));
        assert_eq!(
            runner.calls,
            [
                "python3 scripts/make_sbom.py --binary /o/Vexel.app/Contents/MacOS/vexel --out /o/sbom.cyclonedx.json"
            ]
        );
    }
}
