use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use vexel_mcp_access::{AccessError, AccessPolicy, PATH_CAP};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../.cache")
            .join(format!(
                "access-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(&root).unwrap();
        Self(fs::canonicalize(root).unwrap())
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    fn mkdir(&self, name: &str) -> PathBuf {
        let path = self.path(name);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn file(&self, name: &str) -> PathBuf {
        let path = self.path(name);
        fs::write(&path, b"fixture").unwrap();
        path
    }

    fn policy(&self, allowed: &Path) -> AccessPolicy {
        AccessPolicy::new(Some(allowed.as_os_str().to_owned()), None)
    }

    fn config_home(&self, contents: &[u8]) -> PathBuf {
        let home = self.mkdir("home");
        let config_parent = home.join(".config/vexel");
        fs::create_dir_all(&config_parent).unwrap();
        let config = config_parent.join("mcp-allowed-dirs.txt");
        fs::write(&config, contents).unwrap();
        fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
        home
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn text(path: &Path) -> &str {
    path.to_str().unwrap()
}

#[test]
fn existing_file_is_canonicalized_and_symlink_is_resolved() {
    let fixture = Fixture::new();
    let grant = fixture.mkdir("grant");
    let file = fixture.file("grant/art.svg");
    let alias = fixture.path("alias.svg");
    std::os::unix::fs::symlink(&file, &alias).unwrap();
    let policy = fixture.policy(&grant);

    assert_eq!(
        policy.resolve_existing(text(&alias)).unwrap(),
        fs::canonicalize(&file).unwrap()
    );
    assert!(policy.path_allowed(text(&alias)));
    assert_eq!(
        policy.resolve_granted_directory(text(&grant)).unwrap(),
        fs::canonicalize(grant).unwrap()
    );
}

#[test]
fn directory_boundary_does_not_match_collision_name_or_traversal_escape() {
    let fixture = Fixture::new();
    let grant = fixture.mkdir("grant");
    let collision = fixture.mkdir("grant-copy");
    let sibling = fixture.file("grant-copy/a.svg");
    let outside = fixture.file("outside.svg");
    let inside = fixture.file("grant/inside.svg");
    let policy = fixture.policy(&grant);

    assert!(policy.path_allowed(text(&inside)));
    assert!(!policy.path_allowed(text(&sibling)));
    let traversal = grant.join("../outside.svg");
    assert!(!policy.path_allowed(text(&traversal)));
    assert_eq!(
        policy.resolve_save_path(text(&outside)),
        Err(AccessError::NotGranted)
    );
    assert_eq!(
        policy.resolve_granted_directory(text(&collision)),
        Err(AccessError::NotGranted)
    );
}

#[test]
fn root_grant_covers_absolute_paths() {
    let fixture = Fixture::new();
    let file = fixture.file("root-target.svg");
    let policy = AccessPolicy::new(Some(OsString::from("/")), None);
    assert!(policy.path_allowed(text(&file)));
    assert_eq!(
        policy.resolve_save_path(text(&file)).unwrap(),
        fs::canonicalize(file).unwrap()
    );
}

#[test]
fn save_path_canonicalizes_parent_and_rejects_missing_parent_and_dot_leaf() {
    let fixture = Fixture::new();
    let grant = fixture.mkdir("grant");
    let policy = fixture.policy(&grant);
    let new_path = grant.join("new.svg");
    assert_eq!(
        policy.resolve_save_path(text(&new_path)).unwrap(),
        fs::canonicalize(&grant).unwrap().join("new.svg")
    );
    assert_eq!(
        policy.resolve_save_path(&format!("{}/", new_path.display())),
        Err(AccessError::InvalidPath)
    );
    assert_eq!(
        policy.resolve_save_path(text(&grant.join("missing/new.svg"))),
        Err(AccessError::NotFound)
    );
    assert_eq!(
        policy.resolve_save_path(text(&grant)),
        Err(AccessError::NotRegularFile)
    );
}

#[test]
fn malformed_and_oversized_paths_are_denied() {
    let fixture = Fixture::new();
    let grant = fixture.mkdir("grant");
    let file = fixture.file("grant/a.svg");
    let policy = fixture.policy(&grant);
    let too_long = "x".repeat(PATH_CAP);

    assert_eq!(policy.resolve_existing(""), Err(AccessError::InvalidPath));
    assert_eq!(
        policy.resolve_existing("x\0y"),
        Err(AccessError::InvalidPath)
    );
    assert_eq!(
        policy.resolve_existing(&too_long),
        Err(AccessError::InvalidPath)
    );
    assert!(!policy.path_allowed(&too_long));
    assert_eq!(
        policy.resolve_existing(text(&grant)),
        Err(AccessError::NotRegularFile)
    );
    assert_eq!(
        policy.resolve_existing(text(&fixture.path("absent.svg"))),
        Err(AccessError::NotFound)
    );
    assert!(policy.path_allowed(text(&file)));
}

#[test]
fn environment_grants_are_colon_delimited_bounded_and_can_grant_root() {
    let fixture = Fixture::new();
    let first = fixture.mkdir("first");
    let second = fixture.mkdir("second");
    let file = fixture.file("second/picture.svg");
    let policy = AccessPolicy::new(
        Some(OsString::from(format!(
            "{}:{}",
            first.display(),
            second.display()
        ))),
        None,
    );
    assert!(policy.path_allowed(text(&file)));

    let oversized = AccessPolicy::new(Some(OsString::from("x".repeat(8192))), None);
    assert!(!oversized.path_allowed(text(&file)));
    let empty_items = AccessPolicy::new(
        Some(OsString::from(format!(":{}:", second.display()))),
        None,
    );
    assert!(empty_items.path_allowed(text(&file)));
}

#[test]
fn config_grants_trim_comments_and_limit_processing_to_128_chunks() {
    let fixture = Fixture::new();
    let grant = fixture.mkdir("configured");
    let file = fixture.file("configured/file.svg");
    let home =
        fixture.config_home(format!("  # comment\r\n  {}  \r\n", grant.display()).as_bytes());
    let policy = AccessPolicy::new(None, Some(home));
    assert!(policy.path_allowed(text(&file)));

    let late_home = fixture.config_home(format!("{}\n", "# comment\n".repeat(128)).as_bytes());
    let config = late_home.join(".config/vexel/mcp-allowed-dirs.txt");
    fs::OpenOptions::new()
        .append(true)
        .open(config)
        .unwrap()
        .write_all(format!("{}\n", grant.display()).as_bytes())
        .unwrap();
    let late_policy = AccessPolicy::new(None, Some(late_home));
    assert!(!late_policy.path_allowed(text(&file)));
}

#[test]
fn oversized_config_record_is_processed_in_bounded_chunks_without_granting() {
    let fixture = Fixture::new();
    let grant = fixture.mkdir("configured");
    let file = fixture.file("configured/file.svg");
    let home = fixture.config_home(&vec![b'x'; 2048]);
    let policy = AccessPolicy::new(None, Some(home));

    assert!(!policy.path_allowed(text(&file)));
    assert_eq!(
        policy.resolve_granted_directory(text(&grant)),
        Err(AccessError::NotGranted)
    );
}

#[test]
fn config_rejects_symlink_insecure_mode_and_nonregular_targets() {
    let fixture = Fixture::new();
    let grant = fixture.mkdir("configured");
    let file = fixture.file("configured/file.svg");
    let home = fixture.config_home(format!("{}\n", grant.display()).as_bytes());
    let config = home.join(".config/vexel/mcp-allowed-dirs.txt");

    let insecure = AccessPolicy::new(None, Some(home.clone()));
    assert!(insecure.path_allowed(text(&file)));
    fs::set_permissions(&config, fs::Permissions::from_mode(0o620)).unwrap();
    assert!(!AccessPolicy::new(None, Some(home.clone())).path_allowed(text(&file)));
    fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
    let target = fixture.file("config-target.txt");
    fs::remove_file(&config).unwrap();
    std::os::unix::fs::symlink(target, &config).unwrap();
    assert!(!AccessPolicy::new(None, Some(home)).path_allowed(text(&file)));

    let directory_home = fixture.mkdir("directory-home");
    let config_parent = directory_home.join(".config/vexel");
    fs::create_dir_all(&config_parent).unwrap();
    fs::create_dir(config_parent.join("mcp-allowed-dirs.txt")).unwrap();
    assert!(!AccessPolicy::new(None, Some(directory_home)).path_allowed(text(&file)));
}

#[test]
fn environment_grant_succeeds_even_when_config_is_rejected() {
    let fixture = Fixture::new();
    let env_grant = fixture.mkdir("env-grant");
    let config_grant = fixture.mkdir("config-grant");
    let env_file = fixture.file("env-grant/a.svg");
    let config_file = fixture.file("config-grant/b.svg");
    let home = fixture.config_home(format!("{}\n", config_grant.display()).as_bytes());
    let config = home.join(".config/vexel/mcp-allowed-dirs.txt");
    fs::set_permissions(&config, fs::Permissions::from_mode(0o622)).unwrap();
    let policy = AccessPolicy::new(Some(env_grant.as_os_str().to_owned()), Some(home));

    assert!(policy.path_allowed(text(&env_file)));
    assert!(!policy.path_allowed(text(&config_file)));
}

#[test]
fn raw_non_utf8_existing_and_new_paths_resolve_inside_granted_directory() {
    let fixture = Fixture::new();
    let grant = fixture.mkdir("raw-grant");
    let raw_directory = OsString::from_vec(b"directory-\xff".to_vec());
    let directory = grant.join(&raw_directory);
    let existing_name = OsString::from_vec(b"existing-\xfe.svg".to_vec());
    let existing = directory.join(existing_name);
    let new_name = OsString::from_vec(b"new-\xfd.svg".to_vec());
    let new_target = directory.join(new_name);
    let policy = fixture.policy(&grant);

    match fs::create_dir(&directory) {
        Ok(()) => match fs::write(&existing, b"raw path fixture") {
            Ok(()) => {
                assert_eq!(
                    policy.resolve_existing(&existing).unwrap(),
                    fs::canonicalize(&existing).unwrap()
                );
                assert!(policy.path_allowed(&existing));
                assert_eq!(
                    policy.resolve_granted_directory(&directory).unwrap(),
                    fs::canonicalize(&directory).unwrap()
                );
                assert_eq!(
                    policy.resolve_save_path(&new_target).unwrap(),
                    fs::canonicalize(&directory)
                        .unwrap()
                        .join(new_target.file_name().unwrap())
                );
                assert!(!new_target.exists());

                assert_outside_grant_rules(&fixture, &policy);
                let outside_raw = fixture
                    .path("raw-outside")
                    .join(OsString::from_vec(b"denied-\xff.svg".to_vec()));
                fs::write(&outside_raw, b"outside raw fixture").unwrap();
                assert_eq!(
                    policy.resolve_save_path(&outside_raw),
                    Err(AccessError::NotGranted)
                );
                assert!(!policy.path_allowed(&outside_raw));
            }
            Err(error) if is_native_invalid_byte_rejection(&error) => {
                assert_native_rejection_behavior(
                    &fixture,
                    &grant,
                    &directory,
                    &existing,
                    &new_target,
                    &policy,
                    true,
                );
            }
            Err(error) => panic!("unexpected raw filename fixture error: {error}"),
        },
        Err(error) if is_native_invalid_byte_rejection(&error) => {
            assert_native_rejection_behavior(
                &fixture,
                &grant,
                &directory,
                &existing,
                &new_target,
                &policy,
                false,
            );
        }
        Err(error) => panic!("unexpected raw directory fixture error: {error}"),
    }
}

fn is_native_invalid_byte_rejection(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::PermissionDenied || error.raw_os_error() == Some(92)
}

fn assert_native_rejection_behavior(
    fixture: &Fixture,
    grant: &Path,
    directory: &Path,
    existing: &Path,
    new_target: &Path,
    policy: &AccessPolicy,
    directory_created: bool,
) {
    let existing_bytes = existing.as_os_str().as_bytes();
    assert!(existing_bytes.contains(&0xff));
    assert!(existing_bytes.contains(&0xfe));
    assert!(new_target.as_os_str().as_bytes().contains(&0xfd));
    assert!(!existing_bytes.contains(&0));
    assert_native_path_rejection(existing);
    assert_native_path_rejection(new_target);
    if directory_created {
        assert_eq!(
            policy.resolve_granted_directory(directory).unwrap(),
            fs::canonicalize(directory).unwrap()
        );
    } else {
        assert_native_path_rejection(directory);
        assert_eq!(
            policy.resolve_granted_directory(directory),
            Err(AccessError::NotFound)
        );
    }

    // A valid UTF-8 path formed by lossy replacement is deliberately present.
    // Raw inputs must be denied by the native filesystem result, never rewritten
    // to this different existing file.
    let lossy_existing = PathBuf::from(String::from_utf8_lossy(existing_bytes).into_owned());
    let lossy_directory = lossy_existing.parent().unwrap();
    fs::create_dir_all(lossy_directory).unwrap();
    fs::write(&lossy_existing, b"lossy replacement fixture").unwrap();
    let lossy_new_target = lossy_directory.join("new-�.svg");

    assert_eq!(
        policy.resolve_existing(existing),
        Err(AccessError::NotFound)
    );
    assert!(!policy.path_allowed(existing));
    assert_save_resolution_matches_native_lookup(policy, existing, directory);
    assert_save_resolution_matches_native_lookup(policy, new_target, directory);

    assert_eq!(
        policy.resolve_existing(&lossy_existing).unwrap(),
        fs::canonicalize(&lossy_existing).unwrap()
    );
    assert!(policy.path_allowed(&lossy_existing));
    assert_eq!(
        policy.resolve_save_path(&lossy_new_target).unwrap(),
        fs::canonicalize(lossy_directory).unwrap().join("new-�.svg")
    );
    assert_eq!(
        policy.resolve_granted_directory(lossy_directory).unwrap(),
        fs::canonicalize(lossy_directory).unwrap()
    );

    let mut nul_bytes = existing_bytes.to_vec();
    nul_bytes.push(0);
    let nul_path = OsString::from_vec(nul_bytes);
    assert_eq!(
        policy.resolve_existing(&nul_path),
        Err(AccessError::InvalidPath)
    );
    assert!(!policy.path_allowed(&nul_path));
    assert_eq!(
        policy.resolve_save_path(&nul_path),
        Err(AccessError::InvalidPath)
    );
    assert_eq!(
        policy.resolve_granted_directory(&nul_path),
        Err(AccessError::InvalidPath)
    );

    assert_outside_grant_rules(fixture, policy);
    assert_outside_raw_rejection(fixture, policy);
    assert!(directory.starts_with(grant));
}

fn assert_native_path_rejection(path: &Path) {
    let canonical_error =
        fs::canonicalize(path).expect_err("invalid-byte path unexpectedly resolved");
    assert!(
        is_native_invalid_byte_rejection(&canonical_error)
            || canonical_error.kind() == std::io::ErrorKind::NotFound,
        "unexpected canonicalize rejection: {canonical_error}"
    );
    let stat_error =
        fs::symlink_metadata(path).expect_err("invalid-byte path unexpectedly statted");
    assert!(
        is_native_invalid_byte_rejection(&stat_error)
            || stat_error.kind() == std::io::ErrorKind::NotFound,
        "unexpected stat rejection: {stat_error}"
    );
}

fn assert_save_resolution_matches_native_lookup(policy: &AccessPolicy, path: &Path, parent: &Path) {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match fs::canonicalize(parent) {
                Ok(canonical_parent) => {
                    let expected =
                        canonical_parent.join(path.file_name().expect("save path has a leaf"));
                    assert_eq!(policy.resolve_save_path(path).unwrap(), expected);
                }
                Err(_) => assert_eq!(policy.resolve_save_path(path), Err(AccessError::NotFound)),
            }
        }
        Err(_) => assert_eq!(policy.resolve_save_path(path), Err(AccessError::NotFound)),
        Ok(_) => panic!("fixture target unexpectedly exists: {path:?}"),
    }
}

fn assert_outside_grant_rules(fixture: &Fixture, policy: &AccessPolicy) {
    let outside_directory = fixture.mkdir("raw-outside");
    let outside_file = fixture.file("raw-outside/outside.svg");
    assert_eq!(
        policy.resolve_existing(&outside_file).unwrap(),
        fs::canonicalize(&outside_file).unwrap()
    );
    assert!(!policy.path_allowed(&outside_file));
    assert_eq!(
        policy.resolve_save_path(&outside_file),
        Err(AccessError::NotGranted)
    );
    assert_eq!(
        policy.resolve_granted_directory(&outside_directory),
        Err(AccessError::NotGranted)
    );
}

fn assert_outside_raw_rejection(fixture: &Fixture, policy: &AccessPolicy) {
    let outside_directory = fixture.path("raw-outside");
    let raw_outside_new = outside_directory.join(OsString::from_vec(b"new-\xfe.svg".to_vec()));
    let raw_outside = outside_directory.join(OsString::from_vec(b"outside-\xff.svg".to_vec()));
    assert_native_path_rejection(&raw_outside);
    assert_native_path_rejection(&raw_outside_new);
    assert_eq!(
        policy.resolve_existing(&raw_outside),
        Err(AccessError::NotFound)
    );
    assert!(!policy.path_allowed(&raw_outside));
    assert_eq!(
        policy.resolve_save_path(&raw_outside_new),
        Err(AccessError::NotGranted)
    );
    assert_eq!(
        policy.resolve_granted_directory(&raw_outside),
        Err(AccessError::NotFound)
    );
}

#[test]
fn raw_path_input_and_canonical_output_accept_exact_1023_byte_boundary() {
    let fixture = Fixture::new();
    let grant = fixture.path("boundary-grant");
    fs::create_dir(&grant).unwrap();
    let mut current = grant.clone();
    let mut remaining = PATH_CAP - 1 - current.as_os_str().as_bytes().len();
    assert!(remaining >= 2);
    while remaining > 202 {
        let component = OsString::from_vec(vec![b'd'; 200]);
        current.push(component);
        fs::create_dir(&current).unwrap();
        remaining -= 201;
    }
    let leaf = OsString::from_vec(vec![b'f'; remaining - 1]);
    let boundary_file = current.join(leaf);
    fs::write(&boundary_file, b"boundary fixture").unwrap();
    assert_eq!(boundary_file.as_os_str().as_bytes().len(), PATH_CAP - 1);

    let policy = fixture.policy(&grant);
    let resolved = policy.resolve_existing(&boundary_file).unwrap();
    assert_eq!(resolved.as_os_str().as_bytes().len(), PATH_CAP - 1);
    assert_eq!(resolved, fs::canonicalize(&boundary_file).unwrap());
    assert!(policy.path_allowed(&boundary_file));

    let mut overlong = boundary_file.as_os_str().to_os_string();
    overlong.push("x");
    assert_eq!(overlong.as_os_str().as_bytes().len(), PATH_CAP);
    assert_eq!(
        policy.resolve_existing(&overlong),
        Err(AccessError::InvalidPath)
    );
    assert_eq!(
        policy.resolve_save_path(&overlong),
        Err(AccessError::InvalidPath)
    );
    assert!(!policy.path_allowed(&overlong));
    assert_eq!(
        policy.resolve_granted_directory(&overlong),
        Err(AccessError::InvalidPath)
    );

    let mut nul_bytes = vec![b'x'; PATH_CAP - 1];
    nul_bytes[PATH_CAP - 2] = 0;
    let nul_path = OsString::from_vec(nul_bytes);
    assert_eq!(
        policy.resolve_existing(&nul_path),
        Err(AccessError::InvalidPath)
    );
    assert_eq!(
        policy.resolve_save_path(&nul_path),
        Err(AccessError::InvalidPath)
    );
    assert!(!policy.path_allowed(&nul_path));
    assert_eq!(
        policy.resolve_granted_directory(&nul_path),
        Err(AccessError::InvalidPath)
    );
}
