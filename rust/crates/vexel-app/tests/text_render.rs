use vexel_app::compose::compose_svg;

#[test]
fn text_render_probe() {
    let frame = compose_svg(
        b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"64\"><text x=\"10\" y=\"40\" font-size=\"24\" fill=\"#112233\">Hi</text></svg>",
        64,
        64,
    )
    .unwrap();
    assert!(frame.rgba.chunks_exact(4).any(|pixel| pixel[3] != 0));
}
