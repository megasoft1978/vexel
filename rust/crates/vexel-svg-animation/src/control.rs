use vexel_base::{Stop, WorkControl, WorkKind};

/// Charges real parser/evaluator work. Legacy entry points use the same engines
/// with `control: None`, preserving their original stop-free behavior.
pub(crate) struct Meter<'a> {
    control: Option<&'a mut dyn WorkControl>,
    stopped: Option<Stop>,
}

impl<'a> Meter<'a> {
    pub(crate) fn controlled(control: &'a mut dyn WorkControl) -> Self {
        Self {
            control: Some(control),
            stopped: None,
        }
    }
    pub(crate) fn unlimited() -> Self {
        Self {
            control: None,
            stopped: None,
        }
    }
    pub(crate) fn stop(&self) -> Option<Stop> {
        self.stopped
    }
    pub(crate) fn checkpoint(&mut self) -> bool {
        if self.stopped.is_some() {
            return false;
        }
        if let Some(control) = self.control.as_deref_mut() {
            if let Err(stop) = control.checkpoint() {
                self.stopped = Some(stop);
                return false;
            }
        }
        true
    }
    pub(crate) fn charge(&mut self, kind: WorkKind, mut units: usize) -> bool {
        while units > 0 {
            if !self.checkpoint() {
                return false;
            }
            let batch = units.min(256) as u64;
            if let Some(control) = self.control.as_deref_mut() {
                if let Err(stop) = control.charge(kind, batch) {
                    self.stopped = Some(stop);
                    return false;
                }
            }
            units -= batch as usize;
        }
        true
    }
}

#[cfg(test)]
pub(crate) struct StopAfter {
    pub(crate) kind: WorkKind,
    pub(crate) limit: u64,
    pub(crate) seen: u64,
}

#[cfg(test)]
impl StopAfter {
    pub(crate) fn new(kind: WorkKind, limit: u64) -> Self {
        Self {
            kind,
            limit,
            seen: 0,
        }
    }
}

#[cfg(test)]
impl WorkControl for StopAfter {
    fn charge(&mut self, kind: WorkKind, units: u64) -> Result<(), Stop> {
        if kind == self.kind {
            let next = self.seen.checked_add(units).ok_or(Stop::Overflow)?;
            if next >= self.limit {
                self.seen = self.limit;
                return Err(Stop::Canceled);
            }
            self.seen = next;
        }
        Ok(())
    }
    fn checkpoint(&mut self) -> Result<(), Stop> {
        Ok(())
    }
}
