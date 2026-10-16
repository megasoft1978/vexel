#[path = "support/production.rs"]
mod production;
mod support;
use production::*;
use support::decode;
use vexel_scene::{ErrorKind, LimitKind};
#[test]
fn original_decode_job_total_exact_one_megabyte_and_plus_one() {
    let head = "<g id=\"big\"><!--";
    let end = "--></g>";
    let big = 64 * 1024 - 1;
    let target = format!("{head}{}{end}", "a".repeat(big - head.len() - end.len()));
    assert_eq!(target.len(), big);
    for (id, tiny, ok) in [("tiny12", 16, true), ("tiny123", 17, false)] {
        let small = format!("<g id=\"{id}\"/>");
        assert_eq!(small.len(), tiny);
        assert_eq!(16 * big + tiny, 1024 * 1024 + usize::from(!ok));
        let input = format!(
            "<svg width=\"1\" height=\"1\"><defs>{target}{small}</defs>{}<use href=\"#{id}\"/></svg>",
            "<use href=\"#big\"/>".repeat(16)
        );
        let decoded = decode(&input);
        if ok {
            assert!(decoded.unwrap().root().shapes.is_empty());
        } else {
            assert_eq!(
                decoded.unwrap_err().kind,
                ErrorKind::Limit(LimitKind::References)
            );
        }
    }
}
#[test]
fn original_group_clip_resolves_once_and_shares_one_owned_geometry() {
    for count in [1, 16] {
        let input = format!(
            "<svg width=\"8\" height=\"8\"><defs><clipPath id=\"c\" clipPathUnits=\"objectBoundingBox\"><rect width=\"0.5\" height=\"1\"/></clipPath></defs><g clip-path=\"url(#c)\">{}</g></svg>",
            "<rect width=\"4\" height=\"4\" fill=\"red\"/>".repeat(count)
        );
        let mut fixture = ProductionFixture::default();
        let (g, _) = run_production(&input, &mut fixture).unwrap();
        let r = g.root();
        assert_eq!(r.shapes.len(), count);
        assert_eq!(fixture.plans, 1);
        assert_eq!(r.clips.len(), 1);
        assert_eq!(r.clips[0].geometry.edges.len(), 4);
        let id = r.shapes[0].clip;
        assert!(r.shapes.iter().all(|s| s.clip == id));
    }
}
#[test]
fn original_long_viewbox_values_use_canonical_source_without_wrapper_reparse() {
    let input = "<svg width=\"4096\" height=\"4096\" viewBox=\"-1234.1234567891 -1234.1234567891 4096.1234567891 4096.1234567891\"><defs><clipPath id=\"c\" clipPathUnits=\"userSpaceOnUse\"><rect width=\"1\" height=\"1\"/></clipPath></defs><rect width=\"1\" height=\"1\" clip-path=\"url(#c)\"/></svg>";
    let (g, _) = run_production(input, &mut ProductionFixture::default()).unwrap();
    assert_eq!(g.source(), input.as_bytes());
    assert_eq!(g.root().viewport.view_x, -1234.1234567891);
    // Preserve original byte-decimal arithmetic, not Rust literal rounding.
    let number = "4096.1234567891";
    let expected = vexel_svg_decode::read_number(
        input.as_bytes(),
        vexel_xml::XmlSpan {
            offset: input.find(number).unwrap(),
            length: number.len(),
        },
    )
    .unwrap();
    assert_eq!(g.root().viewport.view_width, expected);
}
