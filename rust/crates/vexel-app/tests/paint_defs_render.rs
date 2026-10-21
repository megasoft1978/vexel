//! Renders a `V1Document` built with gradient and clip definitions through the
//! production compose path and checks pixel colours.
use vexel_app::compose::compose_svg;
use vexel_v1_document::{GradientStop, GradientUnits, Spread, V1Document};

fn pixel(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * width + x) * 4) as usize;
    [rgba[at], rgba[at + 1], rgba[at + 2], rgba[at + 3]]
}

#[test]
fn gradient_fill_and_clip_render_as_authored() {
    let mut doc = V1Document::new(64, 64).unwrap();
    doc.add_rectangle_with_id(0.0, 0.0, 64.0, 64.0, "#00ff00", "box")
        .unwrap();
    let stops = [
        GradientStop {
            offset: 0.0,
            color: "#ff0000".into(),
            opacity: 1.0,
        },
        GradientStop {
            offset: 1.0,
            color: "#0000ff".into(),
            opacity: 1.0,
        },
    ];
    let gradient = doc
        .add_linear_gradient(
            0.0,
            0.0,
            64.0,
            0.0,
            GradientUnits::UserSpace,
            Spread::Pad,
            &stops,
        )
        .unwrap();
    let clip = doc.add_clip_rect(0.0, 0.0, 64.0, 32.0).unwrap();
    doc.set_fill_ref("box", &gradient).unwrap();
    doc.set_clip("box", Some(&clip)).unwrap();

    let bytes = doc.source().unwrap();
    let frame = compose_svg(&bytes, 64, 64).unwrap();
    assert!(frame.disclosures.is_empty(), "{:?}", frame.disclosures);
    let left = pixel(&frame.rgba, 64, 1, 16);
    let right = pixel(&frame.rgba, 64, 62, 16);
    let outside = pixel(&frame.rgba, 64, 32, 60);
    assert!(
        left[0] > 220 && left[1] < 20 && left[2] < 35 && left[3] == 255,
        "{left:?}"
    );
    assert!(
        right[2] > 220 && right[1] < 20 && right[0] < 35 && right[3] == 255,
        "{right:?}"
    );
    assert_eq!(outside[3], 0, "{outside:?}");

    // Removing the clip paints the lower half too.
    doc.set_clip("box", None).unwrap();
    let frame = compose_svg(&doc.source().unwrap(), 64, 64).unwrap();
    assert_eq!(pixel(&frame.rgba, 64, 32, 60)[3], 255);
}
