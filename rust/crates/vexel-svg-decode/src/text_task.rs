//! Bounded whitespace-only text task continuation.
use crate::{decode::Decoder, tasks::Task};
use vexel_base::{WorkControl, WorkKind};
use vexel_scene::{Diagnostic, ErrorKind, Result};
use vexel_xml::XmlSpan;

impl Decoder<'_, '_> {
    pub(crate) fn process_text_task(
        &mut self,
        span: XmlSpan,
        mut cursor: usize,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let raw = span
            .slice(self.shared.source())
            .ok_or_else(|| crate::invalid(span.offset))?;
        let end = cursor.saturating_add(256).min(raw.len());
        while cursor < end {
            crate::source::charge(control, WorkKind::SourceRead, 1, span.offset + cursor)?;
            if !matches!(raw[cursor], b' ' | b'\t' | b'\r' | b'\n') {
                return Err(Diagnostic {
                    kind: ErrorKind::Unsupported,
                    offset: span.offset + cursor,
                });
            }
            cursor += 1;
        }
        if cursor < raw.len() {
            self.push(Task::Text { span, cursor }, control)?;
        }
        Ok(())
    }
}
