#![allow(dead_code)]
use std::sync::Arc;
use vexel_base::{Stop, WorkControl, WorkKind};
use vexel_scene::*;
use vexel_xml::TokenKind;

pub struct Run;
impl WorkControl for Run {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> { Ok(()) }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> { Ok(()) }
}

pub struct Fixture {
    pub lex: LexicalDocument,
    pub builder: SceneBuilder,
}
impl Fixture {
    pub fn new(source: &[u8], limits: Limits) -> Self {
        let owner = Arc::new(source.to_vec());
        let lex = LexicalDocument::parse(owner.clone(), 1 << 20, &mut Run).unwrap();
        let animation = vexel_svg_animation::parse_owned(&owner, lex.tokens()).unwrap();
        let builder = SceneBuilder::new(owner, animation, 1, Viewport {
            width: 100, height: 100, view_x: 0., view_y: 0.,
            view_width: 100., view_height: 100., aspect: Default::default(),
        }, limits).unwrap();
        Self { lex, builder }
    }
    pub fn index(&self, name: &[u8]) -> usize {
        self.lex.tokens().iter().position(|t| {
            matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement)
                && t.local_name.slice(self.lex.source()) == Some(name)
        }).unwrap()
    }
    pub fn map(&self, index: usize) -> SourceMap {
        let span = self.lex.tokens()[index].source;
        SourceMap {
            element: Span::from_offset_length(span.offset, span.length).unwrap(),
            transform: vexel_geometry::affine_identity(), transform_node: None,
        }
    }
    pub fn prefix_input(&self) -> ShapeInput {
        let source = self.lex.source();
        let literal = b"M0 0L1 0L1 1Z";
        let first = source.windows(literal.len()).position(|w| w == literal).unwrap();
        let path = vexel_svg_path::parse_path(source, Span {
            start: first, end: first + literal.len(),
        }, vexel_geometry::affine_identity(), Default::default()).unwrap();
        let edge_colors = vec![[0; 4]; path.edges.len()];
        ShapeInput { shape: Shape::new(self.map(self.index(b"path"))), path, edge_colors }
    }
}

#[derive(Debug)]
pub struct Captured {
    pub text: Vec<u8>, pub families: Vec<Vec<u8>>, pub size: f64,
    pub weight: f64, pub spacing: f64, pub style: TextStyle,
}
#[derive(Default)]
pub struct Provider {
    pub seen: Vec<Captured>,
    pub image_calls: usize,
    pub fail_text: Option<MediaError>,
}
pub fn command(verb: TextVerb, x: f64, y: f64) -> TextCommand {
    TextCommand { verb, values: [x, y, 0., 0., 0., 0.] }
}
pub fn rectangle() -> Vec<TextCommand> {
    vec![command(TextVerb::Move, 0., 0.), command(TextVerb::Line, 4., 0.),
         command(TextVerb::Line, 4., 6.), command(TextVerb::Line, 0., 6.),
         command(TextVerb::Close, 0., 0.)]
}
impl MediaProvider for Provider {
    fn layout_text(&mut self, r: TextRequest<'_>, _: &mut dyn WorkControl)
        -> std::result::Result<TextLayout, MediaError>
    {
        self.seen.push(Captured { text: r.text.to_vec(),
            families: r.font_families.iter().map(|v| v.to_vec()).collect(),
            size: r.size_points, weight: r.weight, spacing: r.letter_spacing_points, style: r.style });
        if let Some(error) = self.fail_text { return Err(error); }
        let glyphs = std::str::from_utf8(r.text).unwrap().chars().enumerate().map(|(i, c)| TextGlyph {
            glyph_id: Some(1), x: i as f64 * 6., y: 0., advance: 6.,
            commands: TableRange { first: 0, count: if c == ' ' { 0 } else { 5 } },
        }).collect::<Vec<_>>();
        Ok(TextLayout { advance_width: glyphs.len() as f64 * 6., glyphs,
            commands: rectangle(), ascent: 6., descent: 1., leading: 0. })
    }
    fn decode_image(&mut self, _: &[u8], _: &mut dyn WorkControl)
        -> std::result::Result<ImagePixels, MediaError>
    {
        self.image_calls += 1;
        Ok(ImagePixels { rgba: vec![40, 120, 200, 255], byte_count: 4,
            row_stride: 4, width: 1, height: 1 })
    }
}

pub fn view(lex: &LexicalDocument, index: usize) -> ElementView<'_> {
    DecodeSource::new(lex, &mut Run).unwrap()
        .element(ElementId(index), lex.tokens().len(), &mut Run).unwrap()
}
