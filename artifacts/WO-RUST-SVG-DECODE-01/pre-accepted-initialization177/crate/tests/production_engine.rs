#[path = "support/production.rs"]
mod production;
mod support;
use production::*;
use vexel_scene::*;
#[test]
fn original_512_late_gradient_instances_use_production_cache_handoff() {
    let source=format!("<svg width=\"16\" height=\"16\">{}<defs><linearGradient id=\"late\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient></defs></svg>","<rect width=\"1\" height=\"1\" fill=\"url(#late)\"/>".repeat(512));
    let mut fixture = ProductionFixture::default();
    let (graph, n) = run_production(&source, &mut fixture).unwrap();
    let r = graph.root();
    assert_eq!(n, 0);
    assert_eq!(r.shapes.len(), 512);
    assert_eq!(fixture.resolves, 512);
    assert_eq!(r.stops.len(), 2);
    let ranges = r.fill_paints.iter().map(|p| p.stops).collect::<Vec<_>>();
    assert!(ranges.iter().all(|v| *v == ranges[0]));
}
#[test]
fn production_nested_patterns_seal_complete_and_resume_before_parent() {
    let source="<svg width=\"8\" height=\"8\"><defs><pattern id=\"outer\"><rect width=\"2\" height=\"2\" fill=\"url(#inner)\"/></pattern><pattern id=\"inner\"><rect width=\"2\" height=\"2\" fill=\"red\"/></pattern></defs><rect width=\"8\" height=\"8\" fill=\"url(#outer)\"/></svg>";
    let (g, n) = run_production(source, &mut ProductionFixture::default()).unwrap();
    assert_eq!(n, 2);
    let r = g.root();
    let p = r.fill_paints[r.shapes[0].fill_paint.unwrap()];
    assert_eq!(p.kind, PaintKind::Pattern);
    assert!(p.pattern.is_some());
    assert_eq!(p.pattern_inverse.a, 0.5);
    assert_eq!(p.pattern_inverse.d, 0.5);
}
#[test]
fn production_filter_real_empty_group_and_original_pre_geometry_failure_priority() {
    let source="<svg width=\"8\" height=\"8\"><defs><filter id=\"f\"/></defs><g filter=\"url(#f)\"/></svg>";
    let mut fixture = ProductionFixture::default();
    let (g, _) = run_production(source, &mut fixture).unwrap();
    assert_eq!(g.root().shapes.len(), 0);
    assert!(g.root().groups.iter().any(|g| g.filter.is_some()));
    assert_eq!(fixture.plans, 1);
    let source="<svg width=\"8\" height=\"8\"><defs><filter id=\"f\"/></defs><g filter=\"url(#f)\"><path d=\"Mbad\"/></g></svg>";
    fixture.fail_filter = true;
    assert_eq!(
        run_production(source, &mut fixture).unwrap_err().kind,
        ErrorKind::Unsupported
    );
}
#[test]
fn production_rejects_wrong_flavor_before_child_publication() {
    let mut fixture = ProductionFixture::default();
    fixture.invalid_flavor = true;
    let source="<svg width=\"8\" height=\"8\"><defs><clipPath id=\"c\"><rect width=\"2\" height=\"2\"/></clipPath></defs><rect width=\"4\" height=\"4\" clip-path=\"url(#c)\"/></svg>";
    assert_eq!(
        run_production(source, &mut fixture).unwrap_err().kind,
        ErrorKind::InvalidArgument
    );
}
#[test]
fn actual_marker_child_host_annotation_and_mask_sealed_content() {
    let marker="<svg width=\"8\" height=\"8\"><defs><marker id=\"m\"><path d=\"M0 0 L1 0 L0 1 Z\" fill=\"red\"/></marker></defs><path d=\"M1 1 L4 4\" marker-start=\"url(#m)\" marker-end=\"url(#m)\"/></svg>";
    let (g, _) = run_production(marker, &mut ProductionFixture::default()).unwrap();
    assert_eq!(g.root().shapes.len(), 2);
    let host = marker.rfind("<path").unwrap();
    assert_eq!(g.root().shapes[1].source.element.start, host);
    let mask="<svg width=\"8\" height=\"8\"><defs><mask id=\"m\"><rect width=\"2\" height=\"2\" fill=\"white\"/></mask></defs><rect width=\"4\" height=\"4\" mask=\"url(#m)\"/></svg>";
    let (g, _) = run_production(mask, &mut ProductionFixture::default()).unwrap();
    let r = g.root();
    assert_eq!(r.shapes.len(), 1);
    let group = r.groups.iter().find(|g| g.mask.is_some()).unwrap();
    let m = r.mask(group.mask.unwrap()).unwrap();
    assert_eq!(r.scene(m.content).unwrap().shapes.len(), 1);
}
#[test]
fn production_fill_and_stroke_pattern_channels_are_explicit() {
    for channel in [PaintChannel::Fill, PaintChannel::Stroke] {
        let style = if channel == PaintChannel::Fill {
            "fill=\"url(#p)\""
        } else {
            "fill=\"none\" stroke=\"url(#p)\""
        };
        let source=format!("<svg width=\"8\" height=\"8\"><defs><pattern id=\"p\"><rect width=\"2\" height=\"2\" fill=\"red\"/></pattern></defs><rect width=\"4\" height=\"4\" {style}/></svg>");
        let (g, n) = run_production(&source, &mut ProductionFixture::default()).unwrap();
        assert_eq!(n, 1);
        let s = g.root().shapes[0];
        if channel == PaintChannel::Fill {
            assert!(s.fill_paint.is_some());
            assert!(s.stroke_paint.is_none());
        } else {
            assert!(s.stroke_paint.is_some());
            assert!(s.fill_paint.is_none());
        }
    }
}
#[test]
fn nested_gradient_clip_and_marker_content_is_complete_before_pattern_request() {
    let input="<svg width=\"8\" height=\"8\"><defs><linearGradient id=\"g\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient><clipPath id=\"c\"><rect width=\"8\" height=\"8\"/></clipPath><marker id=\"m\"><path d=\"M0 0 L1 0 L0 1 Z\" fill=\"red\"/></marker><pattern id=\"p\"><g clip-path=\"url(#c)\"><rect width=\"2\" height=\"2\" fill=\"url(#g)\"/><path d=\"M1 1 L2 2\" marker-start=\"url(#m)\"/></g></pattern></defs><rect width=\"8\" height=\"8\" fill=\"url(#p)\"/></svg>";
    let mut fixture = ProductionFixture::default();
    let (g, n) = run_production(input, &mut fixture).unwrap();
    assert_eq!(n, 1);
    assert_eq!(fixture.resolves, 1);
    let r = g.root();
    let child = r.scene(fixture.pattern_children[0]).unwrap();
    assert_eq!(child.shapes.len(), 3);
    assert_eq!(child.stops.len(), 2);
    assert!(child.shapes.iter().all(|s| s.clip.is_some()));
    assert_eq!(r.shapes.len(), 1);
}
