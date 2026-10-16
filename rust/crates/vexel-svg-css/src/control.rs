use crate::CssError;
use vexel_base::{ControlledError, Stop, WorkControl, WorkKind};

pub(crate) struct NoStop;
impl WorkControl for NoStop {
    fn charge(&mut self, _: WorkKind, _: u64) -> Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> Result<(), Stop> {
        Ok(())
    }
}

pub(crate) fn original(error: CssError) -> ControlledError<CssError> {
    match error {
        CssError::OutOfMemory => ControlledError::OutOfMemory,
        _ => ControlledError::Original(error),
    }
}
pub(crate) fn legacy_error(error: ControlledError<CssError>) -> CssError {
    match error {
        ControlledError::Original(e) => e,
        ControlledError::OutOfMemory => CssError::OutOfMemory,
        _ => CssError::Limit,
    }
}
