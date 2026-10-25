//! Notarization: submit, staple and assess a signed `.app` or `.dmg`.
use crate::runner::{Invocation, Runner, run_checked};
use std::fs;
use std::path::Path;

/// Commands that notarize `path`: `.app` is zipped first (notarytool rejects
/// bare bundles), `.dmg` is submitted directly; then staple and assess.
pub fn notarize_plan(path: &Path, profile: &str) -> Vec<Invocation> {
    let is_app = path.extension().is_some_and(|e| e == "app");
    let zip = path.with_extension("notarize.zip");
    let (submit, assess) = if is_app {
        (
            zip.clone(),
            Invocation::new("spctl")
                .args(["--assess", "--type", "execute", "-vv"])
                .path_arg(path),
        )
    } else {
        (
            path.to_path_buf(),
            Invocation::new("spctl")
                .args(["--assess", "--type", "open"])
                .args(["--context", "context:primary-signature", "-vv"])
                .path_arg(path),
        )
    };
    let mut plan = Vec::new();
    if is_app {
        plan.push(
            Invocation::new("ditto")
                .args(["-c", "-k", "--keepParent"])
                .path_arg(path)
                .path_arg(&zip),
        );
    }
    plan.push(
        Invocation::new("xcrun")
            .args(["notarytool", "submit"])
            .path_arg(&submit)
            .args(["--keychain-profile", profile, "--wait"]),
    );
    plan.push(
        Invocation::new("xcrun")
            .args(["stapler", "staple"])
            .path_arg(path),
    );
    plan.push(assess);
    plan
}

/// Submits, staples and assesses a signed `.app` or `.dmg`. Dry run unless
/// `execute`; signing happens in `bundle` / `package`, not here.
pub fn notarize(
    path: &Path,
    execute: bool,
    profile: Option<&str>,
    runner: &mut dyn Runner,
) -> Result<(), String> {
    if !path.exists() || !(path.extension().is_some_and(|e| e == "app" || e == "dmg")) {
        return Err(format!(
            "notarize path must exist and end in .app or .dmg: {}",
            path.display()
        ));
    }
    let shown = notarize_plan(path, "${VEXEL_NOTARY_PROFILE}");
    for inv in &shown {
        println!("{}", inv.line());
    }
    if !execute {
        return Ok(());
    }
    let profile = profile
        .filter(|v| !v.is_empty())
        .ok_or("--execute requires VEXEL_NOTARY_PROFILE")?;
    let plan = notarize_plan(path, profile);
    let last = plan.len() - 1;
    for (i, inv) in plan.iter().enumerate() {
        if i == last {
            // spctl is informational; the staple step above is the gate.
            let out = runner.run(inv).map_err(|e| format!("spctl: {e}"))?;
            println!(
                "spctl assess: {}",
                if out.success { "accepted" } else { "rejected" }
            );
        } else {
            run_checked(runner, inv)?;
        }
    }
    if path.extension().is_some_and(|e| e == "app") {
        let _ = fs::remove_file(path.with_extension("notarize.zip"));
    }
    Ok(())
}

#[cfg(test)]
mod notarize_tests {
    use super::*;
    use crate::runner::fake::RecordingRunner;

    #[test]
    fn dry_run_validates_input_and_never_invokes_runner() {
        let path = std::env::temp_dir().join(format!("vexel-notary-{}.dmg", std::process::id()));
        fs::write(&path, b"fixture").unwrap();
        let mut runner = RecordingRunner::new(|_| panic!("dry run invoked a tool"));
        notarize(&path, false, Some("must-not-print"), &mut runner).unwrap();
        assert!(runner.calls.is_empty());
        assert!(notarize(Path::new("missing.dmg"), false, None, &mut runner).is_err());
        assert!(notarize(&path, true, None, &mut runner).is_err());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn dmg_plan_submits_staples_and_assesses() {
        let lines: Vec<String> = notarize_plan(Path::new("/o/Vexel.dmg"), "prof")
            .iter()
            .map(Invocation::line)
            .collect();
        assert_eq!(
            lines[0],
            "xcrun notarytool submit /o/Vexel.dmg --keychain-profile prof --wait"
        );
        assert_eq!(lines[1], "xcrun stapler staple /o/Vexel.dmg");
        assert!(lines[2].starts_with("spctl --assess --type open "));
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn app_plan_zips_first() {
        let lines: Vec<String> = notarize_plan(Path::new("/o/Vexel.app"), "prof")
            .iter()
            .map(Invocation::line)
            .collect();
        assert_eq!(
            lines[0],
            "ditto -c -k --keepParent /o/Vexel.app /o/Vexel.notarize.zip"
        );
        assert_eq!(
            lines[1],
            "xcrun notarytool submit /o/Vexel.notarize.zip --keychain-profile prof --wait"
        );
        assert_eq!(lines[2], "xcrun stapler staple /o/Vexel.app");
        assert_eq!(lines[3], "spctl --assess --type execute -vv /o/Vexel.app");
    }

    #[test]
    fn failed_submit_stops_before_staple() {
        let path = std::env::temp_dir().join(format!("vexel-notary2-{}.dmg", std::process::id()));
        fs::write(&path, b"fixture").unwrap();
        let mut runner = RecordingRunner::new(|inv| {
            (inv.args.first().is_some_and(|a| a == "notarytool"))
                .then(|| crate::runner::fake::fail("invalid"))
        });
        assert!(notarize(&path, true, Some("p"), &mut runner).is_err());
        assert_eq!(runner.calls.len(), 1);
        fs::remove_file(path).unwrap();
    }
}
