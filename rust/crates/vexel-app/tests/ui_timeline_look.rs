//! Look check: writes the Animate footer as PNG when `VEXEL_TL_PNG_DIR` is set.
#[path = "ui_timeline_support.rs"]
mod support;
use support::*;
use vexel_png::{Compression, encode_rgba8};

#[test]
fn dump_footer_pngs() {
    let Some(dir) = std::env::var_os("VEXEL_TL_PNG_DIR") else {
        return;
    };
    for (name, src, t) in [("loop", LOOP, 0.8), ("many", LONG, 1.3)] {
        for dark in [true, false] {
            let mut d = driver_with(src);
            d.seek(t);
            let mut tl = timeline_of(src, &d);
            tl.selection = Some(vexel_app::timeline::model::KeyRef {
                element: 0,
                prop: Some(0),
                index: 1,
            });
            let (w, h, px) = frame((1280, 800), dark, 2, tl);
            let png = encode_rgba8(w, h, &px, Compression::Fixed).unwrap();
            let theme = if dark { "dark" } else { "light" };
            std::fs::write(
                std::path::Path::new(&dir).join(format!("{name}-{theme}.png")),
                png,
            )
            .unwrap();
        }
    }
}
