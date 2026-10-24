//! Differential test: a seeded mix scene (fills, gradients, strokes, dashes,
//! transformed shapes, opacity groups, clips) rendered at three sizes must keep
//! the SHA-256 digests captured before the rasterizer performance work.
//! Set `VEXEL_PRINT_HASHES=1` to print the current digests.
#[path = "common/raster_scene.rs"]
mod raster_scene;
#[path = "common/sha256.rs"]
mod sha256;

use raster_scene::{Profile, dashed, scene, view};
use vexel_raster::Tile;
use vexel_render::{render_dashed_shapes, render_groups};

fn tile(size: u32) -> Tile {
    Tile::from_owned_rgba(size, size, vec![0; size as usize * size as usize * 4]).unwrap()
}

fn digest(size: u32, profile: Profile, count: usize) -> String {
    let mut out = tile(size);
    render_groups(scene(7 + size as u64, size, count, profile), view(size), &mut out, &|| false)
        .unwrap();
    sha256::sha256_hex(out.rgba())
}

fn dashed_digest(size: u32, count: usize) -> String {
    let mut out = tile(size);
    render_dashed_shapes(&dashed(11 + size as u64, size, count), view(size), &mut out, &|| false)
        .unwrap();
    sha256::sha256_hex(out.rgba())
}

fn current() -> Vec<(String, String)> {
    let mut rows = Vec::new();
    for (size, count) in [(97u32, 60usize), (256, 220), (389, 400)] {
        for profile in [
            Profile::Fill,
            Profile::Gradient,
            Profile::Stroke,
            Profile::Nest,
            Profile::Clip,
            Profile::Mix,
        ] {
            rows.push((format!("{profile:?}-{size}"), digest(size, profile, count)));
        }
        rows.push((format!("Dash-{size}"), dashed_digest(size, count / 2)));
    }
    // Extra stroke and nest cases (sizes off the 4x4 grid, denser scenes), recorded
    // from the unmodified rasterizer before the stroke/layer optimizations.
    for (size, count) in [(61u32, 150usize), (203, 900), (512, 1500)] {
        for profile in [Profile::Stroke, Profile::Nest] {
            rows.push((format!("{profile:?}x-{size}"), digest(size, profile, count)));
        }
    }
    rows
}

#[test]
fn seeded_mix_scenes_match_the_recorded_digests() {
    let rows = current();
    if std::env::var_os("VEXEL_PRINT_HASHES").is_some() {
        for (name, hash) in &rows {
            println!("{name} {hash}");
        }
    }
    let recorded = include_str!("raster_diff_hashes.txt");
    let expected: Vec<(&str, &str)> =
        recorded.lines().filter_map(|l| l.split_once(' ')).collect();
    assert_eq!(expected.len(), rows.len(), "recorded digest count");
    for ((name, hash), (want_name, want_hash)) in rows.iter().zip(expected) {
        assert_eq!(name, want_name);
        assert_eq!(hash, want_hash, "{name} changed");
    }
}
