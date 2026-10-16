use super::*;

#[test]
fn clipped_rgba_copyback_preserves_pixels_outside_the_text_box() {
    let mut pixels = vec![0u8; 4 * 3 * 3];
    for (index, pixel) in pixels.chunks_exact_mut(4).enumerate() {
        pixel.copy_from_slice(&[20 + index as u8, 40, 60, 255]);
    }
    let before = pixels.clone();
    let rect = (1, 1, 1, 1);
    let mut clip = vec![0; 4];
    copy_to_premultiplied(&pixels, 12, rect, 4, &mut clip);
    assert_eq!(clip, [24, 40, 60, 255]);
    clip.copy_from_slice(&[50, 25, 10, 128]);
    copy_from_premultiplied(&mut clip, 4, &mut pixels, 12, rect);
    for index in 0..9 {
        let at = index * 4;
        if index == 4 {
            assert_eq!(&pixels[at..at + 4], &[100, 50, 20, 128]);
        } else {
            assert_eq!(&pixels[at..at + 4], &before[at..at + 4]);
        }
    }
}

#[test]
fn coretext_foreground_attribute_uses_context_color_and_preserves_alpha() {
    assert!(foreground_context_value().is_some());
    let pairs = text_attribute_pairs(1usize as _, 2usize as _, 3usize as _, 4usize as _);
    assert_eq!(
        pairs,
        [(1usize as _, 2usize as _), (3usize as _, 4usize as _)]
    );
    let (red, green, blue, alpha) = color_components(0x12ab34cd);
    assert_eq!(
        (red, green, blue, alpha),
        (18.0 / 255.0, 171.0 / 255.0, 52.0 / 255.0, 205.0 / 255.0)
    );
}

#[test]
fn flipped_bitmap_keeps_coretext_glyphs_upright() {
    let matrix = upright_text_matrix();
    assert_eq!(
        (matrix.a, matrix.b, matrix.c, matrix.d),
        (1.0, 0.0, 0.0, -1.0)
    );
    assert_eq!((matrix.tx, matrix.ty), (0.0, 0.0));
}

#[test]
fn coretext_raster_keeps_lower_stroke_of_l_in_lower_half() {
    let (width, height) = (64, 32);
    let mut pixels = vec![0; width * height * 4];
    draw_text_rgba(
        &mut pixels,
        width as u32,
        height as u32,
        width * 4,
        1,
        "L",
        (0, 0, width as u32, height as u32),
        0x0000_00FF,
        TextFace::System,
        TextEmphasis::Regular,
        18,
    )
    .unwrap();

    let row_ink = |start: usize, end: usize| {
        (start..end)
            .map(|y| {
                pixels[y * width * 4..(y + 1) * width * 4]
                    .chunks_exact(4)
                    .filter(|pixel| pixel[3] != 0)
                    .count()
            })
            .sum::<usize>()
    };
    assert!(row_ink(height / 2, height) > row_ink(0, height / 2));
}

#[test]
fn semibold_requests_descriptor_weight_and_regular_is_explicit_fallback() {
    assert_eq!(semibold_descriptor_weight(), 0.3);
    assert!(use_semibold(TextEmphasis::Semibold, true));
    assert!(!use_semibold(TextEmphasis::Semibold, false));
    assert!(!use_semibold(TextEmphasis::Regular, true));
}

#[test]
fn rejects_text_and_geometry_before_native_calls() {
    let mut pixels = vec![0; 4 * 4 * 4];
    assert_eq!(
        draw_text_rgba(
            &mut pixels,
            4,
            4,
            16,
            1,
            &"x".repeat(MAX_TEXT_BYTES + 1),
            (0, 0, 4, 4),
            0xffffffff,
            TextFace::System,
            TextEmphasis::Regular,
            12
        ),
        Err(TextError::InvalidText)
    );
    assert_eq!(
        draw_text_rgba(
            &mut pixels,
            4,
            4,
            16,
            1,
            "x",
            (3, 0, 2, 1),
            0xffffffff,
            TextFace::System,
            TextEmphasis::Regular,
            12
        ),
        Err(TextError::InvalidBounds)
    );
    assert_eq!(
        draw_text_rgba(
            &mut pixels,
            4,
            4,
            16,
            4,
            "x",
            (0, 0, 4, 4),
            0xffffffff,
            TextFace::System,
            TextEmphasis::Regular,
            12
        ),
        Err(TextError::InvalidBuffer)
    );
    assert_eq!(
        draw_text_rgba(
            &mut pixels,
            4,
            4,
            15,
            1,
            "x",
            (0, 0, 4, 4),
            0xffffffff,
            TextFace::System,
            TextEmphasis::Regular,
            12
        ),
        Err(TextError::InvalidBuffer)
    );
    assert_eq!(
        draw_text_rgba(
            &mut pixels,
            4097,
            4,
            16388,
            1,
            "x",
            (0, 0, 1, 1),
            0xffffffff,
            TextFace::System,
            TextEmphasis::Regular,
            12
        ),
        Err(TextError::InvalidBuffer)
    );
    assert_eq!(
        draw_text_rgba(
            &mut pixels,
            4,
            4,
            16,
            1,
            "x",
            (0, 0, 4, 4),
            0xffffffff,
            TextFace::System,
            TextEmphasis::Regular,
            0
        ),
        Err(TextError::InvalidSize)
    );
}
