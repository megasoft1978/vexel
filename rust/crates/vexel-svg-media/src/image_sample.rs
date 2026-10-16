use super::image::ImageGeometry;
use vexel_geometry::{Affine, Point, affine_map_point};
use vexel_scene::ImagePixels;

/// Sample one placed image at a document-space point using the frozen half-open
/// bounds and bilinear interpolation of premultiplied RGBA8 pixels. Returned
/// color is straight RGBA8 for the compositor, with nearest-integer rounding.
pub fn sample_image(
    image: &ImagePixels,
    geometry: ImageGeometry,
    inverse_transform: Affine,
    x: f64,
    y: f64,
) -> Option<[u8; 4]> {
    if image.validate(0).is_err()
        || image.width == 0
        || image.height == 0
        || geometry.draw_width <= 0.
        || geometry.draw_height <= 0.
    {
        return None;
    }
    let local = affine_map_point(inverse_transform, Point { x, y })?;
    let right = geometry.x + geometry.width;
    let bottom = geometry.y + geometry.height;
    let draw_right = geometry.draw_x + geometry.draw_width;
    let draw_bottom = geometry.draw_y + geometry.draw_height;
    if ![local.x, local.y, right, bottom, draw_right, draw_bottom]
        .iter()
        .all(|v| v.is_finite())
        || local.x < geometry.x
        || local.y < geometry.y
        || local.x >= right
        || local.y >= bottom
        || local.x < geometry.draw_x
        || local.y < geometry.draw_y
        || local.x >= draw_right
        || local.y >= draw_bottom
    {
        return None;
    }
    let u = (local.x - geometry.draw_x) / geometry.draw_width;
    let v = (local.y - geometry.draw_y) / geometry.draw_height;
    let mut sx = u * f64::from(image.width) - 0.5;
    let mut sy = v * f64::from(image.height) - 0.5;
    sx = sx.clamp(0., f64::from(image.width - 1));
    sy = sy.clamp(0., f64::from(image.height - 1));
    let x0 = sx.floor() as u32;
    let y0 = sy.floor() as u32;
    let x1 = if x0 + 1 < image.width { x0 + 1 } else { x0 };
    let y1 = if y0 + 1 < image.height { y0 + 1 } else { y0 };
    let fx = sx - f64::from(x0);
    let fy = sy - f64::from(y0);
    let p00 = pixel(image, x0, y0)?;
    let p10 = pixel(image, x1, y0)?;
    let p01 = pixel(image, x0, y1)?;
    let p11 = pixel(image, x1, y1)?;
    let mut channels = [0.; 4];
    for (channel, out) in channels.iter_mut().enumerate() {
        let top = f64::from(p00[channel]) * (1. - fx) + f64::from(p10[channel]) * fx;
        let lower = f64::from(p01[channel]) * (1. - fx) + f64::from(p11[channel]) * fx;
        *out = top * (1. - fy) + lower * fy;
    }
    let alpha = (channels[3] + 0.5).floor().clamp(0., 255.) as u8;
    if alpha == 0 {
        return Some([0; 4]);
    }
    let mut result = [0; 4];
    // RGB is a fixed three-channel interpolation, not an input-sized scan.
    for channel in 0..3 {
        result[channel] = (channels[channel] * 255. / channels[3] + 0.5)
            .floor()
            .clamp(0., 255.) as u8;
    }
    result[3] = alpha;
    Some(result)
}

fn pixel(image: &ImagePixels, x: u32, y: u32) -> Option<[u8; 4]> {
    let offset = (y as usize)
        .checked_mul(image.row_stride)?
        .checked_add((x as usize).checked_mul(4)?)?;
    let end = offset.checked_add(4)?;
    let pixel = image.rgba.get(offset..end)?;
    Some([pixel[0], pixel[1], pixel[2], pixel[3]])
}
