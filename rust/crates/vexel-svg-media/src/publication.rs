//! Required production boundary to the decoder's common custom-geometry pipeline.
//! No implementation is supplied here: native providers and the decoder's actual
//! paint/definition resolvers must be explicitly composed downstream.
use vexel_base::WorkControl;
use vexel_geometry::Affine;
use vexel_scene::{
    CssPaint, DraftToken, ElementView, Result, SceneBuilder, SceneId, ShapeAdmission, ShapeId,
    SourceMap,
};
use vexel_svg_css::Property;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Text,
    Image,
}

/// Canonical source/style and independent use-host annotation. Geometry is in
/// scene coordinates; `transform` is the actual canonical-to-scene transform,
/// while `source_map.transform` remains the host annotation, without substitution.
pub struct PublicationContext<'src, 'style> {
    pub element: ElementView<'src>,
    /// Effective authored boundary (TextDecodeRequest.end for text).
    pub end: usize,
    pub style: &'style CssPaint<'src>,
    pub source_map: SourceMap,
    pub transform: Affine,
    pub scene: SceneId,
    pub kind: MediaKind,
}

/// Transferred, fallibly allocated geometry. The publisher owns it on entry,
/// including on error. The optional image is already admitted in this request.
/// Text preserves all contour boundaries. No default paint/colors are provided.
pub struct CustomGeometry {
    pub path: vexel_svg_path::PathDocument,
    pub source_map: SourceMap,
    pub image: Option<usize>,
}

pub struct PublishedStyle {
    pub shape: vexel_scene::Shape,
    pub edge_colors: Vec<[u8; 4]>,
}

/// Required adapter for the original traversal + shape_custom.c pipeline.
///
/// All methods are required. No neutral implementation or optional production
/// fallback exists. Callbacks may append/finalize ONLY the current request's
/// suffix using the supplied builder/draft. They must retain no owned resources,
/// admissions or mutable external state across calls; owned temporaries drop on
/// error. Resolver caches must be request-local or independently transactional.
/// Builder mutations are unconditionally rolled back by this crate on any
/// callback/provider/allocation/control/commit error. Errors propagate verbatim.
/// Callbacks must poll/charge all variable work and use fallible bounded storage.
///
/// `begin`: traversal opacity/filter/mask/blend/isolation validation/admission,
/// before font parsing/layout or image parsing/provider. Display has already
/// been checked. Hidden image skips its provider and prepare/publish after begin.
/// Text parses/layouts/admits/checks positions before visibility; successful runs
/// advance cursor even when hidden. Only nonzero outlines check visibility and
/// may invoke prepare/publish.
/// Hidden text skips geometry, not its provider or successful logical use.
/// Finish closes traversal effects for either media kind.
/// `prepare_geometry`: fill/currentColor + fill-rule validation only. The crate
/// then admits the shape before `prepare_admitted_geometry` performs fill-opacity
/// rounding, stroke/dashes/vector-effect/inverse validation, in C
/// order. Image: before media parsing/provider; text: after layout/positions and
/// before outlining, once per run with commands. No image gradient resolution.
/// `publish`: append the actual styled shape/colors retaining source_map; resolve
/// fill then stroke paint servers (pattern before gradient), then geometry-local
/// clip with actual bounds/inverse. Image keeps IMAGE paint and zero solid alpha.
/// Return its live shape handle; do not publish a neutral substitute on error.
/// `finish`: finalize opacity/filter/mask/blend/isolation and direct text
/// clipping over [first_shape, current shape count). This is the direct request
/// range; enclosing decoder traversal closes shared groups/clip only after
/// descendant tspans, retaining parent opacity semantics. Text custom clipping
/// must be deferred here, as original traversal removes clip-path per text run.
/// Original media does not attach markers. Other enclosing traversal effects
/// remain explicit decoder duties.
pub trait StylePublisher {
    fn begin(
        &self,
        context: &PublicationContext<'_, '_>,
        builder: &mut SceneBuilder,
        draft: &DraftToken,
        control: &mut dyn WorkControl,
    ) -> Result<()>;
    fn prepare_geometry(
        &self,
        context: &PublicationContext<'_, '_>,
        builder: &mut SceneBuilder,
        draft: &DraftToken,
        control: &mut dyn WorkControl,
    ) -> Result<()>;
    /// Fill-opacity/stroke validation after the crate reserves the shape slot.
    fn prepare_admitted_geometry(
        &self,
        context: &PublicationContext<'_, '_>,
        builder: &mut SceneBuilder,
        draft: &DraftToken,
        control: &mut dyn WorkControl,
    ) -> Result<()>;
    fn publish(
        &self,
        context: &PublicationContext<'_, '_>,
        geometry: &CustomGeometry,
        builder: &mut SceneBuilder,
        draft: &DraftToken,
        control: &mut dyn WorkControl,
    ) -> Result<PublishedStyle>;
    fn finish(
        &self,
        context: &PublicationContext<'_, '_>,
        first_shape: usize,
        builder: &mut SceneBuilder,
        draft: &DraftToken,
        control: &mut dyn WorkControl,
    ) -> Result<()>;
}

pub(crate) fn display_none(context: &PublicationContext<'_, '_>) -> Result<bool> {
    if let Some(value) = context.style.value(Property::Display) {
        if value.bytes.eq_ignore_ascii_case(b"none") {
            return Ok(true);
        }
        if ![b"inline".as_slice(), b"block", b"contents"]
            .iter()
            .any(|v| value.bytes.eq_ignore_ascii_case(v))
        {
            return Err(super::text::fail(
                vexel_scene::ErrorKind::Unsupported,
                value.span.offset,
            ));
        }
    }
    Ok(false)
}

pub(crate) fn visibility_hidden(context: &PublicationContext<'_, '_>) -> bool {
    context
        .style
        .value(Property::Visibility)
        .is_some_and(|value| {
            value.bytes.eq_ignore_ascii_case(b"hidden")
                || value.bytes.eq_ignore_ascii_case(b"collapse")
        })
}

pub(crate) fn validate_visibility(context: &PublicationContext<'_, '_>) -> Result<()> {
    if let Some(value) = context.style.value(Property::Visibility) {
        if !value.bytes.eq_ignore_ascii_case(b"visible") {
            return Err(super::text::fail(
                vexel_scene::ErrorKind::Unsupported,
                value.span.offset,
            ));
        }
    }
    Ok(())
}

/// Production image route; None means display:none/hidden/collapse. Validates
/// real source/ledger/scene/host metadata before any callback or provider call.
pub fn publish_image_request(
    request: vexel_scene::ImageDecodeRequest<'_, '_>,
    publisher: &dyn StylePublisher,
    builder: &mut SceneBuilder,
    draft: &DraftToken,
    control: &mut dyn WorkControl,
) -> Result<Option<ShapeId>> {
    request.validate_source_map(builder, control)?;
    let context = PublicationContext {
        element: request.element,
        end: request.element.end,
        style: request.style,
        source_map: request.source_map,
        transform: request.transform,
        scene: request.scene,
        kind: MediaKind::Image,
    };
    super::request::atomic_request(builder, draft, control, |builder, control, token| {
        if display_none(&context)? {
            return Ok(None);
        }
        let first = builder.scene(request.scene)?.shapes.len();
        publisher.begin(&context, builder, draft, control)?;
        ensure_shape_count(&context, builder, first)?;
        if visibility_hidden(&context) {
            let before_finish = builder.scene(request.scene)?.shapes.len();
            publisher.finish(&context, first, builder, draft, control)?;
            ensure_shape_count(&context, builder, before_finish)?;
            return Ok(None);
        }
        validate_visibility(&context)?;
        let admission = prepare_admission(publisher, &context, builder, draft, token, control)?;
        let viewport = builder.scene(request.scene)?.viewport;
        let plan = super::image_element(
            request.element,
            [viewport.view_width, viewport.view_height],
            request.provider,
            control,
        )?;
        let id = super::image::append_image_plan_mapped(
            plan,
            request.source_map,
            request.scene,
            request.transform,
            builder,
            draft,
            control,
            Some((&context, publisher)),
            Some(admission),
        )?;
        let before_finish = builder.scene(request.scene)?.shapes.len();
        publisher.finish(&context, first, builder, draft, control)?;
        ensure_shape_count(&context, builder, before_finish)?;
        Ok(Some(id))
    })
}

/// Production text/tspan route. Same state/cache/document-budget rollback as
/// the compatibility helper, with required common-style callbacks per run.
pub fn publish_text_request(
    request: vexel_scene::TextDecodeRequest<'_, '_>,
    publisher: &dyn StylePublisher,
    builder: &mut SceneBuilder,
    draft: &DraftToken,
    control: &mut dyn WorkControl,
) -> Result<bool> {
    super::text_geometry::render_text_request_impl(
        request,
        builder,
        draft,
        control,
        Some(publisher),
    )
}

/// Enforce live suffix publication and retained host/geometry ownership. A
/// delegate cannot return an earlier shape or report empty successful output.
pub(crate) fn publish_checked(
    publisher: &dyn StylePublisher,
    context: &PublicationContext<'_, '_>,
    geometry: CustomGeometry,
    admission: ShapeAdmission,
    builder: &mut SceneBuilder,
    draft: &DraftToken,
    control: &mut dyn WorkControl,
) -> Result<ShapeId> {
    let first = builder.scene(context.scene)?.shapes.len();
    let image_count = builder.scene(context.scene)?.images.len();
    if geometry.image.is_some_and(|index| index >= image_count) {
        return Err(super::text::fail(
            vexel_scene::ErrorKind::InvalidArgument,
            context.source_map.element.start,
        ));
    }
    if geometry.source_map != context.source_map {
        return Err(super::text::fail(
            vexel_scene::ErrorKind::InvalidArgument,
            context.source_map.element.start,
        ));
    }
    let style = publisher.publish(context, &geometry, builder, draft, control)?;
    if builder.scene(context.scene)?.shapes.len() != first
        || builder.scene(context.scene)?.images.len() != image_count
        || style.shape.source != geometry.source_map
        || style.shape.image != geometry.image
        || style.edge_colors.len() != geometry.path.edges.len()
    {
        return Err(super::text::fail(
            vexel_scene::ErrorKind::InvalidArgument,
            context.source_map.element.start,
        ));
    }
    let id = builder.append_admitted_shape(
        draft,
        &admission,
        vexel_scene::ShapeInput {
            shape: style.shape,
            path: geometry.path,
            edge_colors: style.edge_colors,
        },
        control,
    )?;
    let shape = *builder.scene(context.scene)?.shape(id).ok_or_else(|| {
        super::text::fail(
            vexel_scene::ErrorKind::InvalidArgument,
            context.source_map.element.start,
        )
    })?;
    builder.finalize_shape(draft, id, shape, control)?;
    Ok(id)
}

/// The crate owns the admission boundary, between fill-rule and opacity/stroke.
/// Text/image outlines are generated geometry, never canonical `<path d>` data.
pub(crate) fn prepare_admission(
    publisher: &dyn StylePublisher,
    context: &PublicationContext<'_, '_>,
    builder: &mut SceneBuilder,
    draft: &DraftToken,
    request: vexel_scene::RequestToken,
    control: &mut dyn WorkControl,
) -> Result<ShapeAdmission> {
    let count = builder.scene(context.scene)?.shapes.len();
    publisher.prepare_geometry(context, builder, draft, control)?;
    ensure_shape_count(context, builder, count)?;
    let admission = builder.admit_shape(
        draft,
        request,
        context.element.tokens()[context.element.element.0]
            .name
            .offset,
        control,
    )?;
    publisher.prepare_admitted_geometry(context, builder, draft, control)?;
    ensure_shape_count(context, builder, count)?;
    Ok(admission)
}

pub(crate) fn ensure_shape_count(
    context: &PublicationContext<'_, '_>,
    builder: &SceneBuilder,
    expected: usize,
) -> Result<()> {
    if builder.scene(context.scene)?.shapes.len() != expected {
        Err(super::text::fail(
            vexel_scene::ErrorKind::InvalidArgument,
            context.source_map.element.start,
        ))
    } else {
        Ok(())
    }
}
