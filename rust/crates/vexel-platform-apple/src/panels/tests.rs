use super::*;

#[test]
fn path_is_bounded_and_rejects_nul() {
    assert_eq!(
        checked_path(b"/tmp/a.vexel".to_vec()),
        PanelResult::Path(b"/tmp/a.vexel".to_vec())
    );
    assert_eq!(checked_path(vec![]), PanelResult::Error);
    assert_eq!(checked_path(b"/tmp/a\0b".to_vec()), PanelResult::Error);
    assert!(matches!(
        checked_path(vec![b'a'; MAX_PANEL_PATH]),
        PanelResult::Path(_)
    ));
    assert_eq!(
        checked_path(vec![b'a'; MAX_PANEL_PATH + 1]),
        PanelResult::Error
    );
}

#[test]
fn extension_is_appended_only_when_missing() {
    let ok = |p: &[u8]| PanelResult::Path(p.to_vec());
    assert_eq!(
        with_project_extension(b"/tmp/a".to_vec()),
        ok(b"/tmp/a.vexel")
    );
    assert_eq!(
        with_project_extension(b"/tmp/a.vexel".to_vec()),
        ok(b"/tmp/a.vexel")
    );
    assert_eq!(
        with_project_extension(b"/tmp/a.VEXEL".to_vec()),
        ok(b"/tmp/a.VEXEL")
    );
    assert_eq!(
        with_project_extension(b"/tmp/a.svg".to_vec()),
        ok(b"/tmp/a.svg.vexel")
    );
}

#[test]
fn extension_append_respects_the_path_bound() {
    assert_eq!(
        with_project_extension(vec![b'a'; MAX_PANEL_PATH - 5]),
        PanelResult::Error
    );
    assert!(matches!(
        with_project_extension(vec![b'a'; MAX_PANEL_PATH - 6]),
        PanelResult::Path(p) if p.len() == MAX_PANEL_PATH
    ));
    assert_eq!(with_project_extension(b"a\0".to_vec()), PanelResult::Error);
}

#[test]
fn native_calls_are_rejected_off_the_main_thread() {
    // Never reaches AppKit: the marker check fails on a spawned thread.
    let off = std::thread::spawn(|| native::require_main().is_none())
        .join()
        .unwrap();
    assert!(off);
}

#[test]
fn recovery_modal_responses_map_to_the_requested_choices() {
    assert_eq!(
        native::recovery_choice_from_modal(1000),
        RecoveryChoice::Restore
    );
    assert_eq!(
        native::recovery_choice_from_modal(1001),
        RecoveryChoice::Discard
    );
    assert_eq!(
        native::recovery_choice_from_modal(1002),
        RecoveryChoice::Cancel
    );
    assert_eq!(
        native::recovery_choice_from_modal(-1000),
        RecoveryChoice::Cancel
    );
}
