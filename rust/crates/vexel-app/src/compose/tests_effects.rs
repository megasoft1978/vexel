//! SVG effects: `mix-blend-mode`, `feTurbulence`, text stroke / `paint-order`,
//! and the GPU route staying out of the way. Pixel expectations are the W3C
//! formulas for opaque colours; the showcase renders when `VEXEL_EFFECTS_OUT`
//! names a directory (PPM over white).
use super::gpu_route::Decision;
use super::tests_gpu::{cpu_frame, frame, serial, with_gpu};
use super::*;

const HEAD: &str =
    "<svg xmlns='http://www.w3.org/2000/svg' width='40' height='40' viewBox='0 0 40 40'>";

fn px(frame: &Frame, x: u32, y: u32) -> [u8; 4] {
    let o = (y as usize * frame.width as usize + x as usize) * 4;
    frame.rgba[o..o + 4].try_into().unwrap()
}

fn blended(mode: &str, backdrop: &str, source: &str) -> [u8; 4] {
    let svg = format!(
        "{HEAD}<rect width='40' height='40' fill='{backdrop}'/>\
         <g style='mix-blend-mode:{mode}'><rect width='40' height='40' fill='{source}'/></g></svg>"
    );
    px(&compose_svg(svg.as_bytes(), 40, 40).unwrap(), 20, 20)
}

#[test]
fn every_blend_mode_name_decodes_and_group_blends_analytically() {
    // backdrop (200, 100, 50), source (100, 200, 250): per-channel W3C results.
    let (b, s) = ((200, 100, 50), (100, 200, 250));
    let expect = |f: fn(f64, f64) -> f64| {
        let c = |cb: i32, cs: i32| {
            (f(f64::from(cb) / 255.0, f64::from(cs) / 255.0) * 255.0).round() as u8
        };
        [c(b.0, s.0), c(b.1, s.1), c(b.2, s.2), 255]
    };
    let cases: [(&str, fn(f64, f64) -> f64); 6] = [
        ("multiply", |cb, cs| cb * cs),
        ("screen", |cb, cs| cb + cs - cb * cs),
        ("darken", f64::min),
        ("lighten", f64::max),
        ("difference", |cb, cs| (cb - cs).abs()),
        ("exclusion", |cb, cs| cb + cs - 2.0 * cb * cs),
    ];
    for (mode, f) in cases {
        let got = blended(mode, "rgb(200,100,50)", "rgb(100,200,250)");
        let want = expect(f);
        for c in 0..3 {
            assert!(
                got[c].abs_diff(want[c]) <= 1,
                "{mode} channel {c}: {got:?} vs {want:?}"
            );
        }
        assert_eq!(got[3], 255);
    }
    for mode in [
        "normal",
        "overlay",
        "color-dodge",
        "color-burn",
        "hard-light",
        "soft-light",
        "plus-lighter",
    ] {
        assert_eq!(blended(mode, "#fff", "#000").len(), 4, "{mode}");
    }
    assert_eq!(
        blended("normal", "rgb(200,100,50)", "rgb(100,200,250)"),
        [100, 200, 250, 255]
    );
    assert_eq!(
        blended("plus-lighter", "rgb(200,100,50)", "rgb(100,200,250)"),
        [255, 255, 255, 255]
    );
    // overlay(cb, cs) = hard-light(cs, cb): channel 0 has cb > 0.5 -> screen branch.
    let overlay = blended("overlay", "rgb(200,100,50)", "rgb(100,200,250)");
    let (cb, cs) = (200.0 / 255.0, 100.0 / 255.0);
    let t = 2.0 * cb - 1.0;
    let want = (cs + t - cs * t) * 255.0;
    assert!(
        (f64::from(overlay[0]) - want).abs() <= 1.0,
        "overlay {overlay:?}"
    );
}

#[test]
fn blend_group_over_transparent_backdrop_is_the_source() {
    let svg = format!(
        "{HEAD}<g style='mix-blend-mode:multiply' opacity='0.5'><rect width='40' height='40' fill='#f00'/></g></svg>"
    );
    assert_eq!(
        px(&compose_svg(svg.as_bytes(), 40, 40).unwrap(), 5, 5),
        [255, 0, 0, 128]
    );
}

fn turbulence_svg(kind: &str, extra: &str) -> String {
    format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' viewBox='0 0 64 64'>\
         <defs><filter id='n' x='0' y='0' width='1' height='1'>\
         <feTurbulence type='{kind}' baseFrequency='0.08' numOctaves='3' seed='4' {extra}/>\
         </filter></defs><g filter='url(#n)'><rect width='64' height='64'/></g></svg>"
    )
}

#[test]
fn turbulence_filter_renders_deterministic_noise() {
    let a = compose_svg(turbulence_svg("fractalNoise", "").as_bytes(), 64, 64).unwrap();
    let b = compose_svg(turbulence_svg("fractalNoise", "").as_bytes(), 64, 64).unwrap();
    assert_eq!(a, b);
    let distinct: std::collections::HashSet<[u8; 4]> = a
        .rgba
        .chunks_exact(4)
        .map(|p| p.try_into().unwrap())
        .collect();
    assert!(
        distinct.len() > 200,
        "noise has texture ({} colours)",
        distinct.len()
    );
    let t = compose_svg(
        turbulence_svg("turbulence", "stitchTiles='stitch'").as_bytes(),
        64,
        64,
    )
    .unwrap();
    assert_ne!(a, t);
    // Grain recipe: noise -> luminanceToAlpha is accepted and varies the alpha.
    let grain = turbulence_svg("fractalNoise", "").replace(
        "/></filter>",
        "/><feColorMatrix type='luminanceToAlpha'/></filter>",
    );
    let g = compose_svg(grain.as_bytes(), 64, 64).unwrap();
    let alphas: std::collections::HashSet<u8> = g.rgba.chunks_exact(4).map(|p| p[3]).collect();
    assert!(
        alphas.len() > 20,
        "luminanceToAlpha grain alpha varies ({})",
        alphas.len()
    );
}

#[test]
fn turbulence_attribute_errors_are_typed() {
    for bad in [
        "baseFrequency='-1'",
        "numOctaves='1.5'",
        "type='bogus'",
        "stitchTiles='x'",
    ] {
        let svg = turbulence_svg("fractalNoise", bad).replace("baseFrequency='0.08'", "");
        assert!(compose_svg(svg.as_bytes(), 64, 64).is_err(), "{bad}");
    }
}

#[test]
fn text_stroke_and_paint_order_draw_outline_below_fill() {
    let svg = |order: &str| {
        format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='120' height='60' viewBox='0 0 120 60'>\
             <rect width='120' height='60' fill='#888'/>\
             <text x='60' y='44' text-anchor='middle' font-family='Helvetica' font-weight='bold' font-size='44' \
             fill='#fff' stroke='#000' stroke-width='8' stroke-linejoin='round' {order}>I</text></svg>"
        )
    };
    let count = |f: &Frame, rgb: [u8; 3]| f.rgba.chunks_exact(4).filter(|p| p[..3] == rgb).count();
    let normal = compose_svg(svg("").as_bytes(), 120, 60).unwrap();
    let below = compose_svg(svg("paint-order='stroke'").as_bytes(), 120, 60).unwrap();
    let white_normal = count(&normal, [255, 255, 255]);
    let white_below = count(&below, [255, 255, 255]);
    let black = count(&below, [0, 0, 0]);
    assert!(black > 100, "outline drawn ({black})");
    assert!(
        white_below > white_normal + 40,
        "fill stays whole under paint-order: {white_below} vs {white_normal}"
    );
}

#[test]
fn gpu_route_stays_on_the_cpu_for_blend_and_turbulence_scenes() {
    let _g = serial();
    let blend = format!(
        "{HEAD}<rect width='40' height='40' fill='#fa0'/><g style='mix-blend-mode:screen'><rect width='20' height='20' fill='#08f'/></g></svg>"
    );
    for svg in [blend, turbulence_svg("fractalNoise", "")] {
        let mut cache = DiffCache::display();
        let got = with_gpu(Some(true), || frame(svg.as_bytes(), 64, 64, &mut cache));
        let decision = super::gpu_route::last_decision();
        if decision == Decision::NoDevice {
            continue;
        }
        assert_eq!(decision, Decision::Cpu("group"), "{svg}");
        assert_eq!(got, cpu_frame(svg.as_bytes(), 64, 64));
    }
}

/// Writes the showcase (and a 1024x1024 grain timing) when asked.
#[test]
fn showcase_renders_when_requested() {
    let Some(dir) = std::env::var_os("VEXEL_EFFECTS_OUT") else {
        return;
    };
    let path = format!(
        "{}/../../../docs/media/effects_showcase.svg",
        env!("CARGO_MANIFEST_DIR")
    );
    let svg = std::fs::read(path).unwrap();
    let frame = compose_svg(&svg, 640, 360).unwrap();
    let mut ppm = format!("P6\n{} {}\n255\n", frame.width, frame.height).into_bytes();
    for p in frame.rgba.chunks_exact(4) {
        let a = u32::from(p[3]);
        ppm.extend((0..3).map(|c| ((u32::from(p[c]) * a + 255 * (255 - a) + 127) / 255) as u8));
    }
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(std::path::Path::new(&dir).join("effects_showcase.ppm"), ppm).unwrap();
}
