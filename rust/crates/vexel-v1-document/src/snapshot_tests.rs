use crate::V1Document;

#[test]
fn edit_snapshot_shares_an_immutable_view_of_source_bytes() {
    let document = V1Document::new(32, 24).expect("document");
    let expected = document.source().expect("source");
    let snapshot = document.edit_snapshot().expect("snapshot");
    let mut materialized = vec![0; snapshot.byte_length()];
    let mut length = 0;

    assert!(snapshot.materialize_into(&mut materialized, &mut length));
    assert_eq!(length, expected.len());
    assert_eq!(materialized, expected);
}
