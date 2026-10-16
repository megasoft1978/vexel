//! Owned SVG text and embedded raster media primitives.
#![forbid(unsafe_code)]

mod image;
mod image_sample;
mod publication;
mod request;
pub use publication::{
    CustomGeometry, MediaKind, PublicationContext, PublishedStyle, StylePublisher,
    publish_image_request, publish_text_request,
};
mod text;
mod text_cache;
mod text_geometry;

pub use image::{
    ImageGeometry, ImageMime, ImagePayload, ImagePlan, MAX_ENCODED_IMAGE_BYTES,
    MAX_IMAGE_DIMENSION, MAX_IMAGE_URI_BYTES, MAX_TOTAL_IMAGE_PIXELS, append_image_plan,
    attribute as image_attribute, base64_value, decode_base64, decode_image, decode_image_request,
    image_element, image_geometry, parse_data_uri, release_image, span_is,
};
pub use image_sample::sample_image;
pub use text::{
    MAX_POSITION_VALUES, MAX_TEXT_BYTES, TextStyleValues, normalize_text, parse_families,
    parse_position_list, parse_size, parse_spacing, parse_text_style, parse_weight,
    parse_xml_space, text_attribute, text_span_equal,
};
pub use text_cache::{
    LayoutResult, MAX_GLYPHS, MAX_PATH_COMMANDS, TEXT_CACHE_BYTES, TEXT_CACHE_ENTRIES, TextCache,
    TextCacheKey, charge_outline, layout_shared as layout_shared_cache, release_layout,
};
pub use text_geometry::{
    render_text_request, text_geometry, text_path_geometry, text_path_geometry_at,
};

/// Convert an XML span into the scene crate's source map representation.
pub fn source_span(span: vexel_xml::XmlSpan) -> Option<vexel_base::Span> {
    vexel_base::Span::from_offset_length(span.offset, span.length)
}
