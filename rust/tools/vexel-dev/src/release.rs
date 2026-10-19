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

/// Generates a CycloneDX SBOM for the bundled executable.
pub fn make_sbom(runner: &mut dyn Runner, _root: &Path, out: &Path) -> Result<PathBuf, String> {
    let binary = out.join(APP_NAME).join("Contents/MacOS").join(EXECUTABLE);
    let sbom = out.join(SBOM_NAME);
    if !binary.is_file() {
        return Err(format!(
            "no built binary at {}; run `vexel-dev bundle` first",
            binary.display()
        ));
    }
    let digest_output = run_checked(
        runner,
        &Invocation::new("shasum")
            .args(["-a", "256"])
            .path_arg(&binary),
    )?;
    let digest = digest_output.stdout.split_whitespace().next().unwrap_or("");
    let digest = digest.strip_prefix('\\').unwrap_or(digest);
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "`shasum -a 256` returned an invalid SHA-256 digest for {}",
            binary.display()
        ));
    }
    let otool = runner
        .run(&Invocation::new("otool").arg_str("-L").path_arg(&binary))
        .map_err(|e| format!("could not start `otool -L {}`: {e}", binary.display()))?;
    let mut libraries = if otool.success {
        otool
            .stdout
            .lines()
            .skip(1)
            .filter_map(|line| {
                let name = line.trim().split(" (").next().unwrap_or("").trim();
                (!name.is_empty()).then(|| name.to_string())
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    libraries.sort();
    libraries.dedup();

    let version = env!("CARGO_PKG_VERSION");
    let mut components = vec![format!(
        "    {{\n      \"type\": \"application\",\n      \"bom-ref\": {},\n      \"name\": \"vexel\",\n      \"version\": {},\n      \"licenses\": [{{\"license\": {{\"id\": \"MPL-2.0\"}}}}],\n      \"hashes\": [{{\"alg\": \"SHA-256\", \"content\": {}}}]\n    }}",
        json_string(&format!("pkg:generic/vexel@{version}")),
        json_string(version),
        json_string(digest),
    )];
    for library in libraries {
        let basename = Path::new(&library)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        let purl = format!("pkg:generic/{basename}?os=macos");
        let description =
            if library.starts_with("/usr/lib/") || library.starts_with("/System/Library/") {
                "macOS system library".to_string()
            } else {
                format!("Linked library at {library}")
            };
        components.push(format!(
            "    {{\n      \"type\": \"library\",\n      \"bom-ref\": {},\n      \"name\": {},\n      \"version\": \"system\",\n      \"scope\": \"required\",\n      \"description\": {}\n    }}",
            json_string(&purl),
            json_string(basename),
            json_string(&description),
        ));
    }
    let contents = format!(
        "{{\n  \"bomFormat\": \"CycloneDX\",\n  \"specVersion\": \"1.5\",\n  \"version\": 1,\n  \"metadata\": {{\n    \"component\": {{\n      \"type\": \"application\",\n      \"name\": \"vexel\",\n      \"version\": {},\n      \"licenses\": [{{\"license\": {{\"id\": \"MPL-2.0\"}}}}]\n    }},\n    \"tools\": [{{\"name\": \"vexel-dev\", \"version\": {}}}]\n  }},\n  \"components\": [\n{}\n  ]\n}}\n",
        json_string(version),
        json_string(version),
        components.join(",\n"),
    );
    fs::create_dir_all(out).map_err(|e| format!("create `{}`: {e}", out.display()))?;
    fs::write(&sbom, contents).map_err(|e| format!("write `{}`: {e}", sbom.display()))?;
    Ok(sbom)
}

fn json_string(value: &str) -> String {
    let mut escaped = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0C}' => escaped.push_str("\\f"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            control if control <= '\u{1F}' => {
                use std::fmt::Write as _;
                write!(escaped, "\\u{:04x}", control as u32).unwrap();
            }
            other => escaped.push(other),
        }
    }
    escaped.push('"');
    escaped
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

    fn sbom_fixture(tag: &str) -> (PathBuf, PathBuf) {
        let out = temp(tag);
        let binary = out.join(APP_NAME).join("Contents/MacOS").join(EXECUTABLE);
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::write(&binary, b"fixture executable").unwrap();
        (out, binary)
    }

    #[derive(Debug, PartialEq)]
    enum JsonValue {
        Object(Vec<(String, JsonValue)>),
        Array(Vec<JsonValue>),
        String(String),
        Number(u64),
    }

    impl JsonValue {
        fn field(&self, name: &str) -> &JsonValue {
            match self {
                Self::Object(fields) => &fields.iter().find(|(key, _)| key == name).unwrap().1,
                _ => panic!("expected JSON object"),
            }
        }

        fn string(&self) -> &str {
            match self {
                Self::String(value) => value,
                _ => panic!("expected JSON string"),
            }
        }

        fn array(&self) -> &[JsonValue] {
            match self {
                Self::Array(values) => values,
                _ => panic!("expected JSON array"),
            }
        }

        fn number(&self) -> u64 {
            match self {
                Self::Number(value) => *value,
                _ => panic!("expected JSON number"),
            }
        }
    }

    fn parse_json(source: &str) -> JsonValue {
        struct Parser {
            chars: Vec<char>,
            cursor: usize,
        }

        impl Parser {
            fn whitespace(&mut self) {
                while self
                    .chars
                    .get(self.cursor)
                    .is_some_and(|ch| ch.is_whitespace())
                {
                    self.cursor += 1;
                }
            }

            fn take(&mut self, expected: char) {
                self.whitespace();
                assert_eq!(self.chars.get(self.cursor), Some(&expected));
                self.cursor += 1;
            }

            fn string(&mut self) -> String {
                self.take('"');
                let mut value = String::new();
                loop {
                    let ch = *self
                        .chars
                        .get(self.cursor)
                        .expect("unterminated JSON string");
                    self.cursor += 1;
                    match ch {
                        '"' => return value,
                        '\\' => {
                            let escaped = *self.chars.get(self.cursor).expect("incomplete escape");
                            self.cursor += 1;
                            match escaped {
                                '"' => value.push('"'),
                                '\\' => value.push('\\'),
                                '/' => value.push('/'),
                                'b' => value.push('\u{08}'),
                                'f' => value.push('\u{0C}'),
                                'n' => value.push('\n'),
                                'r' => value.push('\r'),
                                't' => value.push('\t'),
                                'u' => {
                                    let mut codepoint = 0u32;
                                    for _ in 0..4 {
                                        let digit = self.chars[self.cursor].to_digit(16).unwrap();
                                        self.cursor += 1;
                                        codepoint = codepoint * 16 + digit;
                                    }
                                    value.push(char::from_u32(codepoint).unwrap());
                                }
                                other => panic!("invalid JSON escape: {other}"),
                            }
                        }
                        control if control <= '\u{1F}' => {
                            panic!("unescaped JSON control character")
                        }
                        other => value.push(other),
                    }
                }
            }

            fn value(&mut self) -> JsonValue {
                self.whitespace();
                match self.chars[self.cursor] {
                    '{' => {
                        self.cursor += 1;
                        let mut fields = Vec::new();
                        self.whitespace();
                        if self.chars.get(self.cursor) == Some(&'}') {
                            self.cursor += 1;
                            return JsonValue::Object(fields);
                        }
                        loop {
                            let key = self.string();
                            self.take(':');
                            fields.push((key, self.value()));
                            self.whitespace();
                            if self.chars.get(self.cursor) == Some(&'}') {
                                self.cursor += 1;
                                return JsonValue::Object(fields);
                            }
                            self.take(',');
                        }
                    }
                    '[' => {
                        self.cursor += 1;
                        let mut values = Vec::new();
                        self.whitespace();
                        if self.chars.get(self.cursor) == Some(&']') {
                            self.cursor += 1;
                            return JsonValue::Array(values);
                        }
                        loop {
                            values.push(self.value());
                            self.whitespace();
                            if self.chars.get(self.cursor) == Some(&']') {
                                self.cursor += 1;
                                return JsonValue::Array(values);
                            }
                            self.take(',');
                        }
                    }
                    '"' => JsonValue::String(self.string()),
                    digit if digit.is_ascii_digit() => {
                        let start = self.cursor;
                        while self
                            .chars
                            .get(self.cursor)
                            .is_some_and(|ch| ch.is_ascii_digit())
                        {
                            self.cursor += 1;
                        }
                        JsonValue::Number(
                            self.chars[start..self.cursor]
                                .iter()
                                .collect::<String>()
                                .parse()
                                .unwrap(),
                        )
                    }
                    other => panic!("unsupported JSON value starting with {other}"),
                }
            }
        }

        let mut parser = Parser {
            chars: source.chars().collect(),
            cursor: 0,
        };
        let value = parser.value();
        parser.whitespace();
        assert_eq!(parser.cursor, parser.chars.len(), "trailing JSON data");
        value
    }

    fn sbom_runner(libraries: &'static str) -> RecordingRunner {
        RecordingRunner::new(move |inv| match inv.program.as_str() {
            "shasum" => Some(ok(&format!("{}  {}\n", "a".repeat(64), inv.args[2]))),
            "otool" => Some(ok(libraries)),
            _ => panic!("unexpected command: {}", inv.line()),
        })
    }

    #[test]
    fn sbom_emits_contract_and_sorts_unique_linked_libraries() {
        let (out, binary) = sbom_fixture("sbom-valid");
        let libs = "/fixture/vexel\n/usr/lib/libz.1.dylib (compatibility version 1.0.0)\n/opt/local/lib/liba.dylib (compatibility version 1.0.0)\n/usr/lib/libz.1.dylib (compatibility version 1.0.0)\n";
        let mut runner = sbom_runner(libs);
        let sbom = make_sbom(&mut runner, Path::new("/ignored"), &out).unwrap();
        let json = fs::read_to_string(&sbom).unwrap();
        let document = parse_json(&json);
        assert_eq!(document.field("bomFormat").string(), "CycloneDX");
        assert_eq!(document.field("specVersion").string(), "1.5");
        assert_eq!(document.field("version").number(), 1);
        let metadata = document.field("metadata");
        let version = env!("CARGO_PKG_VERSION");
        assert_eq!(
            metadata.field("component").field("version").string(),
            version
        );
        assert_eq!(
            metadata.field("component").field("licenses").array()[0]
                .field("license")
                .field("id")
                .string(),
            "MPL-2.0"
        );
        let components = document.field("components").array();
        assert_eq!(components.len(), 3);
        assert_eq!(components[0].field("type").string(), "application");
        assert_eq!(components[0].field("name").string(), "vexel");
        assert_eq!(components[0].field("version").string(), version);
        assert_eq!(
            components[0].field("licenses").array()[0]
                .field("license")
                .field("id")
                .string(),
            "MPL-2.0"
        );
        let hash = &components[0].field("hashes").array()[0];
        assert_eq!(hash.field("alg").string(), "SHA-256");
        assert_eq!(hash.field("content").string(), "a".repeat(64));
        assert_eq!(components[1].field("type").string(), "library");
        assert_eq!(
            components[1].field("bom-ref").string(),
            "pkg:generic/liba.dylib?os=macos"
        );
        assert_eq!(components[1].field("name").string(), "liba.dylib");
        assert_eq!(components[1].field("version").string(), "system");
        assert_eq!(components[1].field("scope").string(), "required");
        assert_eq!(
            components[1].field("description").string(),
            "Linked library at /opt/local/lib/liba.dylib"
        );
        assert_eq!(
            components[2].field("bom-ref").string(),
            "pkg:generic/libz.1.dylib?os=macos"
        );
        assert_eq!(
            components[2].field("description").string(),
            "macOS system library"
        );
        let repeat = make_sbom(&mut runner, Path::new("/ignored"), &out).unwrap();
        assert_eq!(
            fs::read_to_string(repeat).unwrap(),
            json,
            "generation is deterministic"
        );
        assert_eq!(runner.calls.len(), 4);
        for calls in runner.calls.chunks_exact(2) {
            assert!(calls[0].starts_with("shasum -a 256 "));
            assert!(calls[1].starts_with("otool -L "));
        }
        assert!(!runner.calls.iter().any(|call| call.starts_with("python")));
        assert!(binary.is_file());
        fs::remove_dir_all(out).unwrap();
    }

    #[test]
    fn sbom_accepts_shasum_escaped_filename_marker() {
        let (out, binary) = sbom_fixture("sbom-escaped\n\\filename");
        assert!(binary.to_string_lossy().contains('\n'));
        assert!(binary.to_string_lossy().contains('\\'));
        let expected_digest = "c".repeat(64);
        let mut runner = RecordingRunner::new(|inv| match inv.program.as_str() {
            "shasum" => Some(ok(&format!("\\{}  {}\n", "c".repeat(64), inv.args[2]))),
            "otool" => Some(ok("/fixture/vexel\n")),
            _ => panic!("unexpected command: {}", inv.line()),
        });

        let sbom = make_sbom(&mut runner, Path::new("/"), &out).unwrap();
        let document = parse_json(&fs::read_to_string(sbom).unwrap());
        let hash = &document.field("components").array()[0]
            .field("hashes")
            .array()[0];
        assert_eq!(hash.field("content").string(), expected_digest);
        assert_eq!(runner.calls.len(), 2);
        assert!(runner.calls[0].starts_with("shasum -a 256 "));
        fs::remove_dir_all(out).unwrap();
    }

    #[test]
    fn sbom_json_escapes_quotes_backslashes_and_controls() {
        let (out, _) = sbom_fixture("sbom-escape");
        let libs: &'static str = Box::leak(
            "/fixture/bin\n/opt/a\"b\\c\u{0001}.dylib (compatibility version 1.0.0)\n"
                .to_string()
                .into_boxed_str(),
        );
        let mut runner = sbom_runner(libs);
        let sbom = make_sbom(&mut runner, Path::new("/"), &out).unwrap();
        let json = fs::read_to_string(sbom).unwrap();
        let document = parse_json(&json);
        let components = document.field("components").array();
        assert_eq!(
            components[1].field("name").string(),
            "a\"b\\c\u{0001}.dylib"
        );
        fs::remove_dir_all(out).unwrap();
    }

    #[test]
    fn missing_binary_and_malformed_sha_do_not_write_sbom() {
        let out = temp("sbom-missing");
        let mut runner = RecordingRunner::new(|_| panic!("no command for missing binary"));
        let error = make_sbom(&mut runner, Path::new("/"), &out).unwrap_err();
        assert!(error.contains("no built binary"));
        assert!(!out.join(SBOM_NAME).exists());
        fs::remove_dir_all(&out).unwrap();

        let (out, _) = sbom_fixture("sbom-bad-sha");
        let mut runner =
            RecordingRunner::new(|inv| (inv.program == "shasum").then(|| ok("xyz  executable\n")));
        assert!(
            make_sbom(&mut runner, Path::new("/"), &out)
                .unwrap_err()
                .contains("invalid SHA-256")
        );
        assert!(!out.join(SBOM_NAME).exists());
        assert_eq!(runner.calls.len(), 1);
        fs::remove_dir_all(out).unwrap();
    }

    #[test]
    fn sbom_reports_sha_command_error_and_ignores_failed_otool() {
        let (out, _) = sbom_fixture("sbom-sha-error");
        let mut runner =
            RecordingRunner::new(|inv| (inv.program == "shasum").then(|| fail("sha unavailable")));
        assert!(
            make_sbom(&mut runner, Path::new("/"), &out)
                .unwrap_err()
                .contains("sha unavailable")
        );
        assert!(!out.join(SBOM_NAME).exists());
        fs::remove_dir_all(out).unwrap();

        let (out, _) = sbom_fixture("sbom-otool-failure");
        let mut runner = RecordingRunner::new(|inv| match inv.program.as_str() {
            "shasum" => Some(ok(&format!("{}  binary\n", "b".repeat(64)))),
            "otool" => Some(fail("not a Mach-O file")),
            _ => None,
        });
        let sbom = make_sbom(&mut runner, Path::new("/"), &out).unwrap();
        let json = fs::read_to_string(sbom).unwrap();
        let document = parse_json(&json);
        assert_eq!(document.field("components").array().len(), 1);
        fs::remove_dir_all(out).unwrap();
    }
}
