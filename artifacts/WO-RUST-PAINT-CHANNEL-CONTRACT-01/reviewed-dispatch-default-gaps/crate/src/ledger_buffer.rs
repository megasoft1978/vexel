//! Explicit owning clients of the accepted ledger allocator. No Vec/lease escape.
//! Drop without release conservatively retains the charge until ledger teardown.
//! Clients must release before discarding; replacements count as a second active
//! record, so a query ledger configured for 65536 enforces overlap at that limit.
use crate::{
    types::{invalid, stopped},
    *,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedgerIdentity(u64);
#[derive(Debug)]
pub struct LedgerBuffer<T: Copy> {
    values: Option<Vec<T>>,
    lease: AllocationLease,
}
impl ChargeLedger {
    pub fn owner_identity(&self) -> LedgerIdentity {
        LedgerIdentity(self.identity())
    }
    pub fn require_owner(&self, identity: LedgerIdentity, source: &SourceOwner) -> Result<()> {
        if identity != self.owner_identity() || !std::sync::Arc::ptr_eq(source, self.source_owner())
        {
            return Err(invalid(0));
        }
        Ok(())
    }
    /// Requested reservation precedes physical reserve; actual capacity is
    /// admitted before return. Successful physical history survives cancellation.
    pub fn allocate_buffer<T: Copy>(
        &mut self,
        tag: AllocationTag,
        capacity: usize,
        control: &mut dyn WorkControl,
    ) -> Result<LedgerBuffer<T>> {
        let (values, lease) = self.allocate(tag, capacity, control)?;
        Ok(LedgerBuffer {
            values: Some(values),
            lease,
        })
    }
    pub(crate) fn require_lease(&self, lease: AllocationLease) -> Result<()> {
        // Defined alongside private registry access in accounting.rs.
        self.check_lease(lease)
    }
}
impl<T: Copy> LedgerBuffer<T> {
    fn require(&self, ledger: &ChargeLedger) -> Result<()> {
        if self.values.is_none() {
            return Err(invalid(0));
        }
        ledger.require_lease(self.lease)
    }
    pub fn values<'a>(&'a self, ledger: &ChargeLedger) -> Result<&'a [T]> {
        self.require(ledger)?;
        Ok(self.values.as_ref().ok_or_else(|| invalid(0))?)
    }
    pub fn values_mut<'a>(&'a mut self, ledger: &ChargeLedger) -> Result<&'a mut [T]> {
        self.require(ledger)?;
        Ok(self.values.as_mut().ok_or_else(|| invalid(0))?)
    }
    pub fn capacity(&self, ledger: &ChargeLedger) -> Result<usize> {
        self.require(ledger)?;
        Ok(self.values.as_ref().ok_or_else(|| invalid(0))?.capacity())
    }
    /// No allocation. Poll every real iteration; unpublished suffix is removed
    /// on cancellation, preserving the old values and admitted capacity.
    pub fn append(
        &mut self,
        ledger: &ChargeLedger,
        suffix: &[T],
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        self.require(ledger)?;
        control.checkpoint().map_err(stopped)?;
        let values = self.values.as_mut().ok_or_else(|| invalid(0))?;
        let len = values.len();
        let end = len
            .checked_add(suffix.len())
            .ok_or_else(crate::memory::arithmetic)?;
        if end > values.capacity() {
            return Err(invalid(0));
        }
        let result = (|| {
            for value in suffix {
                control.charge(WorkKind::SourceRead, 1).map_err(stopped)?;
                values.push(*value);
            }
            control.checkpoint().map_err(stopped)
        })();
        if result.is_err() {
            values.truncate(len);
        }
        result
    }
    /// Reserve old+new overlap, then controlled copy, then drop old storage before
    /// releasing its charge. Errors leave old data/token/capacity unchanged.
    pub fn replace_capacity(
        &mut self,
        ledger: &mut ChargeLedger,
        tag: AllocationTag,
        capacity: usize,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        self.require(ledger)?;
        if capacity < self.values(ledger)?.len() {
            return Err(invalid(0));
        }
        let mut replacement = ledger.allocate_buffer(tag, capacity, control)?;
        if let Err(error) = replacement.append(ledger, self.values(ledger)?, control) {
            replacement.release(ledger)?;
            return Err(error);
        }
        self.release(ledger)?;
        *self = replacement;
        Ok(())
    }
    pub fn truncate(&mut self, ledger: &ChargeLedger, len: usize) -> Result<()> {
        self.require(ledger)?;
        self.values
            .as_mut()
            .ok_or_else(|| invalid(0))?
            .truncate(len);
        Ok(())
    }
    /// Unconditional after cancellation, checked stale/double/foreign access.
    pub fn release(&mut self, ledger: &mut ChargeLedger) -> Result<()> {
        self.require(ledger)?;
        drop(self.values.take());
        ledger.release(self.lease)
    }
}
