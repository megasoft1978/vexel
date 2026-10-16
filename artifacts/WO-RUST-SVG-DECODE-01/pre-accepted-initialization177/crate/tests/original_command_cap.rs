mod support;
use support::decode;
use vexel_scene::{ErrorKind, LimitKind};
/// Exact input construction and return-category predicate from
/// tests/fuzz/shape/shape_fuzz.c::command_cap_probe. Uses the frozen4M cap.
#[test]
fn original_four_million_command_cap_probe() {
    let prefix = "<svg width=\"16\" height=\"16\"><path d=\"M0 0";
    let segment = " L1 1";
    let suffix = "\"/></svg>";
    let groups = 4 * 1024 * 1024;
    let mut input = String::with_capacity(prefix.len() + groups * segment.len() + suffix.len());
    input.push_str(prefix);
    for _ in 0..groups {
        input.push_str(segment);
    }
    input.push_str(suffix);
    assert_eq!(
        input.len(),
        prefix.len() + groups * segment.len() + suffix.len()
    );
    let error = decode(&input).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Limit(LimitKind::PathCommands));
}
