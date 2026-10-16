use std::sync::{Arc, atomic::AtomicBool};
use vexel_base::{BudgetControl, Span};
use vexel_geometry::affine_identity;
use vexel_scene::*;
use vexel_svg_css::Property;
use vexel_svg_definitions::DefinitionPlanner;
use vexel_xml::TokenKind;

pub const SVG: &[u8] = br#"<svg width="20" height="20" viewBox="0 0 20 20"><defs style="fill:#ff0000">
<clipPath id="c" clipPathUnits="objectBoundingBox"><path d="M0 0L1 0L1 1Z" clip-rule="evenodd"/></clipPath>
<mask id="m" maskUnits="userSpaceOnUse" maskContentUnits="objectBoundingBox" x="10%" y="20%" width="50%" height="60%" style="mask-type:ALPHA"><path d="M0 0L1 0L1 1Z"/></mask>
<pattern id="p" x=".1" y=".2" width=".5" height=".25" patternUnits="objectBoundingBox" patternContentUnits="objectBoundingBox" patternTransform="translate(1 2)" viewBox="0 0 2 1"><path d="M0 0L2 0L2 1Z"/></pattern>
<marker id="mk" style="marker-start:url(#outside)" markerWidth="50%" markerHeight="25%" refX="1" refY="0" viewBox="0 0 2 1"><path style="marker-mid:url(#nested)" d="M0 0L2 0L2 1Z"/></marker>
<filter id="f" filterUnits="userSpaceOnUse" x="10%" y="5%" width="50%" height="60%"><feGaussianBlur stdDeviation="1"/></filter>
</defs><path id="owner" clip-path="url(#c)" fill="url(#p)" mask="url(#m)" filter="url(#f)" d="M0 0L10 0L10 10Z"/></svg>"#;

pub struct Fixture {
    pub source: SourceOwner,
    pub document: LexicalDocument,
    pub builder: SceneBuilder,
    pub scene: SceneId,
    pub owner: ShapeId,
    pub owner_element: ElementId,
    pub ledger: ChargeLedger,
}

pub fn fixture(xml: &[u8]) -> Fixture {
    let source = Arc::new(xml.to_vec());
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let document = LexicalDocument::parse(source.clone(), 8 * 1024 * 1024, &mut control).unwrap();
    let animation = vexel_svg_animation::parse_owned(&source, document.tokens()).unwrap();
    let viewport = Viewport {
        width: 20,
        height: 20,
        view_x: 0.0,
        view_y: 0.0,
        view_width: 20.0,
        view_height: 20.0,
        aspect: vexel_svg_values::Aspect {
            align_x: 0.5,
            align_y: 0.5,
            none: false,
            slice: false,
            set: true,
        },
    };
    let mut limits = Limits::default();
    limits.reserved_bytes = 32 * 1024 * 1024;
    let mut builder = SceneBuilder::new(source.clone(), animation, 1, viewport, limits).unwrap();
    let scene = builder.root_id();
    let tokens = document.tokens();
    let owner_element = tokens
        .iter()
        .position(|t| {
            matches!(t.kind, TokenKind::StartElement | TokenKind::EmptyElement)
                && span(source.as_slice(), t.local_name) == Some(b"path")
                && attribute(source.as_slice(), tokens, t, b"id")
                    .is_some_and(|s| span(source.as_slice(), s) == Some(b"owner"))
        })
        .unwrap();
    let d = attribute(source.as_slice(), tokens, &tokens[owner_element], b"d").unwrap();
    let path = vexel_svg_path::parse_path(
        source.as_slice(),
        Span {
            start: d.offset,
            end: d.offset + d.length,
        },
        affine_identity(),
        vexel_svg_path::ParseOptions::default(),
    )
    .unwrap();
    let mut shape = Shape::new(SourceMap {
        element: Span {
            start: tokens[owner_element].source.offset,
            end: tokens[owner_element].source.offset + tokens[owner_element].source.length,
        },
        transform: affine_identity(),
        transform_node: None,
    });
    shape.edges.count = path.edges.len();
    shape.subpaths.count = path.subpaths.len();
    let edge_colors = vec![[0, 0, 0, 255]; path.edges.len()];
    let owner = builder
        .append_shape(
            scene,
            ShapeInput {
                shape,
                path,
                edge_colors,
            },
            &mut control,
        )
        .unwrap();
    let ledger = ChargeLedger::new(&source, 32 * 1024 * 1024, 4096).unwrap();
    Fixture {
        source,
        document,
        builder,
        scene,
        owner,
        owner_element: ElementId(owner_element),
        ledger,
    }
}

pub fn span(source: &[u8], value: vexel_xml::XmlSpan) -> Option<&[u8]> {
    source.get(value.offset..value.offset.checked_add(value.length)?)
}

pub fn attribute(
    source: &[u8],
    tokens: &[vexel_xml::XmlToken],
    element: &vexel_xml::XmlToken,
    name: &[u8],
) -> Option<vexel_xml::XmlSpan> {
    let index = tokens
        .iter()
        .position(|token| std::ptr::eq(token, element))?;
    tokens
        .get(index + 1..)?
        .iter()
        .take_while(|t| t.kind == TokenKind::Attribute)
        .find(|t| t.prefix.length == 0 && span(source, t.local_name) == Some(name))
        .map(|t| t.value)
}

pub fn named_element(
    source: &[u8],
    tokens: &[vexel_xml::XmlToken],
    name: &[u8],
    id: &[u8],
) -> ElementId {
    ElementId(
        tokens
            .iter()
            .enumerate()
            .find_map(|(i, token)| {
                if !matches!(
                    token.kind,
                    TokenKind::StartElement | TokenKind::EmptyElement
                ) || span(source, token.local_name) != Some(name)
                {
                    return None;
                }
                attribute(source, tokens, token, b"id")
                    .filter(|value| span(source, *value) == Some(id))
                    .map(|_| i)
            })
            .unwrap(),
    )
}

pub fn clip_boundary_source(child_bytes: usize, definitions: usize) -> Vec<u8> {
    let opening = b"<svg width=\"1\" height=\"1\"><defs>";
    let definition_open = b"<clipPath id=\"c0\" clipPathUnits=\"objectBoundingBox\">";
    let comment_start = b"<!--";
    let child_tail = b"--><path d=\"M0 0L1 0L1 1Z\"/>";
    let definition_close = b"</clipPath>";
    let defs_close = b"</defs><path id=\"owner\" d=\"M0 0L1 0L1 1Z\"/></svg>";
    let fixed_child = comment_start.len() + child_tail.len();
    assert!(child_bytes >= fixed_child);
    let mut source = Vec::new();
    source
        .try_reserve_exact(
            opening.len()
                + definition_open.len()
                + child_bytes * definitions
                + definition_close.len() * definitions
                + defs_close.len(),
        )
        .unwrap();
    source.extend_from_slice(opening);
    for i in 0..definitions {
        if i == 0 {
            source.extend_from_slice(definition_open);
        } else {
            let mut open = b"<clipPath id=\"c0\" clipPathUnits=\"objectBoundingBox\">".to_vec();
            open[15] = b'0' + u8::try_from(i).unwrap();
            source.extend_from_slice(&open);
        }
        source.extend_from_slice(comment_start);
        source.resize(source.len() + child_bytes - fixed_child, b'b');
        source.extend_from_slice(child_tail);
        source.extend_from_slice(definition_close);
    }
    source.extend_from_slice(defs_close);
    source
}

pub fn clip_plan_result(xml: &[u8], definition_id: &[u8]) -> Result<()> {
    let mut fixture = fixture(xml);
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let mut shared = DecodeSource::new(&fixture.document, &mut control)?;
    let style = shared.compute_style(fixture.owner_element, None, &mut control)?;
    let mut active = ActiveDefinitions::default();
    let mut planner = DefinitionPlanner::new();
    let definition = named_element(
        &fixture.source,
        fixture.document.tokens(),
        b"clipPath",
        definition_id,
    );
    let mut context = DefinitionContext {
        shared: &mut shared,
        builder: &mut fixture.builder,
        scene: fixture.scene,
        active: &mut active,
        ledger: &mut fixture.ledger,
    };
    planner
        .plan(
            &mut context,
            DefinitionKind::Clip,
            definition,
            fixture.owner,
            affine_identity(),
            [0.0, 0.0, 1.0, 1.0],
            &style,
            &mut control,
        )
        .map(|_| ())
}

pub fn clip_plan_rules(xml: &[u8]) -> Result<Vec<Option<Vec<u8>>>> {
    let mut fixture = fixture(xml);
    let cancel = AtomicBool::new(false);
    let mut control = BudgetControl::new(&cancel, u64::MAX);
    let mut shared = DecodeSource::new(&fixture.document, &mut control)?;
    let style = shared.compute_style(fixture.owner_element, None, &mut control)?;
    let mut active = ActiveDefinitions::default();
    let mut planner = DefinitionPlanner::new();
    let definition = named_element(
        &fixture.source,
        fixture.document.tokens(),
        b"clipPath",
        b"c",
    );
    let mut context = DefinitionContext {
        shared: &mut shared,
        builder: &mut fixture.builder,
        scene: fixture.scene,
        active: &mut active,
        ledger: &mut fixture.ledger,
    };
    let plan = planner.plan(
        &mut context,
        DefinitionKind::Clip,
        definition,
        fixture.owner,
        affine_identity(),
        [0.0, 0.0, 10.0, 10.0],
        &style,
        &mut control,
    )?;
    Ok(plan
        .expansions
        .iter()
        .filter(|expansion| {
            span(
                &fixture.source,
                fixture.document.tokens()[expansion.range.first].local_name,
            ) != Some(&b"style"[..])
        })
        .map(|expansion| {
            expansion
                .style
                .value(Property::ClipRule)
                .map(|value| value.bytes.to_vec())
        })
        .collect())
}
