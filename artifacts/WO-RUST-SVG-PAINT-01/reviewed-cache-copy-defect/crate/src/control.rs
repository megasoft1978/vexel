use vexel_base::{Stop, WorkControl, WorkKind};
use vexel_scene::{ChargeLedger, Diagnostic, ErrorKind, Result};

/// Couples accepted job accounting to caller cancellation/work control.
pub(crate) struct PaintControl<'a> {
    pub(crate) ledger: &'a mut ChargeLedger,
    pub(crate) outer: &'a mut dyn WorkControl,
    failure: Option<Diagnostic>,
}

impl<'a> PaintControl<'a> {
    pub(crate) fn new(ledger: &'a mut ChargeLedger, outer: &'a mut dyn WorkControl) -> Self {
        Self {
            ledger,
            outer,
            failure: None,
        }
    }
    pub(crate) fn result_at<T>(&mut self, result: Result<T>, offset: usize) -> Result<T> {
        match result {
            Ok(value) => Ok(value),
            Err(error) => {
                let mut diagnostic = self.failure.take().unwrap_or(error);
                diagnostic.offset = offset;
                Err(diagnostic)
            }
        }
    }
}

impl WorkControl for PaintControl<'_> {
    fn charge(&mut self, kind: WorkKind, units: u64) -> std::result::Result<(), Stop> {
        if let Err(error) = self.ledger.work.charge(kind, units) {
            let stop = stop_for(&error);
            self.failure = Some(error);
            return Err(stop);
        }
        if let Err(stop) = self.outer.charge(kind, units) {
            self.failure = Some(from_stop(stop, 0));
            return Err(stop);
        }
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        if let Err(stop) = self.outer.checkpoint() {
            self.failure = Some(from_stop(stop, 0));
            return Err(stop);
        }
        Ok(())
    }
}

fn from_stop(stop: Stop, offset: usize) -> Diagnostic {
    let kind = match stop {
        Stop::Canceled => ErrorKind::Canceled,
        Stop::Overflow => ErrorKind::Limit(vexel_scene::LimitKind::Arithmetic),
        Stop::Limit => ErrorKind::Limit(vexel_scene::LimitKind::Work),
    };
    Diagnostic { kind, offset }
}

fn stop_for(error: &Diagnostic) -> Stop {
    match error.kind {
        ErrorKind::OutOfMemory => Stop::Limit,
        ErrorKind::Canceled => Stop::Canceled,
        ErrorKind::Limit(vexel_scene::LimitKind::Arithmetic) => Stop::Overflow,
        ErrorKind::Limit(_) => Stop::Limit,
        _ => Stop::Limit,
    }
}
