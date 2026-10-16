//! `Vexel.app` assembly: release build, bundle layout, ad-hoc signing.
use crate::plist::{BUNDLE_ID, EXECUTABLE, render_info_plist};
use crate::runner::{Invocation, Runner, run_checked};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub const TARGET_TRIPLE: &str = "aarch64-apple-darwin";
pub const APP_NAME: &str = "Vexel.app";

/// Repo-local cargo locations; `CARGO_HOME` / `CARGO_TARGET_DIR` from the
/// environment win so isolated worktrees can share an offline registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoEnv {
    pub home: PathBuf,
    pub target: PathBuf,
    pub tmp: PathBuf,
}

pub fn cargo_env(root: &Path, home: Option<&str>, target: Option<&str>) -> CargoEnv {
    CargoEnv {
        home: home.map_or_else(|| root.join(".cache/cargo"), PathBuf::from),
        target: target.map_or_else(|| root.join("build/rust"), PathBuf::from),
        tmp: root.join(".cache/tmp"),
    }
}

pub fn cargo_invocation(subcommand: &[&str], root: &Path, env: &CargoEnv) -> Invocation {
    Invocation::new("cargo")
        .args(["+stable"])
        .args(subcommand)
        .in_dir(root)
        .env("CARGO_HOME", &env.home)
        .env("CARGO_TARGET_DIR", &env.target)
        .env("TMPDIR", &env.tmp)
}

pub fn release_build(root: &Path, env: &CargoEnv) -> Invocation {
    cargo_invocation(
        &[
            "build",
            "-p",
            "vexel-app",
            "--release",
            "--locked",
            "--offline",
            "--target",
            TARGET_TRIPLE,
        ],
        root,
        env,
    )
}

pub fn built_binary(env: &CargoEnv) -> PathBuf {
    env.target
        .join(TARGET_TRIPLE)
        .join("release")
        .join(EXECUTABLE)
}

/// Where each piece of the bundle goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundlePlan {
    pub app: PathBuf,
    pub executable: PathBuf,
    pub lproj: PathBuf,
    pub strings: PathBuf,
    pub plist: PathBuf,
    /// Destination of the optional icon inside `Contents/Resources`.
    pub icon: Option<PathBuf>,
}

pub fn plan(out: &Path, icon: Option<&Path>) -> Result<BundlePlan, String> {
    let contents = out.join(APP_NAME).join("Contents");
    let resources = contents.join("Resources");
    let icon = match icon {
        None => None,
        Some(path) => {
            let name = path
                .file_name()
                .filter(|_| path.extension().is_some_and(|e| e == "icns"))
                .ok_or_else(|| format!("icon `{}` must be a .icns file", path.display()))?;
            Some(resources.join(name))
        }
    };
    Ok(BundlePlan {
        app: out.join(APP_NAME),
        executable: contents.join("MacOS").join(EXECUTABLE),
        lproj: resources.join("en.lproj"),
        strings: resources.join("en.lproj").join("Localizable.strings"),
        plist: contents.join("Info.plist"),
        icon,
    })
}

fn io_err(action: &str, path: &Path, error: std::io::Error) -> String {
    format!("{action} `{}`: {error}", path.display())
}

/// Lays out the bundle from an already built `binary`, then signs and verifies
/// it ad hoc. Real identities and notarization are owner-gated and not used.
pub fn assemble(
    runner: &mut dyn Runner,
    plan: &BundlePlan,
    binary: &Path,
    strings_source: &Path,
    icon_source: Option<&Path>,
    version: &str,
) -> Result<(), String> {
    if plan.app.exists() {
        fs::remove_dir_all(&plan.app).map_err(|e| io_err("remove old bundle", &plan.app, e))?;
    }
    for dir in [plan.executable.parent(), Some(plan.lproj.as_path())] {
        let dir = dir.expect("bundle paths have parents");
        fs::create_dir_all(dir).map_err(|e| io_err("create", dir, e))?;
    }
    fs::copy(binary, &plan.executable).map_err(|e| io_err("copy binary", binary, e))?;
    fs::set_permissions(&plan.executable, fs::Permissions::from_mode(0o755))
        .map_err(|e| io_err("chmod", &plan.executable, e))?;
    fs::copy(strings_source, &plan.strings).map_err(|e| io_err("copy", strings_source, e))?;
    let icon_name = match (icon_source, &plan.icon) {
        (Some(source), Some(dest)) => {
            fs::copy(source, dest).map_err(|e| io_err("copy icon", source, e))?;
            dest.file_name().map(|n| n.to_string_lossy().into_owned())
        }
        _ => None,
    };
    let plist = render_info_plist(version, BUNDLE_ID, icon_name.as_deref());
    fs::write(&plan.plist, plist).map_err(|e| io_err("write", &plan.plist, e))?;
    run_checked(
        runner,
        &Invocation::new("codesign")
            .args(["--force", "--sign", "-", "--timestamp=none"])
            .path_arg(&plan.app),
    )?;
    run_checked(
        runner,
        &Invocation::new("codesign")
            .args(["--verify", "--strict", "--verbose=2"])
            .path_arg(&plan.app),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::fake::{RecordingRunner, fail};
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

    #[test]
    fn plan_places_every_piece_inside_the_bundle() {
        let plan = plan(Path::new("/o"), Some(Path::new("/x/Art.icns"))).unwrap();
        assert_eq!(plan.app, Path::new("/o/Vexel.app"));
        assert_eq!(
            plan.executable,
            Path::new("/o/Vexel.app/Contents/MacOS/vexel")
        );
        assert_eq!(plan.plist, Path::new("/o/Vexel.app/Contents/Info.plist"));
        assert_eq!(
            plan.strings,
            Path::new("/o/Vexel.app/Contents/Resources/en.lproj/Localizable.strings")
        );
        assert_eq!(
            plan.icon.as_deref(),
            Some(Path::new("/o/Vexel.app/Contents/Resources/Art.icns"))
        );
        assert_eq!(super::plan(Path::new("/o"), None).unwrap().icon, None);
        assert!(super::plan(Path::new("/o"), Some(Path::new("/x/a.png"))).is_err());
    }

    #[test]
    fn cargo_env_prefers_environment_then_repo_local() {
        let root = Path::new("/r");
        let local = cargo_env(root, None, None);
        assert_eq!(local.home, Path::new("/r/.cache/cargo"));
        assert_eq!(local.target, Path::new("/r/build/rust"));
        let shared = cargo_env(root, Some("/h"), Some("/t"));
        assert_eq!(
            (shared.home.as_path(), shared.target.as_path()),
            (Path::new("/h"), Path::new("/t"))
        );
        assert_eq!(
            built_binary(&shared),
            Path::new("/t/aarch64-apple-darwin/release/vexel")
        );
    }

    #[test]
    fn release_build_is_locked_offline_and_release() {
        let line = release_build(Path::new("/r"), &cargo_env(Path::new("/r"), None, None)).line();
        assert_eq!(
            line,
            "cargo +stable build -p vexel-app --release --locked --offline --target aarch64-apple-darwin"
        );
    }

    #[test]
    fn assemble_lays_out_bundle_then_signs_and_verifies() {
        let dir = temp("assemble");
        let binary = dir.join("vexel-bin");
        let strings = dir.join("Localizable.strings");
        fs::write(&binary, b"bin").unwrap();
        fs::write(&strings, b"\"a\" = \"b\";").unwrap();
        let plan = plan(&dir.join("out"), None).unwrap();
        let mut runner = RecordingRunner::new(|_| None);
        assemble(&mut runner, &plan, &binary, &strings, None, "9.8.7").unwrap();
        assert_eq!(
            runner.calls,
            [
                format!(
                    "codesign --force --sign - --timestamp=none {}",
                    plan.app.display()
                ),
                format!(
                    "codesign --verify --strict --verbose=2 {}",
                    plan.app.display()
                ),
            ]
        );
        let mode = fs::metadata(&plan.executable).unwrap().permissions().mode();
        assert_eq!(mode & 0o111, 0o111);
        assert_eq!(fs::read(&plan.strings).unwrap(), b"\"a\" = \"b\";");
        let plist = fs::read_to_string(&plan.plist).unwrap();
        assert!(plist.contains("<string>9.8.7</string>"));
        assert!(!plist.contains("CFBundleIconFile"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn assemble_copies_icon_and_names_it_in_the_plist() {
        let dir = temp("icon");
        let (binary, strings, icon) = (dir.join("b"), dir.join("s"), dir.join("Vexel.icns"));
        for file in [&binary, &strings, &icon] {
            fs::write(file, b"x").unwrap();
        }
        let plan = plan(&dir.join("out"), Some(&icon)).unwrap();
        let mut runner = RecordingRunner::new(|_| None);
        assemble(&mut runner, &plan, &binary, &strings, Some(&icon), "1.0.0").unwrap();
        assert!(plan.icon.as_ref().unwrap().is_file());
        let plist = fs::read_to_string(&plan.plist).unwrap();
        assert!(plist.contains("<key>CFBundleIconFile</key>\n\t<string>Vexel.icns</string>"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_failing_codesign_stops_before_verify() {
        let dir = temp("signfail");
        let (binary, strings) = (dir.join("b"), dir.join("s"));
        fs::write(&binary, b"x").unwrap();
        fs::write(&strings, b"x").unwrap();
        let plan = plan(&dir.join("out"), None).unwrap();
        let mut runner = RecordingRunner::new(|_| Some(fail("no identity")));
        let error = assemble(&mut runner, &plan, &binary, &strings, None, "1.0.0").unwrap_err();
        assert!(error.contains("codesign") && error.contains("no identity"));
        assert_eq!(runner.calls.len(), 1);
        fs::remove_dir_all(&dir).unwrap();
    }
}
