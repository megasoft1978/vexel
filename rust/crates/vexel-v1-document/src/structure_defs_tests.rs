use super::*;

const HEAD: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"100\">";

fn doc(body: &str) -> V1Document {
    V1Document::from_source(format!("{HEAD}{body}</svg>").as_bytes()).unwrap()
}
fn text(d: &V1Document) -> String {
    String::from_utf8(d.source().unwrap()).unwrap()
}
fn gradient(d: &mut V1Document) -> String {
    let stops = [
        GradientStop {
            offset: 0.0,
            color: "#ff0000".to_owned(),
            opacity: 1.0,
        },
        GradientStop {
            offset: 1.0,
            color: "#0000ff".to_owned(),
            opacity: 1.0,
        },
    ];
    d.add_linear_gradient(
        0.0,
        0.0,
        1.0,
        0.0,
        GradientUnits::ObjectBoundingBox,
        Spread::Pad,
        &stops,
    )
    .unwrap()
}
/// The start tag of the element whose id attribute is `id` (either quote style).
fn start_tag<'a>(s: &'a str, id: &str) -> &'a str {
    let at = [format!("id=\"{id}\""), format!("id='{id}'")]
        .iter()
        .find_map(|needle| s.find(needle.as_str()))
        .unwrap();
    let tag_start = s[..at].rfind('<').unwrap();
    &s[tag_start..tag_start + s[tag_start..].find('>').unwrap()]
}
/// The `url(#...)` target of `attribute` in a start tag.
fn url_in(tag: &str, attribute: &str) -> String {
    let at = tag.find(&format!(" {attribute}=")).unwrap();
    let value = &tag[at..];
    let start = value.find("url(#").unwrap() + 5;
    value[start..].split(')').next().unwrap().to_owned()
}
/// The first `<name ` start tag after the element whose id is `anchor`.
fn child_tag<'a>(s: &'a str, anchor: &str, name: &str) -> &'a str {
    let from = s.find(&format!("id='{anchor}'")).unwrap();
    let at = from + s[from..].find(&format!("<{name} ")).unwrap();
    &s[at..at + s[at..].find('>').unwrap()]
}

#[test]
fn duplicate_clones_gradient_so_stop_edits_stay_on_the_copy() {
    let mut d = doc("<rect id='a' width='10' height='10'/>");
    let g = gradient(&mut d);
    d.set_fill_ref("a", &g).unwrap();
    let copy = d.duplicate("a").unwrap();
    let clone = url_in(start_tag(&text(&d), &copy), "fill");
    assert_ne!(clone, g);
    d.set_gradient_stop(&clone, 0, None, Some("#00ff00"), None)
        .unwrap();
    let s = text(&d);
    assert_eq!(url_in(start_tag(&s, "a"), "fill"), g);
    assert_eq!(url_in(start_tag(&s, &copy), "fill"), clone);
    assert!(s.contains(&format!(
        "<stop id=\"{g}-stop-0\" offset=\"0\" stop-color=\"#ff0000\""
    )));
    assert_eq!(s.matches("#00ff00").count(), 1);
}

#[test]
fn duplicate_clones_clip_path_and_keeps_original_reference() {
    let mut d = doc("<rect id='a' width='10' height='10'/>");
    let clip = d.add_clip_rect(0.0, 0.0, 5.0, 5.0).unwrap();
    d.set_clip("a", Some(&clip)).unwrap();
    let copy = d.duplicate("a").unwrap();
    let s = text(&d);
    let clone = url_in(start_tag(&s, &copy), "clip-path");
    assert_ne!(clone, clip);
    assert_eq!(s.matches("<clipPath").count(), 2);
    assert_eq!(url_in(start_tag(&s, "a"), "clip-path"), clip);
    d.set_clip(&copy, None).unwrap();
    assert_eq!(url_in(start_tag(&text(&d), "a"), "clip-path"), clip);
}

#[test]
fn duplicate_group_clones_each_referenced_def_once_and_undo_restores() {
    let mut d = doc("<g id='g'><rect id='a' width='10' height='10'/><circle id='c' r='4'/></g>");
    let g1 = gradient(&mut d);
    let clip = d.add_clip_rect(0.0, 0.0, 9.0, 9.0).unwrap();
    d.set_fill_ref("a", &g1).unwrap();
    d.set_stroke_ref("a", &g1).unwrap();
    d.set_fill_ref("c", &g1).unwrap();
    d.set_clip("c", Some(&clip)).unwrap();
    let before = d.source().unwrap();
    let count = d.history_count();

    let copy = d.duplicate("g").unwrap();
    assert_eq!(d.history_count(), count + 1);
    let s = text(&d);
    assert_eq!(s.matches("<linearGradient").count(), 2);
    assert_eq!(s.matches("<clipPath").count(), 2);
    let anchor = format!("{copy}");
    let rect = child_tag(&s, &anchor, "rect");
    let circle = child_tag(&s, &anchor, "circle");
    let fill = url_in(rect, "fill");
    assert_ne!(fill, g1);
    assert_eq!(url_in(rect, "stroke"), fill);
    assert_eq!(url_in(circle, "fill"), fill);
    assert_ne!(url_in(circle, "clip-path"), clip);
    assert_eq!(url_in(start_tag(&s, "a"), "fill"), g1);
    assert_eq!(url_in(start_tag(&s, "c"), "clip-path"), clip);

    d.undo().unwrap();
    assert_eq!(d.source().unwrap(), before);
    d.redo().unwrap();
    assert_eq!(text(&d), s);
}

#[test]
fn duplicate_leaves_unknown_references_untouched() {
    let mut d = doc("<rect id='a' width='1' height='1' fill='url(#nowhere)'/>");
    let copy = d.duplicate("a").unwrap();
    assert_eq!(url_in(start_tag(&text(&d), &copy), "fill"), "nowhere");
    assert_eq!(d.history_count(), 1);
}
