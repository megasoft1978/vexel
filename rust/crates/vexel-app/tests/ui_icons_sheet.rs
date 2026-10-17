//! Icon review sheet and first-party golden check (WO-RUST-UI-ICONS-01).
//!
//! The sheet is rendered by the software `WidgetCanvas` path only (no CoreText, no text), so
//! goldens match exactly (tolerance 0). Goldens are `rust/fixtures/ui/icons/icon-sheet-<theme>-<n>x.rgba`
//! (8-byte header: width, height as little-endian u32, then straight RGBA8). They stay CANDIDATES
//! until a human has viewed `artifacts/WO-RUST-UI-ICONS-01/review/*.png`.
//! `VEXEL_BLESS_GOLDENS=1 cargo test -p vexel-app --test ui_icons_sheet` rewrites candidates and
//! review PNGs; normal runs never write.

use std::path::PathBuf;
use vexel_ui::{
    ControlState, IconKind, Rect, Theme, WidgetCanvas, icon_centered, icon_definitions,
    theme_tokens,
};

const CELL: u32 = 32;
const MARGIN: u32 = 8;
const COLS: u32 = 11;
const STATES: [ControlState; 7] = [
    ControlState::Rest,
    ControlState::Hover,
    ControlState::Pressed,
    ControlState::Focus,
    ControlState::Disabled,
    ControlState::Selected,
    ControlState::Invalid,
];
const STATE_ROWS: [IconKind; 5] = [
    IconKind::SELECT,
    IconKind::PEN,
    IconKind::EYE,
    IconKind::LOCK,
    IconKind::KEYFRAME_FILLED,
];

fn sheet_size() -> (u32, u32) {
    let rows = (icon_definitions().len() as u32).div_ceil(COLS);
    (
        MARGIN * 2 + COLS * CELL,
        MARGIN * 3 + (rows + STATE_ROWS.len() as u32) * CELL,
    )
}

fn render_sheet(theme: Theme, scale: u32) -> (u32, u32, Vec<u8>) {
    let (w, h) = sheet_size();
    let (pw, ph) = (w * scale, h * scale);
    let mut rgba = vec![0u8; (pw * ph * 4) as usize];
    let capacity = rgba.len();
    let tokens = theme_tokens(theme, false);
    let mut c = WidgetCanvas::new(
        &mut rgba,
        capacity,
        pw,
        ph,
        (pw * 4) as usize,
        scale,
        theme,
        false,
        None,
    )
    .expect("canvas");
    c.paint_rect(
        Rect {
            x: 0,
            y: 0,
            width: w,
            height: h,
        },
        tokens.panel,
    );
    let at = |col: u32, row: u32, extra: u32| Rect {
        x: MARGIN + col * CELL,
        y: MARGIN + row * CELL + extra,
        width: CELL,
        height: CELL,
    };
    for (i, def) in icon_definitions().iter().enumerate() {
        let (col, row) = (i as u32 % COLS, i as u32 / COLS);
        assert!(
            icon_centered(
                &mut c,
                at(col, row, 0),
                def.icon,
                ControlState::Rest,
                &tokens
            ),
            "{}",
            def.name
        );
    }
    let base = (icon_definitions().len() as u32).div_ceil(COLS);
    for (r, icon) in STATE_ROWS.iter().enumerate() {
        for (col, state) in STATES.iter().enumerate() {
            let cell = at(col as u32, base + r as u32, MARGIN);
            if *state == ControlState::Selected {
                c.paint_round(cell, 6, tokens.selection_fill, tokens.selection_fill, 0);
            }
            assert!(icon_centered(&mut c, cell, *icon, *state, &tokens));
        }
    }
    (pw, ph, rgba)
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn zoom(w: u32, h: u32, rgba: &[u8], k: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((w * k * h * k * 4) as usize);
    for y in 0..h {
        let row: Vec<u8> = (0..w)
            .flat_map(|x| {
                let p = ((y * w + x) * 4) as usize;
                (0..k).flat_map(move |_| rgba[p..p + 4].to_vec())
            })
            .collect();
        for _ in 0..k {
            out.extend_from_slice(&row);
        }
    }
    out
}

fn png(path: PathBuf, w: u32, h: u32, rgba: &[u8]) {
    let bytes = vexel_png::encode_rgba8(w, h, rgba, vexel_png::Compression::Fixed).expect("png");
    std::fs::write(path, bytes).expect("write png");
}

#[test]
fn icon_sheet_matches_first_party_goldens() {
    let bless = std::env::var("VEXEL_BLESS_GOLDENS").as_deref() == Ok("1");
    for (name, theme) in [("dark", Theme::Dark), ("light", Theme::Light)] {
        for scale in [1u32, 2] {
            let (w, h, rgba) = render_sheet(theme, scale);
            assert_eq!(
                render_sheet(theme, scale).2,
                rgba,
                "deterministic {name} {scale}x"
            );
            let golden = root().join(format!(
                "rust/fixtures/ui/icons/icon-sheet-{name}-{scale}x.rgba"
            ));
            let mut file = Vec::from(w.to_le_bytes());
            file.extend_from_slice(&h.to_le_bytes());
            file.extend_from_slice(&rgba);
            if bless {
                std::fs::create_dir_all(golden.parent().unwrap()).unwrap();
                std::fs::write(&golden, &file).unwrap();
                let review = root().join("artifacts/WO-RUST-UI-ICONS-02/review");
                std::fs::create_dir_all(&review).unwrap();
                png(
                    review.join(format!("icon-sheet-{name}-{scale}x.png")),
                    w,
                    h,
                    &rgba,
                );
                let k = 4 / scale;
                png(
                    review.join(format!("icon-sheet-{name}-{scale}x-zoom{}.png", k * scale)),
                    w * k,
                    h * k,
                    &zoom(w, h, &rgba, k),
                );
            }
            let expected = std::fs::read(&golden).unwrap_or_else(|_| {
                panic!(
                    "missing candidate golden {}; run with VEXEL_BLESS_GOLDENS=1",
                    golden.display()
                )
            });
            assert!(
                expected == file,
                "icon sheet {name} {scale}x differs from golden"
            );
        }
    }
}
