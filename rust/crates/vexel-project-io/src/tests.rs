use super::*;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use vexel_platform_macos::{AtomicFileCreateNewHooks, AtomicFileReplaceHooks};
use vexel_project::{Asset, BundleInput, Error as BundleError};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

fn input<'a>(assets: &'a [Asset<'a>]) -> BundleInput<'a> {
    BundleInput {
        manifest_json: br#"{"title":"Draft"}"#,
        scene_svg: b"<svg xmlns='urn:svg' xmlns:x='urn:unknown'><x:data/></svg>",
        timeline_json: br#"{"tracks":[]}"#,
        assets,
    }
}

fn fixture() -> PathBuf {
    let root = std::env::current_dir().unwrap().join(".cache/project-io-tests");
    fs::create_dir_all(&root).unwrap();
    let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
    let path = root.join(format!("{}-{id}", std::process::id()));
    fs::create_dir(&path).unwrap();
    path
}

fn path_bytes(path: &std::path::Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

#[test]
fn file_save_creates_replaces_and_preserves_payload_and_errors() {
    let dir = fixture();
    let path = dir.join("drawing.vexel");
    let bytes = path_bytes(&path);
    let asset_data = [0, 0x14, 0xff];
    let assets = [Asset {
        name: b"palette.bin",
        media_type: b"application/octet-stream",
        data: &asset_data,
    }];
    let mut project = input(&assets);

    let saved = project_file_save_as(&bytes, &project);
    assert_eq!(saved.error, FileError::Ok);
    assert!(saved.published && saved.directory_fsynced);
    assert!(saved.bytes_written > vexel_project::HEADER_BYTES);
    assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
    let actual = fs::read(&path).unwrap();
    let decoded = vexel_project::decode(&actual).unwrap();
    assert_eq!(decoded.manifest_json, project.manifest_json);
    assert_eq!(decoded.scene_svg, project.scene_svg);
    assert_eq!(decoded.timeline_json, project.timeline_json);
    assert_eq!(decoded.asset_at(0).unwrap().name, assets[0].name);
    assert_eq!(decoded.asset_at(0).unwrap().data, asset_data);

    project.scene_svg = b"<svg><rect width='20'/></svg>";
    let replaced = project_file_save_as(&bytes, &project);
    assert_eq!(replaced.error, FileError::Ok);
    assert!(replaced.published && replaced.directory_fsynced);
    let replacement_bytes = fs::read(&path).unwrap();
    let decoded = vexel_project::decode(&replacement_bytes).unwrap();
    assert_eq!(decoded.scene_svg, project.scene_svg);
    assert_eq!(decoded.timeline_json, project.timeline_json);

    let invalid_asset = [Asset {
        name: b"../escape",
        media_type: b"application/octet-stream",
        data: &asset_data,
    }];
    let invalid = BundleInput { assets: &invalid_asset, ..project };
    let rejected = project_file_save_as(&bytes, &invalid);
    assert_eq!(rejected.error, FileError::InvalidArgument);
    assert_eq!(rejected.bundle_error, BundleError::InvalidArgument);
    assert!(!rejected.published);
    assert_eq!(vexel_project::decode(&fs::read(&path).unwrap()).unwrap().scene_svg, project.scene_svg);

    let directory_target = dir.join("directory");
    fs::create_dir(&directory_target).unwrap();
    assert_eq!(project_file_save_as(&path_bytes(&directory_target), &project).error, FileError::Io);
    assert!(directory_target.is_dir());
    let too_long = vec![b'x'; MAX_ATOMIC_FILE_PATH_BYTES + 1];
    assert_eq!(project_file_save_as(&too_long, &project).error, FileError::PathLimit);

    let encoded = vexel_project::encode(&project).unwrap();
    let created = dir.join("encoded.vexel");
    let created_path = path_bytes(&created);
    let saved = project_file_save_encoded(&created_path, &encoded);
    assert_eq!(saved.error, FileError::Ok);
    assert_eq!(fs::read(&created).unwrap(), encoded);
    assert_eq!(project_file_save_encoded(&created_path, b"bad").error, FileError::InvalidArgument);
    let symlink_target = dir.join("link.vexel");
    std::os::unix::fs::symlink(&created, &symlink_target).unwrap();
    assert_eq!(project_file_save_encoded(&path_bytes(&symlink_target), &encoded).error, FileError::Io);
    assert_eq!(fs::read(&created).unwrap(), encoded);
    fs::remove_file(symlink_target).unwrap();
    fs::remove_file(created).unwrap();

    struct RaceCreate { path: PathBuf }
    impl AtomicFileCreateNewHooks for RaceCreate {
        fn before_publish(&mut self, _: &std::path::Path, _: &std::path::Path) -> io::Result<()> {
            fs::write(&self.path, b"competing writer")
        }
    }
    let raced = dir.join("raced.vexel");
    let raced_path = path_bytes(&raced);
    let collision = super::file::project_file_save_encoded_with_hooks(
        &raced_path, &encoded, None, Some(&mut RaceCreate { path: raced.clone() }),
    );
    assert_eq!(collision.error, FileError::AlreadyExists);
    assert_eq!(collision.create_error, vexel_platform_macos::AtomicFileCreateNewError::Exists);
    assert_eq!(fs::read(&raced).unwrap(), b"competing writer");
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 3, "raced create removes its temporary file");
    fs::remove_file(raced).unwrap();
    fs::remove_file(path).unwrap();
    fs::remove_dir(directory_target).unwrap();
    fs::remove_dir(dir).unwrap();
}

#[test]
fn file_postcommit_durability_reports_published_state() {
    struct FailFileSync;
    impl AtomicFileReplaceHooks for FailFileSync {
        fn before_sync_file(&mut self, _: &std::fs::File) -> io::Result<()> {
            Err(io::Error::other("injected precommit file sync failure"))
        }
    }
    struct FailDirectorySync;
    impl AtomicFileReplaceHooks for FailDirectorySync {
        fn before_sync_directory(&mut self, _: &std::path::Path) -> io::Result<()> {
            Err(io::Error::other("injected directory sync failure"))
        }
    }
    let dir = fixture();
    let path = dir.join("durability.vexel");
    let bytes = path_bytes(&path);
    fs::write(&path, b"old bytes").unwrap();
    let project = input(&[]);
    let encoded = vexel_project::encode(&project).unwrap();
    let precommit = super::file::project_file_save_encoded_with_hooks(
        &bytes, &encoded, Some(&mut FailFileSync), None,
    );
    assert_eq!(precommit.error, FileError::Io);
    assert_eq!(precommit.bytes_written, encoded.len());
    assert!(!precommit.published && !precommit.directory_fsynced);
    assert_eq!(fs::read(&path).unwrap(), b"old bytes");
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 1, "failed transaction removes its temporary file");
    let saved = super::file::project_file_save_encoded_with_hooks(
        &bytes, &encoded, Some(&mut FailDirectorySync), None,
    );
    assert_eq!(saved.error, FileError::Durability);
    assert_eq!(saved.replace_error, vexel_platform_macos::AtomicFileReplaceError::Durability);
    assert!(saved.published);
    assert!(!saved.directory_fsynced);
    assert_eq!(fs::read(&path).unwrap(), encoded);
    fs::remove_file(path).unwrap();
    fs::remove_dir(dir).unwrap();
}

#[test]
fn encoded_decoder_failure_mapping_preserves_all_error_classes_and_flags() {
    let cases = [
        (BundleError::OutOfMemory, FileError::OutOfMemory),
        (BundleError::Limit, FileError::Limit),
        (BundleError::Corrupt, FileError::InvalidArgument),
        (BundleError::UnsupportedVersion, FileError::InvalidArgument),
    ];
    for (bundle_error, file_error) in cases {
        let result = super::file::bundle_decode_failure(bundle_error);
        assert_eq!(result.error, file_error);
        assert_eq!(result.bundle_error, bundle_error);
        assert_eq!(result.replace_error, vexel_platform_macos::AtomicFileReplaceError::Ok);
        assert_eq!(result.create_error, vexel_platform_macos::AtomicFileCreateNewError::Ok);
        assert_eq!(result.bytes_written, 0);
        assert!(!result.published);
        assert!(!result.directory_fsynced);
    }
}

#[test]
fn checksum_corrupt_encoded_save_is_rejected_before_publication() {
    let dir = fixture();
    let missing = dir.join("corrupt-missing.vexel");
    let missing_bytes = path_bytes(&missing);
    let mut encoded = vexel_project::encode(&input(&[])).unwrap();
    let manifest_length = u32::from_le_bytes(encoded[16..20].try_into().unwrap()) as usize;
    encoded[vexel_project::HEADER_BYTES + manifest_length] ^= 1;

    let missing_result = project_file_save_encoded(&missing_bytes, &encoded);
    assert_eq!(missing_result.error, FileError::InvalidArgument);
    assert_eq!(missing_result.bundle_error, BundleError::Corrupt);
    assert_eq!(missing_result.bytes_written, 0);
    assert!(!missing_result.published);
    assert!(!missing_result.directory_fsynced);
    assert_eq!(missing_result.replace_error, vexel_platform_macos::AtomicFileReplaceError::Ok);
    assert_eq!(missing_result.create_error, vexel_platform_macos::AtomicFileCreateNewError::Ok);
    assert!(!missing.exists());

    let existing = dir.join("corrupt-existing.vexel");
    let existing_bytes = path_bytes(&existing);
    let seed = b"known destination bytes";
    fs::write(&existing, seed).unwrap();
    let entries_before = fs::read_dir(&dir).unwrap().count();
    let existing_result = project_file_save_encoded(&existing_bytes, &encoded);
    assert_eq!(existing_result.error, FileError::InvalidArgument);
    assert_eq!(existing_result.bundle_error, BundleError::Corrupt);
    assert_eq!(existing_result.bytes_written, 0);
    assert!(!existing_result.published);
    assert!(!existing_result.directory_fsynced);
    assert_eq!(existing_result.replace_error, vexel_platform_macos::AtomicFileReplaceError::Ok);
    assert_eq!(existing_result.create_error, vexel_platform_macos::AtomicFileCreateNewError::Ok);
    assert_eq!(fs::read(&existing).unwrap(), seed);
    assert_eq!(fs::read_dir(&dir).unwrap().count(), entries_before,
        "validation failure creates no temporary file");

    fs::remove_file(existing).unwrap();
    fs::remove_dir(dir).unwrap();
}

fn recovery_input<'a>(project: BundleInput<'a>) -> RecoveryInput<'a> {
    RecoveryInput {
        source_path: b"/Users/example/Walk.vexel",
        source_size: 123_456,
        source_modified_seconds: 1_791_840_000,
        source_modified_nanoseconds: 456,
        captured_time_nanoseconds: 99_887_766,
        project,
    }
}

#[test]
fn recovery_v1_round_trip_header_crc_path_and_error_precedence() {
    let asset_data = [0, 0x34, 0xff];
    let assets = [Asset { name: b"frame.svg", media_type: b"image/svg+xml", data: &asset_data }];
    let project = BundleInput {
        manifest_json: br#"{"title":"Walk"}"#,
        scene_svg: b"<svg><path id='hero'/></svg>",
        timeline_json: br#"{"version":1,"tracks":[]}"#,
        assets: &assets,
    };
    let input = recovery_input(project);
    let measured = project_recovery_encoded_size(&input);
    assert_eq!(measured.error, RecoveryError::Ok);
    assert!(measured.encoded_size > RECOVERY_HEADER_BYTES);
    let mut encoded = vec![0; measured.encoded_size];
    let wrote = project_recovery_encode_into(&input, &mut encoded);
    assert_eq!(wrote.error, RecoveryError::Ok);
    assert_eq!(wrote.encoded_size, measured.encoded_size);
    assert_eq!(&encoded[..8], b"VXRECOV\x01");
    assert_eq!(u32::from_le_bytes(encoded[8..12].try_into().unwrap()), RECOVERY_VERSION);
    assert_eq!(u32::from_le_bytes(encoded[12..16].try_into().unwrap()) as usize, RECOVERY_HEADER_BYTES);
    assert_eq!(&encoded[20..24], &[0; 4]);
    assert_eq!(&encoded[60..64], &crc32_for_test(&encoded).to_le_bytes());

    let view = project_recovery_decode(&encoded, Some(input.source_path)).unwrap();
    // Successful C decode reports the encoded capsule length in its result.
    // Rust's borrowed view omits that redundant field; the validated input
    // length remains available to callers and is exactly this encoded length.
    assert_eq!(encoded.len(), RECOVERY_HEADER_BYTES + view.source_path.len() + view.project.encoded.len());
    assert_eq!(view.source_path, input.source_path);
    assert_eq!(view.source_size, input.source_size);
    assert_eq!(view.source_modified_seconds, input.source_modified_seconds);
    assert_eq!(view.source_modified_nanoseconds, input.source_modified_nanoseconds);
    assert_eq!(view.captured_time_nanoseconds, input.captured_time_nanoseconds);
    assert_eq!(view.project.manifest_json, project.manifest_json);
    assert_eq!(view.project.scene_svg, project.scene_svg);
    assert_eq!(view.project.timeline_json, project.timeline_json);
    assert_eq!(view.project.asset_count, 1);
    assert_eq!(view.project.asset_at(0).unwrap().name, assets[0].name);
    assert_eq!(view.project.asset_at(0).unwrap().data, asset_data);
    assert_eq!(project_recovery_decode(&encoded, Some(b"/Users/example/Other.vexel")).unwrap_err().error,
        RecoveryError::PathMismatch);

    // Corrupt the inner bundle while repairing the outer capsule checksums.
    // The decoder must pass envelope validation and identity comparison
    // before it reports the embedded project's precise failure.
    let mut invalid_project = encoded.clone();
    let bundle_start = RECOVERY_HEADER_BYTES + input.source_path.len();
    invalid_project[bundle_start + 40] ^= 1;
    repair_recovery_checksums(&mut invalid_project);
    let failure = project_recovery_decode(&invalid_project, Some(input.source_path)).unwrap_err();
    assert_eq!(failure.error, RecoveryError::InvalidProject);
    assert_eq!(failure.bundle_error, BundleError::Corrupt);
    assert_eq!(failure.error_offset, vexel_project::HEADER_BYTES);
    let identity_precedence = project_recovery_decode(
        &invalid_project, Some(b"/Users/example/Other.vexel"),
    ).unwrap_err();
    assert_eq!(identity_precedence.error, RecoveryError::PathMismatch);
    assert_eq!(identity_precedence.bundle_error, BundleError::Ok);
    assert_eq!(identity_precedence.error_offset, 0);
    assert_eq!(project_recovery_decode(&encoded[..encoded.len() - 1], None).unwrap_err().error,
        RecoveryError::Corrupt);

    let path_offset = RECOVERY_HEADER_BYTES + 2;
    let mut damaged = encoded.clone();
    damaged[path_offset] ^= 1;
    assert_eq!(project_recovery_decode(&damaged, None).unwrap_err().error, RecoveryError::Corrupt);
    damaged.clone_from(&encoded);
    damaged[8] = 2;
    assert_eq!(project_recovery_decode(&damaged, None).unwrap_err().error,
        RecoveryError::UnsupportedVersion);
    damaged.clone_from(&encoded);
    damaged.push(0);
    assert_eq!(project_recovery_decode(&damaged, None).unwrap_err().error, RecoveryError::Corrupt);
    damaged.clone_from(&encoded);
    damaged[20] = 1;
    assert_eq!(project_recovery_decode(&damaged, None).unwrap_err().error, RecoveryError::Corrupt);
    damaged.clone_from(&encoded);
    damaged[24] ^= 1;
    assert_eq!(project_recovery_decode(&damaged, None).unwrap_err().error, RecoveryError::Corrupt);
    damaged.clone_from(&encoded);
    damaged[44..52].fill(0xff);
    assert_eq!(project_recovery_decode(&damaged, None).unwrap_err().error, RecoveryError::Corrupt);

    let mut invalid = input;
    invalid.source_modified_nanoseconds = 1_000_000_000;
    assert_eq!(project_recovery_encoded_size(&invalid).error, RecoveryError::InvalidArgument);
    invalid.source_modified_nanoseconds = 0;
    let mut long_path = vec![b'p'; RECOVERY_MAX_PATH_BYTES + 1];
    long_path[0] = b'/';
    invalid.source_path = &long_path;
    assert_eq!(project_recovery_encoded_size(&invalid).error, RecoveryError::InvalidArgument);
    let mut too_small = vec![0; measured.encoded_size - 1];
    assert_eq!(project_recovery_encode_into(&input, &mut too_small).error,
        RecoveryError::InvalidArgument);
}

#[test]
fn recovery_header_matches_frozen_v1_bytes_and_independent_crc_vector() {
    let input = RecoveryInput {
        source_path: b"/tmp/g.vexel",
        source_size: 13,
        source_modified_seconds: 20,
        source_modified_nanoseconds: 30,
        captured_time_nanoseconds: 40,
        project: BundleInput {
            manifest_json: b"{}",
            scene_svg: b"<svg/>",
            timeline_json: b"{}",
            assets: &[],
        },
    };
    let mut encoded = vec![0; project_recovery_encoded_size(&input).encoded_size];
    assert_eq!(project_recovery_encode_into(&input, &mut encoded).error, RecoveryError::Ok);
    const EXPECTED_HEADER: [u8; 72] = [
        0x56,0x58,0x52,0x45,0x43,0x4f,0x56,0x01,0x01,0x00,0x00,0x00,0x48,0x00,0x00,0x00,
        0x0c,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x0d,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
        0x14,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x1e,0x00,0x00,0x00,0x42,0x00,0x00,0x00,
        0x00,0x00,0x00,0x00,0x4c,0x00,0x46,0xc5,0x72,0x10,0xd6,0xa4,0xb0,0x3c,0x4c,0x47,
        0x28,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
    ];
    assert_eq!(&encoded[..RECOVERY_HEADER_BYTES], &EXPECTED_HEADER);
    assert_eq!(project_recovery_decode(&encoded, Some(b"/tmp/g.vexel")).unwrap().project.scene_svg,
        b"<svg/>");
}

#[test]
fn recovery_maximum_total_size_boundary_is_checked_without_large_allocation() {
    assert_eq!(
        RECOVERY_MAX_BYTES,
        RECOVERY_HEADER_BYTES + RECOVERY_MAX_PATH_BYTES + vexel_project::MAX_BYTES,
    );
    let mut absolute_path = vec![b'p'; RECOVERY_MAX_PATH_BYTES];
    absolute_path[0] = b'/';
    let input = RecoveryInput {
        source_path: &absolute_path,
        ..recovery_input(BundleInput {
            manifest_json: b"{}",
            scene_svg: b"<svg/>",
            timeline_json: b"{}",
            assets: &[],
        })
    };
    // The recovery path limit is checked independently of total-size
    // arithmetic, and its largest accepted value remains below the cap.
    let measured = project_recovery_encoded_size(&input);
    assert_eq!(measured.error, RecoveryError::Ok);
    assert!(measured.encoded_size <= RECOVERY_MAX_BYTES);

    // A capsule may declare the exact maximum total length, but a short
    // backing slice is rejected before any payload access. One byte beyond
    // the bundle ceiling is rejected at the same structural boundary.
    let mut declared_max = vec![0; RECOVERY_HEADER_BYTES];
    declared_max[..8].copy_from_slice(b"VXRECOV\x01");
    declared_max[8..12].copy_from_slice(&RECOVERY_VERSION.to_le_bytes());
    declared_max[12..16].copy_from_slice(&(RECOVERY_HEADER_BYTES as u32).to_le_bytes());
    declared_max[16..20].copy_from_slice(&(RECOVERY_MAX_PATH_BYTES as u32).to_le_bytes());
    declared_max[44..52].copy_from_slice(&(vexel_project::MAX_BYTES as u64).to_le_bytes());
    assert_eq!(
        project_recovery_decode(&declared_max, None).unwrap_err().error,
        RecoveryError::Corrupt,
    );
    declared_max[44..52].copy_from_slice(&((vexel_project::MAX_BYTES + 1) as u64).to_le_bytes());
    assert_eq!(
        project_recovery_decode(&declared_max, None).unwrap_err().error,
        RecoveryError::Corrupt,
    );
}

#[test]
fn recovery_atomic_store_preserves_source_and_prior_file_on_error() {
    let dir = fixture();
    let source = dir.join("source.vexel");
    let target = dir.join("recovery.vxr");
    let blocked = dir.join("blocked");
    fs::write(&source, b"source bytes").unwrap();
    fs::create_dir(&blocked).unwrap();
    let source_bytes = path_bytes(&source);
    let input = RecoveryInput { source_path: &source_bytes, ..recovery_input(input(&[])) };
    let target_bytes = path_bytes(&target);
    let saved = project_recovery_save(&target_bytes, &input);
    assert_eq!(saved.error, RecoveryError::Ok);
    assert!(saved.published && saved.directory_fsynced);
    assert!(saved.bytes_written > RECOVERY_HEADER_BYTES);
    assert_eq!(fs::read(&source).unwrap(), b"source bytes");
    let capsule = fs::read(&target).unwrap();
    let reopened = project_recovery_decode(&capsule, Some(&source_bytes)).unwrap();
    assert_eq!(reopened.source_path, source_bytes);
    assert_eq!(reopened.project.scene_svg, b"<svg xmlns='urn:svg' xmlns:x='urn:unknown'><x:data/></svg>");
    let replaced = project_recovery_save(&target_bytes, &input);
    assert_eq!(replaced.error, RecoveryError::Ok);
    assert!(replaced.published && replaced.directory_fsynced);
    let prior = fs::read(&target).unwrap();
    assert_eq!(project_recovery_save(&path_bytes(&blocked), &input).error, RecoveryError::Io);
    assert_eq!(fs::read(&target).unwrap(), prior);
    let link = dir.join("recovery-link.vxr");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    assert_eq!(project_recovery_save(&path_bytes(&link), &input).error, RecoveryError::Io);
    assert_eq!(fs::read(&target).unwrap(), prior);
    assert_eq!(project_recovery_save(&source_bytes, &input).error, RecoveryError::InvalidArgument);
    fs::remove_file(link).unwrap();
    fs::remove_file(&target).unwrap();
    fs::remove_file(source).unwrap();
    fs::remove_dir(blocked).unwrap();
    fs::remove_dir(dir).unwrap();
}

#[test]
fn recovery_postcommit_durability_and_no_replace_collision_are_reported() {
    struct FailDirectorySync;
    impl AtomicFileCreateNewHooks for FailDirectorySync {
        fn before_sync_directory(&mut self, _: &std::path::Path) -> io::Result<()> {
            Err(io::Error::other("injected directory sync failure"))
        }
    }
    struct RaceCreate { path: PathBuf }
    impl AtomicFileCreateNewHooks for RaceCreate {
        fn before_publish(&mut self, _: &std::path::Path, _: &std::path::Path) -> io::Result<()> {
            fs::write(&self.path, b"competing capsule")
        }
    }
    let dir = fixture();
    let target = dir.join("recovery.vxr");
    let target_bytes = path_bytes(&target);
    let input = recovery_input(input(&[]));
    let size = project_recovery_encoded_size(&input).encoded_size;
    let mut bytes = vec![0; size];
    assert_eq!(project_recovery_encode_into(&input, &mut bytes).error, RecoveryError::Ok);
    let saved = super::recovery::recovery_save_encoded_with_hooks(
        &target_bytes, &bytes, None, Some(&mut FailDirectorySync),
    );
    assert_eq!(saved.error, RecoveryError::Durability);
    assert!(saved.published);
    assert!(!saved.directory_fsynced);
    assert_eq!(fs::read(&target).unwrap(), bytes);
    fs::remove_file(target).unwrap();

    let raced = dir.join("raced.vxr");
    let raced_path = path_bytes(&raced);
    let collision = super::recovery::recovery_save_encoded_with_hooks(
        &raced_path, &bytes, None, Some(&mut RaceCreate { path: raced.clone() }),
    );
    assert_eq!(collision.error, RecoveryError::AlreadyExists);
    assert_eq!(collision.create_error, vexel_platform_macos::AtomicFileCreateNewError::Exists);
    assert!(!collision.published);
    assert_eq!(fs::read(&raced).unwrap(), b"competing capsule");
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 1, "raced recovery create removes its temporary file");
    fs::remove_file(raced).unwrap();
    fs::remove_dir(dir).unwrap();
}

fn crc32_for_test(encoded: &[u8]) -> u32 {
    let mut header = [0; RECOVERY_HEADER_BYTES];
    header.copy_from_slice(&encoded[..RECOVERY_HEADER_BYTES]);
    header[60..64].fill(0);
    let crc = crc32_update_for_test(u32::MAX, &header);
    !crc32_update_for_test(crc, &encoded[RECOVERY_HEADER_BYTES..])
}

fn repair_recovery_checksums(encoded: &mut [u8]) {
    let path_len = u32::from_le_bytes(encoded[16..20].try_into().unwrap()) as usize;
    let bundle_start = RECOVERY_HEADER_BYTES + path_len;
    let bundle = &encoded[bundle_start..];
    let bundle_crc = !crc32_update_for_test(u32::MAX, bundle);
    encoded[52..56].copy_from_slice(&bundle_crc.to_le_bytes());
    encoded[60..64].fill(0);
    let record_crc = crc32_for_test(encoded);
    encoded[60..64].copy_from_slice(&record_crc.to_le_bytes());
}

fn crc32_update_for_test(mut crc: u32, bytes: &[u8]) -> u32 {
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    crc
}

use std::os::unix::fs::PermissionsExt;
use vexel_platform_macos::MAX_ATOMIC_FILE_PATH_BYTES;
