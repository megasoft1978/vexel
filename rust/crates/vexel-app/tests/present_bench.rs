//! Headless bench of the app-side CPU of one present at a 2560x1440 buffer
//! (1280x720 points at 2x): full shell paint, document blit, and the pooled
//! repaint paths. The NSWindow hand-off is not exercised.
//! `cargo test -p vexel-app --release --test present_bench -- --ignored --nocapture`
use std::path::PathBuf;
use std::time::{Duration, Instant};
use vexel_app::animation::driver::Driver;
use vexel_app::animation::playback::MonotonicClock;
use vexel_app::animation::stage;
use vexel_app::compose::{DocumentView, blit_document};
use vexel_app::model::Model;
use vexel_app::render::render;
use vexel_app::session::AppDocument;

fn pct(v: &mut [f64], p: f64) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    v[((v.len() - 1) as f64 * p).round() as usize]
}

fn report(name: &str, v: &mut [f64]) {
    println!(
        "present_bench {name}: p50 {:.2} ms p95 {:.2} ms",
        pct(v, 0.5),
        pct(v, 0.95)
    );
}

#[test]
#[ignore]
fn app_present_cpu() {
    let model = Model::new(true);
    let layout = model.layout(1280, 720).unwrap();
    let stage_size = (layout.stage.width, layout.stage.height);
    let mut app = AppDocument::new(10, 10).unwrap();
    let mut view = DocumentView::new();
    app.open_path(&PathBuf::from(format!(
        "{}/../../fixtures/v1/compose/compose-v1.svg",
        env!("CARGO_MANIFEST_DIR")
    )))
    .unwrap();
    let mut driver = Driver::new(MonotonicClock::new());
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        stage::sync(&mut driver, &app, &mut view, stage_size);
        view.poll();
        if view
            .composed()
            .is_some_and(|c| (c.frame.width, c.frame.height) == stage_size)
        {
            break;
        }
        assert!(Instant::now() < deadline, "compose timed out");
        std::thread::sleep(Duration::from_millis(2));
    }
    let (mut full, mut blit, mut both) = (vec![], vec![], vec![]);
    for _ in 0..40 {
        let t = Instant::now();
        let frame = render(1280, 720, 2, &model).unwrap();
        let a = t.elapsed().as_secs_f64() * 1e3;
        let t = Instant::now();
        let frame = blit_document(frame, &view, &layout, true);
        let b = t.elapsed().as_secs_f64() * 1e3;
        assert_eq!(frame.width(), 2560);
        full.push(a);
        blit.push(b);
        both.push(a + b);
    }
    report("shell_paint", &mut full);
    report("blit_document", &mut blit);
    report("paint+blit", &mut both);
}
