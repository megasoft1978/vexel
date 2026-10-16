//! Every predicate from original path_number_test.c main, through lexical,
//! traversal, actual animation installation and immutable scene publication.
mod support;
use support::decode;
use vexel_scene::ErrorKind;
#[test]
fn original_path_number_main_all_assertions_through_real_decoder_scene() {
    let compact = "<svg width='8' height='8'><path d='M-.4.6L1.2.4Z'/></svg>";
    let separated = "<svg width='8' height='8'><path d='M-0.4 0.6 L1.2 0.4 Z'/></svg>";
    let a = decode(compact).unwrap();
    let b = decode(separated).unwrap();
    assert_eq!(a.source(), compact.as_bytes());
    assert_eq!(b.source(), separated.as_bytes());
    let av = a.root();
    let bv = b.root();
    assert_eq!(av.path_numbers.len(), 4);
    assert_eq!(bv.path_numbers.len(), 4);
    assert_eq!(av.edges.len(), bv.edges.len());
    let expected = [-0.4, 0.6, 1.2, 0.4];
    let literals = ["-.4", ".6", "1.2", ".4"];
    let mut from = 0;
    for (i, number) in av.path_numbers.iter().enumerate() {
        let offset = from + compact[from..].find(literals[i]).unwrap();
        assert_eq!(number.source.start, offset);
        assert_eq!(number.source.end - number.source.start, literals[i].len());
        assert_eq!(
            &compact[number.source.start..number.source.end],
            literals[i]
        );
        assert!((number.value - expected[i]).abs() < 1e-12);
        assert!((bv.path_numbers[i].value - number.value).abs() < 1e-12);
        from = number.source.end;
    }
    for (left, right) in av.edges.iter().zip(bv.edges) {
        for (x, y) in [
            (left.a.x, right.a.x),
            (left.a.y, right.a.y),
            (left.b.x, right.b.x),
            (left.b.y, right.b.y),
        ] {
            assert!((x - y).abs() < 1e-12);
        }
    }
    let malformed = "<svg width='8' height='8'><path d='M.4..6'/></svg>";
    let error = decode(malformed).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Path);
    assert_eq!(error.offset, malformed.find(".4..6").unwrap());
}
