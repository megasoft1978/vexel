//! Release micro-benchmark (p50 ms) of the rasterizer on seeded scenes.
//! Run: `cargo test -p vexel-render --release --test raster_bench -- --ignored --nocapture`
#[path = "common/raster_scene.rs"]
mod raster_scene;

use raster_scene::{Profile, scene, view};
use std::time::Instant;
use vexel_raster::Tile;
use vexel_render::render_groups;

const SIZE: u32 = 1024;
const DEFAULT_RUNS: usize = 7;
// `VEXEL_BENCH_COUNT=5000` restricts the run to one shape count; `VEXEL_BENCH_RUNS` sets the repeats.
// `VEXEL_BENCH_ONLY=Stroke` restricts the run to one profile.

#[test]
#[ignore]
fn p50_render_ms() {
    let runs = std::env::var("VEXEL_BENCH_RUNS").ok().and_then(|v| v.parse().ok()).unwrap_or(DEFAULT_RUNS);
    for profile in [Profile::Fill, Profile::Gradient, Profile::Stroke, Profile::Nest, Profile::Clip, Profile::Rect] {
        let only = std::env::var("VEXEL_BENCH_ONLY").ok();
        if only.is_some_and(|name| name != format!("{profile:?}")) {
            continue;
        }
        let only_count = std::env::var("VEXEL_BENCH_COUNT").ok();
        for count in [500usize, 2000, 5000] {
            if only_count.as_ref().is_some_and(|c| *c != count.to_string()) {
                continue;
            }
            let nodes = scene(3, SIZE, count, profile);
            let mut times = Vec::new();
            for _ in 0..runs {
                let mut tile =
                    Tile::from_owned_rgba(SIZE, SIZE, vec![0; (SIZE * SIZE * 4) as usize]).unwrap();
                let start = Instant::now();
                render_groups(nodes, view(SIZE), &mut tile, &|| false).unwrap();
                times.push(start.elapsed().as_secs_f64() * 1000.0);
            }
            times.sort_by(f64::total_cmp);
            println!("BENCH {profile:?} {count} p50_ms {:.1}", times[runs / 2]);
        }
    }
}
