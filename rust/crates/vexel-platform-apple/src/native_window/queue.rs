use super::types::{EventBatch, PointerKind, QueueFault, QueueFaultReason, WindowEvent};
use std::collections::VecDeque;

const CAP: usize = 256;
const ORDINARY: usize = 240;
const PAYLOAD_CAP: usize = 1024 * 1024;
#[derive(Default)]
pub(crate) struct EventQueue {
    entries: VecDeque<(u64, WindowEvent, usize)>,
    sequence: u64,
    payload: usize,
    external_payload: usize,
    dropped: u64,
    disabled: bool,
    retired: bool,
    close_requested: bool,
    fault: Option<(u64, QueueFault)>,
    reset: Option<u64>,
    close: Option<u64>,
    delivered_reset: Option<u64>,
}
impl EventQueue {
    fn next(&mut self) -> u64 {
        // Exhaustion cannot wrap or reopen ingress. Sticky metadata remains bounded.
        self.sequence = self.sequence.checked_add(1).unwrap_or_else(|| {
            self.disabled = true;
            u64::MAX
        });
        self.sequence
    }
    fn sequence_exhausted(&mut self) -> bool {
        if self.retired {
            return true;
        }
        if self.sequence < u64::MAX - 8 {
            return false;
        }
        // Reserve strictly ordered terminal metadata; never wrap or reopen this queue.
        self.retired = true;
        self.disabled = true;
        self.fault = Some((
            u64::MAX - 3,
            QueueFault {
                reason: QueueFaultReason::InputOverflow,
                coalesced_or_dropped: self.dropped,
            },
        ));
        self.reset = Some(u64::MAX - 2);
        if self.close.is_some() {
            self.close = Some(u64::MAX - 1);
        }
        true
    }
    pub(crate) fn fault(&mut self, reason: QueueFaultReason) {
        if self.sequence_exhausted() {
            return;
        }
        if let Some((_, f)) = &mut self.fault {
            f.coalesced_or_dropped = self.dropped;
        } else {
            let s = self.next();
            self.fault = Some((
                s,
                QueueFault {
                    reason,
                    coalesced_or_dropped: self.dropped,
                },
            ));
        }
    }
    pub(crate) fn fail(&mut self, reason: QueueFaultReason) {
        if self.sequence_exhausted() {
            return;
        }
        if self.reset.is_some() || self.delivered_reset.is_some() {
            self.fault(reason);
            return;
        }
        self.disabled = true;
        let s = self.next();
        self.fault = Some((
            s,
            QueueFault {
                reason,
                coalesced_or_dropped: self.dropped,
            },
        ));
        self.reset = Some(self.next());
        if self.close.is_some() {
            self.close = Some(self.next());
        }
    }
    pub(crate) fn push(&mut self, event: WindowEvent, control: bool) -> bool {
        if matches!(event, WindowEvent::CloseRequested) {
            self.request_close();
            return true;
        }
        if self.sequence_exhausted() {
            return false;
        }
        if self.disabled && !control {
            return false;
        } // covered by the sequenced reset
        let bytes = payload(&event);
        if bytes > 64 * 1024 {
            self.fail(QueueFaultReason::InputOverflow);
            return false;
        }
        let replace = self
            .entries
            .back()
            .is_some_and(|(_, tail, _)| coalesces(tail, &event));
        if replace {
            let old = self.entries.back().unwrap().2;
            if self.payload - old + bytes + self.external_payload > PAYLOAD_CAP {
                self.fail(QueueFaultReason::InputOverflow);
                return false;
            }
            let seq = self.next();
            self.payload = self.payload - old + bytes;
            *self.entries.back_mut().unwrap() = (seq, event, bytes);
            return true;
        }
        let controls = self
            .entries
            .iter()
            .filter(|(_, e, _)| is_control(e))
            .count();
        let ordinary = self.entries.len() - controls;
        if !control && ordinary >= ORDINARY && lossy(&event) {
            self.dropped = self.dropped.saturating_add(1);
            self.fault(QueueFaultReason::InputOverflow);
            return false;
        }
        // Non-droppable transitions may consume the sixteen reserved records.
        if self.entries.len() == CAP
            || (control && controls >= CAP - ORDINARY)
            || self
                .payload
                .checked_add(bytes)
                .and_then(|n| n.checked_add(self.external_payload))
                .is_none_or(|n| n > PAYLOAD_CAP)
        {
            self.fail(QueueFaultReason::InputOverflow);
            return false;
        }
        if self.entries.try_reserve(1).is_err() {
            self.fail(QueueFaultReason::AllocationFailure);
            return false;
        }
        let seq = self.next();
        self.payload += bytes;
        self.entries.push_back((seq, event, bytes));
        true
    }
    pub(crate) fn request_close(&mut self) {
        if !self.close_requested {
            self.close_requested = true;
            self.close = Some(if self.sequence_exhausted() {
                u64::MAX - 1
            } else {
                self.next()
            });
        }
    }
    pub(crate) fn acknowledge_reset(&mut self, sequence: u64) -> bool {
        if !self.retired && self.delivered_reset == Some(sequence) {
            self.delivered_reset = None;
            self.disabled = false;
            true
        } else {
            false
        }
    }
    pub(crate) fn set_external_payload(&mut self, bytes: usize) -> bool {
        if self
            .payload
            .checked_add(bytes)
            .is_none_or(|n| n > PAYLOAD_CAP)
        {
            return false;
        }
        self.external_payload = bytes;
        true
    }
    pub(crate) fn disabled(&self) -> bool {
        self.disabled
    }
    pub(crate) fn has_pending(&self) -> bool {
        !self.entries.is_empty()
            || self.fault.is_some()
            || self.reset.is_some()
            || self.close.is_some()
    }
    pub(crate) fn drain(&mut self, max: usize, native_pending: bool) -> EventBatch {
        let mut events = Vec::new();
        if events.try_reserve(max.min(CAP + 3)).is_err() {
            self.fail(QueueFaultReason::AllocationFailure);
            return EventBatch {
                events,
                pending: true,
            };
        }
        while events.len() < max {
            let entry = self.entries.front().map(|v| v.0);
            let sticky = [self.fault.as_ref().map(|v| v.0), self.reset, self.close]
                .into_iter()
                .flatten()
                .min();
            if entry.is_some() && sticky.is_none_or(|s| entry.unwrap() < s) {
                let (_, event, bytes) = self.entries.pop_front().unwrap();
                self.payload -= bytes;
                events.push(event);
                continue;
            }
            let Some(seq) = sticky else {
                break;
            };
            if self.fault.as_ref().is_some_and(|v| v.0 == seq) {
                events.push(WindowEvent::QueueFault(self.fault.take().unwrap().1));
            } else if self.reset == Some(seq) {
                self.reset = None;
                self.delivered_reset = Some(seq);
                events.push(WindowEvent::InputReset {
                    sequence: seq,
                    all_keys_up: true,
                    all_buttons_up: true,
                });
            } else {
                self.close = None;
                if !self.retired {
                    self.close_requested = false;
                }
                events.push(WindowEvent::CloseRequested);
            }
        }
        EventBatch {
            events,
            pending: self.has_pending() || native_pending,
        }
    }
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.payload = 0;
        self.external_payload = 0;
        self.fault = None;
        self.reset = None;
        self.close = None;
        self.disabled = true;
    }
    pub(crate) const fn native_budget() -> usize {
        64
    }
}
fn payload(e: &WindowEvent) -> usize {
    match e {
        WindowEvent::TextCommitted(s) | WindowEvent::TextCommand(s) => s.len(),
        WindowEvent::TextCommittedWithReplacement { text, .. } => text.len(),
        WindowEvent::Composition(c) => {
            c.text.len()
                + c.attributes.len() * std::mem::size_of::<super::types::TextAttributeSpan>()
        }
        _ => 0,
    }
}
fn lossy(e: &WindowEvent) -> bool {
    matches!(e,WindowEvent::Pointer(p) if p.kind==PointerKind::Move)
        || matches!(
            e,
            WindowEvent::Resize { .. } | WindowEvent::BackingScaleChanged { .. }
        )
}
fn coalesces(a: &WindowEvent, b: &WindowEvent) -> bool {
    matches!((a,b),(WindowEvent::Pointer(a),WindowEvent::Pointer(b)) if a.kind==PointerKind::Move && b.kind==PointerKind::Move)
        || matches!(
            (a, b),
            (WindowEvent::Resize { .. }, WindowEvent::Resize { .. })
                | (
                    WindowEvent::BackingScaleChanged { .. },
                    WindowEvent::BackingScaleChanged { .. }
                )
        )
}
fn is_control(e: &WindowEvent) -> bool {
    matches!(
        e,
        WindowEvent::Open
            | WindowEvent::Focus(_)
            | WindowEvent::AppearanceChanged { .. }
            | WindowEvent::CloseRequested
            | WindowEvent::QueueFault(_)
            | WindowEvent::InputReset { .. }
    )
}

#[cfg(test)]
mod sequence_tests {
    use super::*;
    #[test]
    fn terminal_sequence_metadata_cannot_wrap_or_reopen_ingress() {
        let mut q = EventQueue::default();
        q.sequence = u64::MAX - 8;
        assert!(!q.push(WindowEvent::Focus(true), true));
        q.request_close();
        let events = q.drain(4, false).events;
        assert!(matches!(events[0], WindowEvent::QueueFault(_)));
        assert!(matches!(events[1],WindowEvent::InputReset{sequence,..} if sequence==u64::MAX-2));
        assert!(matches!(events[2], WindowEvent::CloseRequested));
        assert!(!q.acknowledge_reset(u64::MAX - 2));
        assert!(!q.push(WindowEvent::Focus(true), true));
    }
}

#[cfg(test)]
mod close_tests {
    use super::*;
    #[test]
    fn delivered_close_can_be_requested_again_after_application_cancel() {
        let mut q = EventQueue::default();
        q.request_close();
        q.request_close();
        assert_eq!(q.drain(4, false).events, vec![WindowEvent::CloseRequested]);
        q.request_close();
        assert_eq!(q.drain(4, false).events, vec![WindowEvent::CloseRequested]);
    }
}
