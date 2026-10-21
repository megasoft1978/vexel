use super::*;
use std::io;
use std::os::unix::fs::PermissionsExt;

#[test]
fn save_suggestion_uses_document_stem_and_format_extension() {
    assert_eq!(
        suggestion(Some("Walk.vexel"), ExportFormat::Gif),
        "Walk.gif"
    );
    assert_eq!(
        suggestion(Some("Walk.vexel"), ExportFormat::Video),
        "Walk.mp4"
    );
    assert_eq!(suggestion(Some("Walk.svg"), ExportFormat::Png), "Walk.png");
    assert_eq!(suggestion(None, ExportFormat::Gif), "Untitled.gif");
    assert_eq!(
        suggestion(Some(".vexel"), ExportFormat::Gif),
        "Untitled.gif"
    );
}

#[test]
fn export_target_replaces_project_suffix_with_format_extension() {
    let target = |bytes: &[u8], format| {
        export_target(bytes.to_vec(), format).map(|path| path.to_string_lossy().into_owned())
    };
    assert_eq!(
        target(b"/x/Walk.gif.vexel", ExportFormat::Gif).as_deref(),
        Some("/x/Walk.gif")
    );
    assert_eq!(
        target(b"/x/Walk.vexel", ExportFormat::Video).as_deref(),
        Some("/x/Walk.mp4")
    );
    assert_eq!(
        target(b"/x/Walk", ExportFormat::Gif).as_deref(),
        Some("/x/Walk.gif")
    );
    assert_eq!(target(b"/x/.gif", ExportFormat::Gif), None);
}

#[test]
fn permission_denied_maps_to_folder_message_and_other_errors_keep_generic() {
    let denied = io::Error::from(io::ErrorKind::PermissionDenied);
    assert_eq!(write_failed(&denied), UNWRITABLE);
    assert!(UNWRITABLE.contains("choose a folder you have permission to save in"));
    let missing = io::Error::from(io::ErrorKind::NotFound);
    assert_eq!(write_failed(&missing), WRITE_FAILED);
}

#[test]
fn stage_in_read_only_folder_reports_folder_message() {
    let base = std::env::temp_dir().join(format!(
        "vexel-export-readonly-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let folder = base.join("locked");
    fs::create_dir_all(&folder).unwrap();
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o555)).unwrap();
    let result = Stage::create(&folder.join("out.gif")).map(|stage| stage.dir.clone());
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o755)).unwrap();
    let _ = fs::remove_dir_all(&base);
    assert_eq!(result.err(), Some(UNWRITABLE));
}
