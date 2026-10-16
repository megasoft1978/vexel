//! `<image>` decoding for the production engine (WO-RUST-CORPUS-UNSUPPORTED-01).
//!
//! The decoder already opened the media request, so the plan from
//! `vexel_svg_media::image_element` (data-URI payload decoded by the session's
//! `MediaProvider`) is published here as one rectangle shape carrying the
//! image, as the media crate's own geometry-only helper does. Hidden images
//! fail closed.

use vexel_base::WorkControl;
use vexel_geometry::Point;
use vexel_scene::*;
use vexel_svg_decode::GeometryBuilder;
use vexel_svg_media as media;

fn fail(kind: ErrorKind, offset: usize) -> Diagnostic {
    Diagnostic { kind, offset }
}

pub(crate) fn decode(
    request: ImageDecodeRequest<'_, '_>,
    builder: &mut SceneBuilder,
    draft: &DraftToken,
    control: &mut dyn WorkControl,
) -> Result<ShapeId> {
    // For an empty `<image/>` the decoder's view ends at the element token, which
    // hides its attributes from the media crate; widen it (scans stop at the
    // first non-attribute token).
    let mut element = request.element;
    element.end = element.tokens().len();
    let own = element
        .tokens()
        .get(element.element.0)
        .map(|t| (t.name.offset, t.source))
        .ok_or_else(|| fail(ErrorKind::InvalidArgument, 0))?;
    let viewport = builder
        .traversal_scene(request.scene)?
        .viewport
        .ok_or_else(|| fail(ErrorKind::InvalidArgument, own.0))?;
    let style = vexel_svg_decode::parse_style(
        element.source(),
        request.style,
        viewport.view_width.hypot(viewport.view_height) / 2.0f64.sqrt(),
        request.transform,
        viewport,
        control,
    )?;
    if !style.visible || style.display_none {
        return Err(fail(ErrorKind::Unsupported, own.0));
    }
    let plan = media::image_element(
        element,
        [viewport.view_width, viewport.view_height],
        request.provider,
        control,
    )?;
    let geometry = plan.geometry;
    let image = match plan.pixels {
        Some(pixels) => Some(builder.append_image(
            request.scene,
            Image {
                pixels,
                x: geometry.x,
                y: geometry.y,
                width: geometry.width,
                height: geometry.height,
                draw_x: geometry.draw_x,
                draw_y: geometry.draw_y,
                draw_width: geometry.draw_width,
                draw_height: geometry.draw_height,
            },
            control,
        )?),
        None => None,
    };
    // The legacy renderer paints only where the sample lies inside both the
    // element rectangle and the aspect-fitted draw rectangle, so the shape's
    // geometry is their intersection.
    let (left, top) = (
        geometry.x.max(geometry.draw_x),
        geometry.y.max(geometry.draw_y),
    );
    let right = (geometry.x + geometry.width).min(geometry.draw_x + geometry.draw_width);
    let bottom = (geometry.y + geometry.height).min(geometry.draw_y + geometry.draw_height);
    let (right, bottom) = (right.max(left), bottom.max(top));
    let corners = [
        Point { x: left, y: top },
        Point { x: right, y: top },
        Point {
            x: right,
            y: bottom,
        },
        Point { x: left, y: bottom },
    ];
    let mut outline = GeometryBuilder::new(request.transform, own.0)?;
    for i in 0..4 {
        outline.add_edge(corners[i], corners[(i + 1) % 4], control)?;
    }
    let path = outline.finish(true, true, 4).path;
    let mut shape = Shape::new(request.source_map);
    shape.image = image;
    shape.stroke_closed = true;
    if let Some(inverse) = vexel_svg_decode::affine_inverse(request.transform) {
        shape.stroke_inverse = inverse;
    }
    let id = builder.append_shape(
        request.scene,
        ShapeInput {
            shape,
            path,
            edge_colors: vec![[0; 4]; 4],
        },
        control,
    )?;
    let published = *builder
        .traversal_scene(request.scene)?
        .shape(id)
        .ok_or_else(|| fail(ErrorKind::InvalidArgument, own.0))?;
    builder.finalize_shape(draft, id, published, control)?;
    Ok(id)
}
