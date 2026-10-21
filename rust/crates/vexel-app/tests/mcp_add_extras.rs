//! `element_add` extras (id, opacity, transform, gradientId), `element_group`
//! with `elementIds`, and `element_reparent`.

mod common;

use common::{Dir, Rig, element_id, policy_for};

fn rig() -> (Dir, Rig) {
    let dir = Dir::new();
    let rig = Rig::new(policy_for(&[&dir.0]));
    (dir, rig)
}

fn add(rig: &mut Rig, parent: &str, extra: &str) -> common::Reply {
    rig.doc_tool(
        "element_add",
        &format!(
            r##""parentId":"{parent}","template":"rect","attributes":{{"x":"1","y":"2","width":"30","height":"20","fill":"#336699"}}{extra}"##
        ),
    )
}

fn svg(rig: &mut Rig) -> String {
    rig.doc_tool("export_svg", "").ok().replace("\\\"", "\"")
}

#[test]
fn explicit_id_opacity_transform_and_gradient_in_one_undo() {
    let (_dir, mut rig) = rig();
    let gradient = rig
        .doc_tool(
            "gradient_add",
            r#""kind":"linear","stops":"0:#ff0000:1,1:#0000ff:1""#,
        )
        .ok()
        .to_owned();
    let gradient = gradient
        .split("\"gradientId\":\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_owned();
    let before = rig.state().source;
    let reply = add(
        &mut rig,
        "root",
        &format!(
            r#","id":"hero-body","opacity":0.5,"transform":{{"translateX":10,"rotate":30,"pivotX":5,"pivotY":5}},"gradientId":"{gradient}""#
        ),
    );
    assert_eq!(element_id(reply.ok()), "hero-body");
    let svg = svg(&mut rig);
    assert!(svg.contains("id=\"hero-body\""), "{svg}");
    assert!(
        svg.contains("opacity=\"0.5\"") && svg.contains("matrix("),
        "{svg}"
    );
    assert!(svg.contains(&format!("url(#{gradient})")), "{svg}");
    rig.doc_tool("undo", "").ok();
    assert_eq!(rig.state().source, before, "one undo reverts everything");
}

#[test]
fn rejected_extras_change_nothing() {
    let (_dir, mut rig) = rig();
    add(&mut rig, "root", r#","id":"taken""#).ok();
    let state = rig.state();
    for extra in [
        r#","id":"taken""#,
        r#","id":"9bad""#,
        r#","id":"has space""#,
        r#","opacity":2"#,
        r#","transform":{"spin":1}"#,
        r#","gradientId":"missing""#,
        r#","paint":"stroke""#,
    ] {
        let reply = add(&mut rig, "root", extra);
        assert!(reply.is_error, "{extra}: {}", reply.body);
        assert_eq!(rig.state(), state, "{extra}");
    }
}

#[test]
fn auto_ids_skip_explicit_ids() {
    let (_dir, mut rig) = rig();
    add(&mut rig, "root", r#","id":"vexel-object-1""#).ok();
    let reply = add(&mut rig, "root", "");
    assert_eq!(element_id(reply.ok()), "vexel-object-2");
}

#[test]
fn group_wraps_several_siblings_and_reparent_moves() {
    let (_dir, mut rig) = rig();
    let ids: Vec<String> = (0..3).map(|n| rig.add_rect(n * 10, 0, "#112233")).collect();
    let both = format!(r#""elementIds":["{}","{}"]"#, ids[0], ids[1]);
    let group = element_id(rig.doc_tool("element_group", &both).ok());
    let tree = svg(&mut rig);
    let at = |id: &str| tree.find(&format!("id=\"{id}\"")).unwrap();
    assert!(at(&group) < at(&ids[0]) && at(&ids[0]) < at(&ids[1]) && at(&ids[1]) < at(&ids[2]));
    let state = rig.state();
    for bad in [
        r#""elementIds":[]"#.to_owned(),
        format!(r#""elementIds":["{}"],"elementId":"{}""#, ids[0], ids[0]),
        format!(r#""elementIds":["{}","nope"]"#, ids[0]),
    ] {
        assert!(rig.doc_tool("element_group", &bad).is_error, "{bad}");
    }
    assert_eq!(rig.state(), state);

    rig.doc_tool(
        "element_reparent",
        &format!(r#""elementId":"{}","parentId":"{group}","index":0"#, ids[2]),
    )
    .ok();
    let tree = svg(&mut rig);
    let at = |id: &str| tree.find(&format!("id=\"{id}\"")).unwrap();
    assert!(
        at(&group) < at(&ids[2]) && at(&ids[2]) < at(&ids[0]),
        "{tree}"
    );
    rig.doc_tool("undo", "").ok();
    assert_eq!(
        svg(&mut rig).find(&format!("id=\"{}\"", ids[2])),
        Some(tree_pos_after_group(&svg(&mut rig), &group, &ids[2]))
    );
    let cycle = rig.doc_tool(
        "element_reparent",
        &format!(r#""elementId":"{group}","parentId":"{group}""#),
    );
    assert!(cycle.error().contains("element_reparent"), "{}", cycle.body);
    let missing = rig.doc_tool(
        "element_reparent",
        r#""elementId":"nope","parentId":"root""#,
    );
    assert!(
        missing.error().contains("not an element id"),
        "{}",
        missing.body
    );
}

fn tree_pos_after_group(tree: &str, group: &str, id: &str) -> usize {
    assert!(
        tree.find(&format!("id=\"{group}\"")).unwrap()
            < tree.find(&format!("id=\"{id}\"")).unwrap()
    );
    tree.find(&format!("id=\"{id}\"")).unwrap()
}
