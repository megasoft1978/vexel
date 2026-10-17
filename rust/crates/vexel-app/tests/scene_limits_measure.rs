//! WO-RUST-SCENE-LIMITS-01 measurement: composes the files named in
//! `VEXEL_LIMITS_FILES` (colon-separated paths, read-only) and prints, per file,
//! the id count, `<use>` count, summed `<use>` target bytes and the verdict.
//! Run: `VEXEL_LIMITS_FILES=a.svg:b.svg cargo test -p vexel-app --release
//! --test scene_limits_measure -- --ignored --nocapture`.
use std::time::Instant;
use vexel_app::compose::compose_svg;

fn count(haystack: &[u8], needle: &[u8]) -> usize {
    let mut n = 0;
    let mut i = 0;
    while let Some(p) = haystack[i..]
        .windows(needle.len())
        .position(|w| w == needle)
    {
        n += 1;
        i += p + needle.len();
    }
    n
}

#[test]
#[ignore = "measurement only; reads external files named by VEXEL_LIMITS_FILES"]
fn print_limit_measurements() {
    let Ok(list) = std::env::var("VEXEL_LIMITS_FILES") else {
        return;
    };
    for path in list.split(':').filter(|p| !p.is_empty()) {
        let bytes = std::fs::read(path).expect("readable file");
        let started = Instant::now();
        let verdict = compose_svg(&bytes, 128, 128).map(|f| (f.width, f.height));
        println!(
            "{path}: bytes={} id_attrs={} uses={} text_elements={} verdict={verdict:?} ms={}",
            bytes.len(),
            count(&bytes, b" id=\"") + count(&bytes, b" id='"),
            count(&bytes, b"<use"),
            count(&bytes, b"<text"),
            started.elapsed().as_millis()
        );
    }
}
