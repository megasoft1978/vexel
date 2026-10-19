use super::*;
use crate::recovery::{RECOVERY_HEADER_BYTES, RecoveryInput, project_recovery_encode_into};
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use vexel_project::{Asset, BundleInput};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn path(label: &str) -> Vec<u8> {
    let path = std::env::current_dir()
        .unwrap()
        .join(".cache/recovery-discovery-tests");
    fs::create_dir_all(path).unwrap();
    std::env::current_dir()
        .unwrap()
        .join(".cache/recovery-discovery-tests")
        .join(format!(
            "{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
        .as_os_str()
        .as_bytes()
        .to_vec()
}

fn os_path(path: &[u8]) -> &Path {
    Path::new(std::ffi::OsStr::from_bytes(path))
}

fn write(path: &[u8], bytes: &[u8]) {
    let mut file = File::create(os_path(path)).unwrap();
    file.write_all(bytes).unwrap();
}

fn capsule(source: &[u8], source_stamp: Option<(u64, i64, u32)>) -> Vec<u8> {
    let metadata = fs::metadata(os_path(source)).unwrap();
    let (size, seconds, nanos) = source_stamp.unwrap_or((
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec() as u32,
    ));
    let assets = [Asset {
        name: b"keep.bin",
        media_type: b"application/octet-stream",
        data: b"opaque\0payload",
    }];
    let input = RecoveryInput {
        source_path: source,
        source_size: size,
        source_modified_seconds: seconds,
        source_modified_nanoseconds: nanos,
        captured_time_nanoseconds: 1,
        project: BundleInput {
            manifest_json: br#"{"title":"fixture"}"#,
            scene_svg: b"<svg/>keep",
            timeline_json: b"{}",
            assets: &assets,
        },
    };
    let size = crate::recovery::project_recovery_encoded_size(&input).encoded_size;
    let mut bytes = vec![0; size];
    assert_eq!(
        project_recovery_encode_into(&input, &mut bytes).error,
        RecoveryError::Ok
    );
    bytes
}

fn cleanup(paths: &[&[u8]]) {
    for path in paths {
        let _ = fs::remove_file(os_path(path));
    }
}

fn discovery_error(source: &[u8], capsule_path: &[u8], expected: RecoveryError) {
    assert!(
        matches!(project_recovery_discover(source, capsule_path), RecoveryDiscovery::Invalid { error } if error == expected)
    );
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn refresh_capsule_checksums(bytes: &mut [u8]) {
    let path_length = u32::from_le_bytes(bytes[16..20].try_into().unwrap()) as usize;
    let bundle_start = RECOVERY_HEADER_BYTES + path_length;
    put_u32(bytes, 52, crc32(&bytes[bundle_start..]));
    bytes[60..64].fill(0);
    let record_crc = crc32(bytes);
    put_u32(bytes, 60, record_crc);
}

#[test]
fn absent_matching_conflict_and_read_only_associations() {
    let source = path("source");
    let recovery = path("capsule");
    write(&source, b"source bytes are never read");
    assert!(matches!(
        project_recovery_discover(&source, &recovery),
        RecoveryDiscovery::Absent
    ));
    let encoded = capsule(&source, None);
    write(&recovery, &encoded);
    let source_before = fs::symlink_metadata(os_path(&source)).unwrap();
    let capsule_before = fs::symlink_metadata(os_path(&recovery)).unwrap();
    match project_recovery_discover(&source, &recovery) {
        RecoveryDiscovery::Matching { capsule_bytes } => assert_eq!(capsule_bytes, encoded),
        _ => panic!("expected matching capsule"),
    }
    let conflict = capsule(&source, Some((0, 0, 0)));
    write(&recovery, &conflict);
    match project_recovery_discover(&source, &recovery) {
        RecoveryDiscovery::Conflict { capsule_bytes } => assert_eq!(capsule_bytes, conflict),
        _ => panic!("expected source stamp conflict"),
    }
    let current = fs::metadata(os_path(&source)).unwrap();
    for stamp in [
        (
            current.len() + 1,
            current.mtime(),
            current.mtime_nsec() as u32,
        ),
        (
            current.len(),
            current.mtime() + 1,
            current.mtime_nsec() as u32,
        ),
    ] {
        let changed_stamp = capsule(&source, Some(stamp));
        write(&recovery, &changed_stamp);
        match project_recovery_discover(&source, &recovery) {
            RecoveryDiscovery::Conflict { capsule_bytes } => {
                assert_eq!(capsule_bytes, changed_stamp)
            }
            _ => panic!("expected source stamp conflict for changed size or mtime"),
        }
    }
    let source_after = fs::symlink_metadata(os_path(&source)).unwrap();
    let capsule_after = fs::symlink_metadata(os_path(&recovery)).unwrap();
    assert_eq!(
        (
            source_before.dev(),
            source_before.ino(),
            source_before.len()
        ),
        (source_after.dev(), source_after.ino(), source_after.len())
    );
    assert_eq!(
        (capsule_before.dev(), capsule_before.ino()),
        (capsule_after.dev(), capsule_after.ino())
    );
    assert_eq!(
        fs::read(os_path(&source)).unwrap(),
        b"source bytes are never read"
    );
    assert_eq!(
        fs::read(os_path(&recovery)).unwrap(),
        capsule(
            &source,
            Some((
                current.len(),
                current.mtime() + 1,
                current.mtime_nsec() as u32
            ))
        )
    );
    cleanup(&[&source, &recovery]);
}

#[test]
fn wrong_identity_corrupt_unsupported_and_invalid_bundle_are_invalid() {
    let source = path("source");
    let other = path("other");
    let recovery = path("capsule");
    write(&source, b"source");
    write(&other, b"other");
    write(&recovery, &capsule(&other, None));
    discovery_error(&source, &recovery, RecoveryError::PathMismatch);
    let mut bad = capsule(&source, None);
    *bad.last_mut().unwrap() ^= 1;
    write(&recovery, &bad);
    discovery_error(&source, &recovery, RecoveryError::Corrupt);
    let mut unsupported = capsule(&source, None);
    put_u32(&mut unsupported, 8, 2);
    write(&recovery, &unsupported);
    discovery_error(&source, &recovery, RecoveryError::UnsupportedVersion);
    let mut invalid_bundle = capsule(&source, None);
    let path_length = u32::from_le_bytes(invalid_bundle[16..20].try_into().unwrap()) as usize;
    invalid_bundle[RECOVERY_HEADER_BYTES + path_length] ^= 1;
    refresh_capsule_checksums(&mut invalid_bundle);
    write(&recovery, &invalid_bundle);
    discovery_error(&source, &recovery, RecoveryError::InvalidProject);
    cleanup(&[&source, &other, &recovery]);
}

#[test]
fn rejects_oversized_symlink_nonregular_and_same_path() {
    let source = path("source");
    let recovery = path("capsule");
    write(&source, b"source");
    discovery_error(&source, &source, RecoveryError::InvalidArgument);
    let oversized = File::create(os_path(&recovery)).unwrap();
    oversized.set_len(RECOVERY_MAX_BYTES as u64 + 1).unwrap();
    drop(oversized);
    discovery_error(&source, &recovery, RecoveryError::Limit);
    cleanup(&[&recovery]);

    let dir = path("directory");
    fs::create_dir(os_path(&dir)).unwrap();
    discovery_error(&source, &dir, RecoveryError::InvalidArgument);
    let link = path("link");
    std::os::unix::fs::symlink(os_path(&source), os_path(&link)).unwrap();
    discovery_error(&link, &recovery, RecoveryError::InvalidArgument);
    discovery_error(&source, &link, RecoveryError::InvalidArgument);
    let missing = path("missing");
    assert!(matches!(
        project_recovery_discover(&missing, &recovery),
        RecoveryDiscovery::Invalid {
            error: RecoveryError::InvalidArgument
        }
    ));
    cleanup(&[&source, &link]);
    let _ = fs::remove_dir(os_path(&dir));
}
