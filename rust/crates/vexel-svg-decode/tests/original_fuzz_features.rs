#[path = "support/original_fuzz_inputs.rs"]
mod inputs;
/// Exact pure campaign feature classification from the frozen original harness.
fn identify_features(s: &str) -> [bool; 22] {
    [
        s.contains("<path") && s.contains("stroke="),
        s.contains("rx=") || s.contains("ry="),
        ["rgb(", "hsl(", "currentColor", "rebeccapurple"]
            .iter()
            .any(|x| s.contains(x)),
        ["skewX(", "skewY(", "rotate(", "matrix("]
            .iter()
            .any(|x| s.contains(x)),
        s.contains("opacity=") || s.contains("opacity:"),
        s.contains("<svg ") && s.contains("viewBox="),
        ["Gradient", "gradient", "url(#"]
            .iter()
            .any(|x| s.contains(x)),
        s.contains("<style>"),
        ["em\"", "mm", "cm\"", "pt\"", "in\""]
            .iter()
            .any(|x| s.contains(x)),
        s.contains("<text") || s.contains("<tspan"),
        s.contains("<image") || s.contains("data:image/"),
        s.contains("<mask") || s.contains("mask="),
        s.contains("<pattern") || s.contains("pattern"),
        s.contains("<marker") || s.contains("marker-"),
        s.contains("<filter") || s.contains("filter="),
        ["mix-blend-mode", "isolation", "<feBlend"]
            .iter()
            .any(|x| s.contains(x)),
        s.contains("<switch"),
        s.contains("<a>"),
        s.contains("vector-effect"),
        s.contains("paint-order"),
        s.contains("crispEdges"),
        s.contains("stroke-linejoin=\"arcs\"") || s.contains("stroke-linejoin=\"miter-clip\""),
    ]
}
#[test]
fn original_whole_shape_cases_retain_all_twenty_two_feature_classes() {
    let mut counts = [0usize; 22];
    for source in inputs::CASES {
        for (i, has) in identify_features(source).into_iter().enumerate() {
            counts[i] += usize::from(has);
        }
    }
    for (i, count) in counts.into_iter().enumerate() {
        assert!(count > 0, "original feature{i} disappeared");
    }
    assert_eq!(identify_features(""), [false; 22]);
    assert!(!identify_features("stroke=\"red\"")[0]);
    assert!(!identify_features("<svg>")[5]);
    assert!(!identify_features("viewBox=\"0 0 1 1\"")[5]);
}
