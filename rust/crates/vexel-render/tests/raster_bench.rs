//! Release micro-benchmark (p50 ms) of the rasterizer on seeded scenes.
//! Run: `cargo test -p vexel-render --release --test raster_bench -- --ignored --nocapture`
#[path = "common/raster_scene.rs"]
mod raster_scene;

use raster_scene::{Profile, scene, view};
use std::time::Instant;
use vexel_raster::Tile;
use vexel_render::render_groups;

const SIZE: u32 = 1024;
const RUNS: usize = 7;

#[test]
#[ignore]
fn p50_render_ms() {
    for profile in [Profile::Fill, Profile::Gradient, Profile::Stroke, Profile::Nest, Profile::Clip] {
        for count in [500usize, 2000, 5000] {
            let nodes = scene(3, SIZE, count, profile);
            let mut times = Vec::new();
            for _ in 0..RUNS {
                let mut tile =
                    Tile::from_owned_rgba(SIZE, SIZE, vec![0; (SIZE * SIZE * 4) as usize]).unwrap();
                let start = Instant::now();
                render_groups(nodes, view(SIZE), &mut tile, &|| false).unwrap();
                times.push(start.elapsed().as_secs_f64() * 1000.0);
            }
            times.sort_by(f64::total_cmp);
            println!("BENCH {profile:?} {count} p50_ms {:.1}", times[RUNS / 2]);
        }
    }
}
