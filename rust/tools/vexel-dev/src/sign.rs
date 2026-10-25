//! Developer ID signing, versioned release artifacts and the `release` flow.
//!
//! No entitlements file is generated: the app is a hardened-runtime native
//! binary that needs no JIT, unsigned memory or network-server entitlement.
use crate::bundle::{APP_NAME, BundlePlan};
use crate::notarize::notarize;
use crate::release::{dmg_path, make_dmg};
use crate::runner::{Invocation, Runner, run_checked};
use std::fs;
use std::path::{Path, PathBuf};

/// Signs the executable first, then the bundle (no `--deep`), and verifies.
/// `spctl` is informational: before notarization it reports
/// `rejected` / `Unnotarized Developer ID`.
pub fn sign_bundle(
    runner: &mut dyn Runner,
    plan: &BundlePlan,
    identity: &str,
) -> Result<(), String> {
    for target in [&plan.executable, &plan.app] {
        run_checked(
            runner,
            &Invocation::new("codesign")
                .args([
                    "--force",
                    "--options",
                    "runtime",
                    "--timestamp",
                    "--sign",
                    identity,
                ])
                .path_arg(target),
        )?;
    }
    run_checked(
        runner,
        &Invocation::new("codesign")
            .args(["--verify", "--strict", "--deep", "--verbose=2"])
            .path_arg(&plan.app),
    )?;
    assess(
        runner,
        Invocation::new("spctl")
            .args(["--assess", "--type", "execute", "-vv"])
            .path_arg(&plan.app),
    );
    Ok(())
}

pub fn sign_dmg(runner: &mut dyn Runner, dmg: &Path, identity: &str) -> Result<(), String> {
    run_checked(
        runner,
        &Invocation::new("codesign")
            .args(["--force", "--timestamp", "--sign", identity])
            .path_arg(dmg),
    )?;
    run_checked(
        runner,
        &Invocation::new("codesign")
            .args(["--verify", "--strict", "--verbose=2"])
            .path_arg(dmg),
    )?;
    Ok(())
}

/// Runs a Gatekeeper assessment and prints the verdict; never fails the build.
fn assess(runner: &mut dyn Runner, inv: Invocation) {
    match runner.run(&inv) {
        Ok(out) if out.success => println!("spctl: accepted"),
        Ok(out) => println!(
            "spctl (informational): rejected ({}); expected until notarized",
            out.stderr.trim().replace('\n', "; ")
        ),
        Err(e) => println!("spctl (informational): not run: {e}"),
    }
}

pub struct Artifacts {
    pub dmg: PathBuf,
    pub zip: PathBuf,
    pub sha256: PathBuf,
}

/// Writes `Vexel-<version>.dmg` (copy of the signed `Vexel.dmg`), `.zip` of the
/// bundle and a `.sha256` listing both.
pub fn package_artifacts(
    runner: &mut dyn Runner,
    out: &Path,
    version: &str,
) -> Result<Artifacts, String> {
    let app = out.join(APP_NAME);
    let source = dmg_path(out);
    for p in [&app, &source] {
        if !p.exists() {
            return Err(format!(
                "`{}` not found; run `vexel-dev release`/`dmg` first",
                p.display()
            ));
        }
    }
    let stem = format!("Vexel-{version}");
    let dmg = out.join(format!("{stem}.dmg"));
    let zip = out.join(format!("{stem}.zip"));
    let sha256 = out.join(format!("{stem}.sha256"));
    fs::copy(&source, &dmg).map_err(|e| format!("copy `{}`: {e}", dmg.display()))?;
    let _ = fs::remove_file(&zip);
    run_checked(
        runner,
        &Invocation::new("ditto")
            .args(["-c", "-k", "--keepParent"])
            .path_arg(&app)
            .path_arg(&zip),
    )?;
    let mut sums = String::new();
    for file in [&dmg, &zip] {
        let name = file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let result = run_checked(
            runner,
            &Invocation::new("shasum")
                .args(["-a", "256"])
                .arg_str(name)
                .in_dir(out),
        )?;
        let line = result.stdout.trim();
        let digest = line.split_whitespace().next().unwrap_or_default();
        if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!("`shasum` returned an invalid digest for {name}"));
        }
        sums.push_str(&format!("{digest}  {name}\n"));
    }
    fs::write(&sha256, sums).map_err(|e| format!("write `{}`: {e}", sha256.display()))?;
    Ok(Artifacts { dmg, zip, sha256 })
}

/// Bundle is already built and signed: DMG, sign it, notarize + staple when the
/// keychain profile exists, then versioned artifacts. Returns whether the
/// result was notarized.
pub fn finish_release(
    runner: &mut dyn Runner,
    plan: &BundlePlan,
    out: &Path,
    version: &str,
    identity: &str,
    profile: Option<&str>,
) -> Result<bool, String> {
    let dmg = make_dmg(runner, out)?;
    sign_dmg(runner, &dmg, identity)?;
    let notarized = match profile.filter(|p| !p.is_empty()) {
        Some(p) if profile_exists(runner, p) => {
            notarize(&dmg, true, Some(p), runner)?;
            run_checked(
                runner,
                &Invocation::new("xcrun")
                    .args(["stapler", "staple"])
                    .path_arg(&plan.app),
            )?;
            true
        }
        Some(p) => {
            println!(
                "release: notary profile `{p}` not found in the keychain; stopped after signing.\n\
                 One-time setup: xcrun notarytool store-credentials {p} --apple-id <apple id> \
                 --team-id <TEAMID> --password <app-specific-password>"
            );
            false
        }
        None => {
            println!(
                "release: VEXEL_NOTARY_PROFILE not set; stopped after signing (not notarized). \
                 See docs/release.md for the one-time profile setup."
            );
            false
        }
    };
    let artifacts = package_artifacts(runner, out, version)?;
    println!("dmg: {}", artifacts.dmg.display());
    println!("zip: {}", artifacts.zip.display());
    println!("sha256: {}", artifacts.sha256.display());
    Ok(notarized)
}

fn profile_exists(runner: &mut dyn Runner, profile: &str) -> bool {
    runner
        .run(&Invocation::new("xcrun").args([
            "notarytool",
            "history",
            "--keychain-profile",
            profile,
        ]))
        .is_ok_and(|o| o.success)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::plan;
    use crate::runner::fake::{RecordingRunner, fail, ok};

    fn temp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vexel-sign-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn bundle_plan() -> BundlePlan {
        plan(Path::new("/o"), Path::new("/x/a.icns")).unwrap()
    }

    #[test]
    fn signs_executable_then_bundle_then_verifies_and_assesses() {
        let mut runner =
            RecordingRunner::new(|inv| (inv.program == "spctl").then(|| fail("rejected")));
        sign_bundle(&mut runner, &bundle_plan(), "ID").unwrap();
        let base = "codesign --force --options runtime --timestamp --sign ID ";
        assert_eq!(
            runner.calls,
            [
                format!("{base}/o/Vexel.app/Contents/MacOS/vexel"),
                format!("{base}/o/Vexel.app"),
                "codesign --verify --strict --deep --verbose=2 /o/Vexel.app".to_string(),
                "spctl --assess --type execute -vv /o/Vexel.app".to_string(),
            ]
        );
    }

    #[test]
    fn sign_failure_stops_early() {
        let mut runner = RecordingRunner::new(|_| Some(fail("no identity")));
        assert!(sign_bundle(&mut runner, &bundle_plan(), "ID").is_err());
        assert_eq!(runner.calls.len(), 1);
    }

    #[test]
    fn dmg_is_signed_without_runtime_option() {
        let mut runner = RecordingRunner::new(|_| None);
        sign_dmg(&mut runner, Path::new("/o/Vexel.dmg"), "ID").unwrap();
        assert_eq!(
            runner.calls[0],
            "codesign --force --timestamp --sign ID /o/Vexel.dmg"
        );
        assert!(runner.calls[1].starts_with("codesign --verify --strict"));
    }

    #[test]
    fn artifacts_are_versioned_with_checksums() {
        let out = temp("pkg");
        fs::create_dir_all(out.join(APP_NAME)).unwrap();
        fs::write(dmg_path(&out), b"dmg").unwrap();
        let digest = "a".repeat(64);
        let mut runner = RecordingRunner::new(move |inv| {
            (inv.program == "shasum").then(|| ok(&format!("{digest}  {}\n", inv.args[2])))
        });
        let a = package_artifacts(&mut runner, &out, "1.2.3").unwrap();
        assert_eq!(a.dmg, out.join("Vexel-1.2.3.dmg"));
        assert_eq!(a.zip, out.join("Vexel-1.2.3.zip"));
        assert!(runner.calls[0].starts_with("ditto -c -k --keepParent "));
        let text = fs::read_to_string(&a.sha256).unwrap();
        assert!(text.contains("  Vexel-1.2.3.dmg\n") && text.contains("  Vexel-1.2.3.zip\n"));
        fs::remove_dir_all(&out).unwrap();
    }

    #[test]
    fn invalid_digest_is_rejected() {
        let out = temp("badsum");
        fs::create_dir_all(out.join(APP_NAME)).unwrap();
        fs::write(dmg_path(&out), b"dmg").unwrap();
        let mut runner = RecordingRunner::new(|inv| (inv.program == "shasum").then(|| ok("zz  x")));
        assert!(package_artifacts(&mut runner, &out, "1").is_err());
        fs::remove_dir_all(&out).unwrap();
    }

    #[test]
    fn release_without_profile_stops_after_signing_but_packages() {
        let out = temp("rel");
        fs::create_dir_all(out.join(APP_NAME)).unwrap();
        let digest = "b".repeat(64);
        let dmg = dmg_path(&out);
        let mut runner = RecordingRunner::new(move |inv| {
            if inv.program == "hdiutil" && inv.args[0] == "create" {
                fs::write(&dmg, b"dmg").unwrap();
            }
            (inv.program == "shasum").then(|| ok(&format!("{digest}  f")))
        });
        let p = plan(&out, Path::new("/x/a.icns")).unwrap();
        let done = finish_release(&mut runner, &p, &out, "0.1.0", "ID", None).unwrap();
        assert!(!done);
        assert!(!runner.calls.iter().any(|c| c.contains("notarytool")));
        assert!(
            runner
                .calls
                .iter()
                .any(|c| c.starts_with("codesign --force --timestamp --sign ID "))
        );
        fs::remove_dir_all(&out).unwrap();
    }

    #[test]
    fn release_with_missing_profile_does_not_submit() {
        let out = temp("relmiss");
        fs::create_dir_all(out.join(APP_NAME)).unwrap();
        let digest = "c".repeat(64);
        let dmg = dmg_path(&out);
        let mut runner = RecordingRunner::new(move |inv| {
            if inv.program == "hdiutil" && inv.args[0] == "create" {
                fs::write(&dmg, b"dmg").unwrap();
            }
            if inv.args.first().is_some_and(|a| a == "notarytool") {
                return Some(fail("no profile"));
            }
            (inv.program == "shasum").then(|| ok(&format!("{digest}  f")))
        });
        let p = plan(&out, Path::new("/x/a.icns")).unwrap();
        let done =
            finish_release(&mut runner, &p, &out, "0.1.0", "ID", Some("vexel-notary")).unwrap();
        assert!(!done);
        assert!(!runner.calls.iter().any(|c| c.contains("submit")));
        fs::remove_dir_all(&out).unwrap();
    }
}
