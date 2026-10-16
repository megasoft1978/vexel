mod support;
use support::*;
use vexel_scene::ErrorKind;
const INPUT_0: &str = "<svg width=\"16\" height=\"8\" fill=\"green\"><style>/* keep */\n.paint, rect.alt { FiLl: blue; } rect { fill: green; } #target.paint { FILL: RED ! IMPORTANT; } .paint { fill: red !important /* ; } */; } .outer { stroke: blue; stroke-width: 2; }</style><g class=\"outer\"><g><rect id=\"target\" class=\"paint extra\" x=\"1\" y=\"1\" width=\"4\" height=\"4\" fill=\"white\" style=\"fill: white\"/></g><rect class=\"paint\" x=\"7\" y=\"1\" width=\"4\" height=\"4\" style=\"fill: blue ! IMPORTANT /* ; } */; fill: white\"/><rect class=\"plain\" x=\"12\" y=\"1\" width=\"3\" height=\"4\" style=\"fill: white\"/></g></svg>";
const INPUT_1: &str = "<svg width=\"8\" height=\"8\"><style>rect { fill: green; } .bar { fill: red; } .unsupported g { fill: blue; }</style><rect class=\"foobar\" width=\"3\" height=\"3\"/><rect x=\"4\" width=\"3\" height=\"3\"/></svg>";
const INPUT_2: &str = "<svg width=\"16\" height=\"8\" fill=\"blue\"><style>.direct { fill: red; } .same { fill: red; } .same { fill: green; } .parent { fill: white; }</style><rect x=\"0\" y=\"0\" width=\"3\" height=\"3\"/><g class=\"parent\"><g><rect x=\"4\" y=\"0\" width=\"3\" height=\"3\"/></g></g><rect class=\"direct\" fill=\"blue\" x=\"8\" y=\"0\" width=\"3\" height=\"3\"/><rect class=\"same\" x=\"12\" y=\"0\" width=\"3\" height=\"3\"/></svg>";
const INPUT_3: &str = "<svg width=\"16\" height=\"16\"><g transform=\"skewX(nope)\"/></svg>";

#[test]
fn original_css_cascade_precedence_and_retained_bytes() {
    let g = decode(INPUT_0).unwrap();
    let r = g.root();
    assert_eq!(r.shapes.len(), 3);
    for (i, color) in [[255, 0, 0, 255], [0, 0, 255, 255], [255, 255, 255, 255]]
        .into_iter()
        .enumerate()
    {
        assert_eq!(r.edge_colors[r.shapes[i].edges.first], color)
    }
    let g = decode(INPUT_2).unwrap();
    let r = g.root();
    for (i, color) in [
        [0, 0, 255, 255],
        [255, 255, 255, 255],
        [255, 0, 0, 255],
        [0, 128, 0, 255],
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(r.edge_colors[r.shapes[i].edges.first], color)
    }
}
#[test]
fn original_class_tokens_and_initial_paint() {
    let g = decode(INPUT_1).unwrap();
    assert!(g.root().edge_colors.iter().all(|c| *c == [0, 128, 0, 255]));
    let g = decode("<svg width=\"8\" height=\"8\"><rect width=\"3\" height=\"3\"/></svg>").unwrap();
    assert_eq!(g.root().edge_colors[0], [0, 0, 0, 255]);
    let tokens = (0..256)
        .map(|i| format!("c{i}"))
        .chain(["target".to_owned()])
        .collect::<Vec<_>>()
        .join(" ");
    let terms = format!(
        ".{}",
        (0..17)
            .map(|i| format!("a{i}"))
            .collect::<Vec<_>>()
            .join(".")
    );
    let g=decode(&format!("<svg width=\"8\" height=\"8\"><style>rect {{ fill: green; }} .target {{ fill: red; }} {terms} {{ fill: blue; }}</style><rect class=\"{tokens}\" width=\"3\" height=\"3\"/></svg>")).unwrap();
    assert_eq!(g.root().edge_colors[0], [0, 128, 0, 255]);
}
#[test]
fn original_nine_invalid_paint_values_keep_exact_offsets() {
    for (p, v, kind) in [
        ("fill", "not-a-css-color", ErrorKind::Unsupported),
        ("fill-rule", "invalid", ErrorKind::Unsupported),
        ("stroke", "not-a-css-color", ErrorKind::Unsupported),
        ("stroke-width", "-1", ErrorKind::Number),
        ("stroke-linecap", "triangle", ErrorKind::Unsupported),
        ("stroke-linejoin", "invalid", ErrorKind::Unsupported),
        ("stroke-miterlimit", "0", ErrorKind::Number),
        ("stroke-dasharray", "0", ErrorKind::Number),
        ("stroke-dashoffset", "wat", ErrorKind::Number),
    ] {
        let source=format!("<svg width=\"8\" height=\"8\"><style>rect {{ {p}: {v}; }}</style><rect width=\"4\" height=\"4\"/></svg>");
        let e = decode(&source).unwrap_err();
        assert_eq!((e.kind, e.offset), (kind, source.find(v).unwrap()), "{p}");
    }
    let source="<svg width=\"8\" height=\"8\"><style>rect { stroke-width: nope; }</style><rect width=\"4\" height=\"4\"/></svg>";
    let e = decode(source).unwrap_err();
    assert_eq!(
        (e.kind, e.offset),
        (ErrorKind::Number, source.find("nope").unwrap())
    );
}
#[test]
fn original_height_and_transform_diagnostics() {
    for (source, marker, kind) in [
        (
            "<svg width=\"16\" height=\"bad\"/>",
            "bad",
            ErrorKind::Number,
        ),
        (
            "<svg width=\"16\" height=\"5000\"/>",
            "5000",
            ErrorKind::Viewport,
        ),
    ] {
        let e = decode(source).unwrap_err();
        assert_eq!((e.kind, e.offset), (kind, source.find(marker).unwrap()));
    }
    let e = decode(INPUT_3).unwrap_err();
    assert_eq!(
        (e.kind, e.offset),
        (ErrorKind::Unsupported, INPUT_3.find("skewX").unwrap())
    );
}
#[test]
fn original_w5_crisp_edges_and_extended_join_decode_inputs() {
    for (shape, join) in [
        (
            r#"<rect x="0.25" y="0.25" width="8.5" height="8.5" fill="red" shape-rendering="crispEdges"/>"#,
            None,
        ),
        (
            r#"<polyline points="2,14 8,2 14,14" fill="none" stroke="blue" stroke-width="3" stroke-linejoin="arcs"/>"#,
            Some(vexel_scene::Linejoin::Round),
        ),
        (
            r#"<polyline points="18,14 24,2 30,14" fill="none" stroke="green" stroke-width="3" stroke-linejoin="miter-clip"/>"#,
            Some(vexel_scene::Linejoin::Miter),
        ),
    ] {
        let g = decode(&format!("<svg width=\"32\" height=\"16\">{shape}</svg>")).unwrap();
        assert_eq!(g.root().shapes.len(), 1);
        if let Some(j) = join {
            assert_eq!(g.root().shapes[0].stroke.join, j)
        } else {
            assert!(g.root().shapes[0].stroke.crisp_edges)
        }
    }
}
