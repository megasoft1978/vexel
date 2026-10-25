//! Face tools through MCP: `mouth_set`, `lip_sync`, `eyes_set`, `blink`.

mod common;
mod face_support;

use common::{Rig, no_grants};
use face_support::{build, shapes};

fn source(rig: &Rig) -> String {
    String::from_utf8(rig.app.source_bytes().unwrap()).unwrap()
}

#[test]
fn mouth_set_stores_in_the_document_and_undoes() {
    let mut rig = Rig::new(no_grants());
    let face = build(&mut rig);
    let before = source(&rig);
    let reply = rig.doc_tool("mouth_set", &shapes(&face)).ok().to_owned();
    assert!(reply.contains("\"shapes\":3"), "{reply}");
    let stored = source(&rig);
    assert_eq!(stored.matches("id=\"vexel-mouths\"").count(), 1);
    assert!(stored.contains(&format!("\"rest\":\"{}\"", face.rest)));
    // Replacing keeps one element and one undo restores the first set.
    rig.doc_tool(
        "mouth_set",
        &format!(
            r#""character":"{}","shapes":{{"rest":"{}"}}"#,
            face.character, face.rest
        ),
    )
    .ok();
    assert_eq!(source(&rig).matches("vexel-mouths").count(), 1);
    rig.doc_tool("undo", "").ok();
    assert_eq!(source(&rig), stored);
    rig.doc_tool("undo", "").ok();
    assert_eq!(source(&rig), before);
}

#[test]
fn mouth_set_follows_a_renamed_shape_group() {
    let mut rig = Rig::new(no_grants());
    let face = build(&mut rig);
    rig.doc_tool("mouth_set", &shapes(&face)).ok();
    rig.doc_tool(
        "attribute_set",
        &format!(r#""elementId":"{}","name":"id","value":"mouth_o""#, face.o),
    )
    .ok();
    let text = source(&rig);
    assert!(text.contains("\"O\":\"mouth_o\""), "{text}");
    assert!(!text.contains(&format!("\"{}\"", face.o)));
    let reply = rig.doc_tool(
        "attribute_set",
        r#""elementId":"vexel-mouths","name":"id","value":"x""#,
    );
    assert!(reply.is_error);
}

#[test]
fn mouth_set_rejects_bad_input_without_changes() {
    let mut rig = Rig::new(no_grants());
    let face = build(&mut rig);
    let (c, r, a) = (&face.character, &face.rest, &face.a);
    let stray = {
        let g = common::element_id(
            rig.doc_tool(
                "element_add",
                r##""parentId":"root","template":"rect","attributes":{"x":"1","y":"1","width":"5","height":"5","fill":"#000000"}"##,
            )
            .ok(),
        );
        common::element_id(
            rig.doc_tool("element_group", &format!(r#""elementId":"{g}""#))
                .ok(),
        )
    };
    let cases = [
        format!(r#""character":"{c}","shapes":{{}}"#),
        format!(r#""character":"{c}","shapes":{{"A":"{a}"}}"#),
        format!(r#""character":"{c}","shapes":{{"rest":"{r}","Q":"{a}"}}"#),
        format!(r#""character":"{c}","shapes":{{"rest":"{r}","A":"nope"}}"#),
        format!(r#""character":"{c}","shapes":{{"rest":"{r}","A":"{stray}"}}"#),
        format!(r#""character":"{c}","shapes":{{"rest":"{r}","A":"{c}"}}"#),
        format!(r#""character":"missing","shapes":{{"rest":"{r}"}}"#),
        format!(r#""character":"{r}","shapes":{{"rest":"{a}"}}"#),
        format!(r#""character":"{c}","shapes":{{"rest":"{r}","A":5}}"#),
        format!(r#""character":"{c}","shapes":{{"rest":"{r}","rest":"{a}"}}"#),
        format!(r#""character":"{c}","shapes":{{"rest":"bad id"}}"#),
        format!(r#""character":"{c}","shapes":[1]"#),
        format!(r#""character":3,"shapes":{{"rest":"{r}"}}"#),
        format!(r#""character":"{c}""#),
    ];
    let state = rig.state();
    for args in &cases {
        let reply = rig.doc_tool("mouth_set", args);
        assert!(reply.is_error, "{args}: {}", reply.body);
    }
    assert_eq!(rig.state(), state, "rejected calls changed nothing");
}

#[test]
fn a_forged_mouth_element_is_not_trusted() {
    let mut rig = Rig::new(no_grants());
    let face = build(&mut rig);
    let reply = rig.doc_tool(
        "element_add_fragment",
        r#""parentId":"root","xml":"<rect id=\"vexel-mouths\" x=\"0\" y=\"0\" width=\"1\" height=\"1\"/>""#,
    );
    if reply.is_error {
        return; // the id is refused outright
    }
    let state = rig.state();
    let reply = rig.doc_tool("mouth_set", &shapes(&face));
    assert!(reply.is_error, "{}", reply.body);
    assert_eq!(rig.state(), state);
}
