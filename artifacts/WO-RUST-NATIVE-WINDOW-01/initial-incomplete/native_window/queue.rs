use std::collections::VecDeque;
use super::types::{EventBatch, QueueFault, QueueFaultReason, WindowEvent};

const CAP: usize = 256; const ORDINARY: usize = 240; const NATIVE_BUDGET: usize = 64;
#[derive(Default)]
pub(crate) struct EventQueue {
    entries: VecDeque<(u64, WindowEvent)>, next_sequence: u64, dropped: u64, disabled: bool,
    fault: Option<(u64, QueueFault)>, reset: Option<(u64, u64)>, close: Option<(u64, WindowEvent)>, delivered_reset: Option<u64>,
}
impl EventQueue {
    pub(crate) fn push(&mut self, event: WindowEvent, control: bool) -> bool {
        if self.disabled && !matches!(event, WindowEvent::CloseRequested | WindowEvent::InputReset { .. } | WindowEvent::QueueFault(_)) { return false; }
        let replace = self.entries.back().is_some_and(|(_, tail)|
            matches!((tail, &event), (WindowEvent::Pointer(a), WindowEvent::Pointer(b)) if a.button.is_none() && b.button.is_none())
                || matches!((tail, &event), (WindowEvent::Resize { .. }, WindowEvent::Resize { .. }) | (WindowEvent::BackingScaleChanged { .. }, WindowEvent::BackingScaleChanged { .. })));
        if replace { let seq = self.next(); if let Some(back) = self.entries.back_mut() { back.0 = seq; back.1 = event; } return true; }
        let ordinary_count = self.entries.iter().filter(|(_, e)| !is_control(e)).count();
        if !control && ordinary_count >= ORDINARY {
            if matches!(event, WindowEvent::Pointer(_) | WindowEvent::Resize { .. } | WindowEvent::BackingScaleChanged { .. }) { self.dropped = self.dropped.saturating_add(1); self.latch_fault(QueueFaultReason::InputOverflow); }
            else { self.overflow_control(); }
            return false;
        }
        if self.entries.len() >= CAP { self.overflow_control(); return false; }
        let seq = self.next(); self.entries.push_back((seq, event)); true
    }
    fn next(&mut self) -> u64 { let s = self.next_sequence; self.next_sequence = self.next_sequence.saturating_add(1); s }
    fn latch_fault(&mut self, reason: QueueFaultReason) { if self.fault.is_none() { let s = self.next(); self.fault = Some((s, QueueFault { reason, coalesced_or_dropped: self.dropped })); } }
    fn overflow_control(&mut self) { if self.disabled { return; } self.disabled = true; self.latch_fault(QueueFaultReason::InputOverflow); let s = self.next(); self.reset = Some((s, s)); }
    pub(crate) fn request_close(&mut self) { if self.close.is_none() { let s=self.next(); self.close=Some((s,WindowEvent::CloseRequested)); } }
    pub(crate) fn acknowledge_reset(&mut self, seq:u64) -> bool { if self.delivered_reset == Some(seq) { self.disabled=false; self.delivered_reset=None; true } else { false } }
    pub(crate) fn has_pending(&self) -> bool { !self.entries.is_empty() || self.fault.is_some() || self.reset.is_some() || self.close.is_some() }
    pub(crate) fn drain(&mut self, max:usize, native_pending:bool) -> EventBatch {
        let mut out=Vec::with_capacity(max.min(CAP));
        while out.len()<max {
            let next_entry=self.entries.front().map(|v|v.0);
            let next_sticky=[self.fault.as_ref().map(|v|v.0),self.reset.as_ref().map(|v|v.0),self.close.as_ref().map(|v|v.0)].into_iter().flatten().min();
            if next_sticky.is_none_or(|s| next_entry.is_some_and(|e|e<s)) { if let Some((_,e))=self.entries.pop_front(){out.push(e);continue;} }
            let Some(s)=next_sticky else { break; };
            if self.fault.as_ref().is_some_and(|v|v.0==s) { let (_,f)=self.fault.take().unwrap(); out.push(WindowEvent::QueueFault(f)); }
            else if self.reset.as_ref().is_some_and(|v|v.0==s) { let (_,seq)=self.reset.take().unwrap(); self.delivered_reset=Some(seq); out.push(WindowEvent::InputReset { sequence:seq,all_keys_up:true,all_buttons_up:true }); }
            else if let Some((_,e))=self.close.take(){out.push(e);}
        }
        EventBatch { pending:self.has_pending()||native_pending,events:out }
    }
    pub(crate) const fn native_budget() -> usize { NATIVE_BUDGET }
}
fn is_control(e:&WindowEvent)->bool { matches!(e,WindowEvent::Open|WindowEvent::Focus(_)|WindowEvent::AppearanceChanged{..}|WindowEvent::Resize{..}|WindowEvent::BackingScaleChanged{..}|WindowEvent::CloseRequested|WindowEvent::InputReset{..}|WindowEvent::QueueFault(_)) }
