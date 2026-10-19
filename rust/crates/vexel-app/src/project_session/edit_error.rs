/// Failure from a combined project edit.
///
/// `Operation` preserves the original editor operation error. `History`
/// reports a failure that happened before the operation was admitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionEditError<E> {
    Operation(E),
    History(super::SessionError),
}
