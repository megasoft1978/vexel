//! The retained UI text renderer only draws ASCII (anything else becomes
//! `?`), so painted UI strings must stay ASCII.

use super::*;

fn ascii_code(src: &str) -> bool {
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .all(|l| l.is_ascii())
}

#[test]
fn tooltip_tables_are_ascii() {
    for button in [
        Button::Undo,
        Button::Redo,
        Button::Save,
        Button::Export,
        Button::ToggleInspector,
        Button::Mcp,
    ] {
        let (name, hint) = tooltip(Target::Button(button)).unwrap();
        assert!(name.is_ascii() && hint.is_ascii(), "{button:?}");
    }
    for i in EXT_FIRST..EXT_END {
        if let Some((name, hint)) = tooltip(Target::SectionHeader(i)) {
            assert!(name.is_ascii() && hint.is_ascii(), "ext control {i}");
        }
    }
}

#[test]
fn painted_label_sources_are_ascii() {
    for (name, src) in [
        (
            "native_timeline/paint.rs",
            include_str!("../native_timeline/paint.rs"),
        ),
        ("inspector_ext.rs", include_str!("../inspector_ext.rs")),
        ("inspector.rs", include_str!("../inspector.rs")),
        ("layout/paint.rs", include_str!("paint.rs")),
        ("layout/inspector_ext.rs", include_str!("inspector_ext.rs")),
    ] {
        assert!(ascii_code(src), "{name} has non-ASCII painted text");
    }
}
