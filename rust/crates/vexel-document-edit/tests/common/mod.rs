#![allow(dead_code)]
use std::sync::Arc;
use vexel_document_edit::*;
pub fn create(source: &[u8]) -> Editor {
    let r = Editor::create(&Arc::new(source.to_vec()), EditLimits::default());
    assert_eq!(r.error, EditError::Ok, "{:?}", r.diagnostic);
    r.editor.unwrap()
}
pub fn bytes(edit: &Editor) -> Vec<u8> {
    let mut out = vec![0; edit.byte_length()];
    assert_eq!(edit.materialize(&mut out), Ok(out.len()));
    out
}
pub fn contains(bytes: &[u8], needle: &[u8]) -> bool {
    bytes.windows(needle.len()).any(|w| w == needle)
}
pub fn valid(bytes: &[u8]) {
    let (_, p) = vexel_xml::tokenize_with_capacity(bytes, vexel_xml::XmlLimits::default(), 256);
    assert_eq!(p.diagnostic.category, vexel_xml::XmlError::None);
}
pub fn id(edit: &Editor, id: &[u8]) -> DocumentHandle {
    edit.find_id(id).expect("fixture ID")
}
pub fn set(edit: &mut Editor, id_value: &[u8], name: &[u8], value: &[u8]) {
    let node = id(edit, id_value);
    edit.set_attribute(node, name, value).unwrap();
}
