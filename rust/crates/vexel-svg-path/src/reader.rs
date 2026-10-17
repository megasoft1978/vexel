//! Resumable byte cursors for the path grammar. Each advance performs one byte
//! visit or one bounded numeric/validation operation; no scanner owns a source
//! copy, a work policy, or a temporary mutable parser context.
use crate::{failure, Diagnostic, DiagnosticKind, PathNumber};
use vexel_base::{ControlledError, Span, WorkKind};

#[derive(Debug)]
pub(crate) struct Reader<'src> {
    pub bytes: &'src [u8],
    pub base: usize,
    pub index: usize,
}
impl Reader<'_> {
    // new() checked the span; index <= bytes.len(), so this sum is <= span.end.
    pub fn absolute(&self) -> usize {
        self.base + self.index
    }
    pub fn peek(&self) -> Option<u8> {
        self.bytes.get(self.index).copied()
    }
    pub fn take(&mut self) -> Result<(), ControlledError<Diagnostic>> {
        self.index = self.index.checked_add(1).ok_or(ControlledError::Overflow)?;
        Ok(())
    }
}
pub(crate) fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}
pub(crate) fn is_separator(byte: u8) -> bool {
    is_space(byte) || byte == b','
}

#[derive(Debug, Clone, Copy)]
enum CommaPhase {
    Scan,
    Left,
    Right,
    Check,
}

/// Preserve the original full comma pass and its first-comma error precedence,
/// including its left/right whitespace-neighbor checks, across tiny quanta.
#[derive(Debug)]
pub(crate) struct CommaCursor {
    index: usize,
    comma: usize,
    left: usize,
    right: usize,
    left_byte: Option<u8>,
    right_byte: Option<u8>,
    phase: CommaPhase,
}
impl CommaCursor {
    #[cfg(test)]
    pub fn stage(&self) -> &'static str {
        match self.phase {
            CommaPhase::Scan => "scan",
            CommaPhase::Left => "left",
            CommaPhase::Right => "right",
            CommaPhase::Check => "check",
        }
    }
    pub fn new() -> Self {
        Self {
            index: 0,
            comma: 0,
            left: 0,
            right: 0,
            left_byte: None,
            right_byte: None,
            phase: CommaPhase::Scan,
        }
    }
    pub fn kind(&self) -> WorkKind {
        if matches!(self.phase, CommaPhase::Check) {
            WorkKind::TokenVisit
        } else {
            WorkKind::SourceRead
        }
    }
    pub fn advance(
        &mut self,
        bytes: &[u8],
        base: usize,
    ) -> Result<bool, ControlledError<Diagnostic>> {
        match self.phase {
            CommaPhase::Scan => {
                let Some(byte) = bytes.get(self.index) else {
                    return Ok(true);
                };
                self.comma = self.index;
                self.index = self.index.checked_add(1).ok_or(ControlledError::Overflow)?;
                if *byte == b',' {
                    self.left = self.comma;
                    self.right = self.index;
                    self.phase = CommaPhase::Left;
                }
            }
            CommaPhase::Left => {
                self.left_byte = self
                    .left
                    .checked_sub(1)
                    .and_then(|index| bytes.get(index))
                    .copied();
                if self.left_byte.is_some_and(is_space) {
                    self.left -= 1;
                } else {
                    self.phase = CommaPhase::Right;
                }
            }
            CommaPhase::Right => {
                self.right_byte = bytes.get(self.right).copied();
                if self.right_byte.is_some_and(is_space) {
                    self.right = self.right.checked_add(1).ok_or(ControlledError::Overflow)?;
                } else {
                    self.phase = CommaPhase::Check;
                }
            }
            CommaPhase::Check => {
                let left_ok = self
                    .left_byte
                    .is_some_and(|byte| byte.is_ascii_digit() || byte == b'.');
                let right_ok = self.right_byte.is_some_and(|byte| {
                    byte.is_ascii_digit() || matches!(byte, b'.' | b'+' | b'-')
                });
                if !left_ok || !right_ok {
                    return Err(ControlledError::Original(failure(
                        DiagnosticKind::Path,
                        base + self.comma,
                    )));
                }
                self.phase = CommaPhase::Scan;
            }
        }
        Ok(false)
    }
}

#[derive(Debug, Clone, Copy)]
enum NumberPhase {
    Sign,
    Integer,
    Fraction,
    Exponent,
    ExponentSign,
    ExponentProbe,
    ExponentDigits,
    Finish,
}

/// The original path number reader's arithmetic, split at byte visits. In
/// particular, fractional digits are added individually with the same scale,
/// and exponent digits retain the original saturating 10,000 threshold.
#[derive(Debug)]
pub(crate) struct NumberCursor {
    start: usize,
    negative: bool,
    value: f64,
    digits: usize,
    scale: f64,
    exponent: i32,
    exponent_negative: bool,
    exponent_probe: usize,
    phase: NumberPhase,
}
impl NumberCursor {
    #[cfg(test)]
    pub fn stage(&self) -> &'static str {
        match self.phase {
            NumberPhase::Sign => "sign",
            NumberPhase::Integer => "integer",
            NumberPhase::Fraction => "fraction",
            NumberPhase::Exponent => "exponent",
            NumberPhase::ExponentSign => "exponent-sign",
            NumberPhase::ExponentProbe => "exponent-probe",
            NumberPhase::ExponentDigits => "exponent-digits",
            NumberPhase::Finish => "finish",
        }
    }
    pub fn new(start: usize) -> Self {
        Self {
            start,
            negative: false,
            value: 0.0,
            digits: 0,
            scale: 0.1,
            exponent: 0,
            exponent_negative: false,
            exponent_probe: start,
            phase: NumberPhase::Sign,
        }
    }
    pub fn kind(&self) -> WorkKind {
        if matches!(self.phase, NumberPhase::Finish) {
            WorkKind::TokenVisit
        } else {
            WorkKind::SourceRead
        }
    }
    pub fn advance(
        &mut self,
        reader: &mut Reader<'_>,
    ) -> Result<Option<PathNumber>, ControlledError<Diagnostic>> {
        match self.phase {
            NumberPhase::Sign => {
                match reader.peek() {
                    Some(b'-') => {
                        self.negative = true;
                        reader.take()?;
                    }
                    Some(b'+') => reader.take()?,
                    _ => (),
                }
                self.phase = NumberPhase::Integer;
            }
            NumberPhase::Integer => {
                if let Some(byte @ b'0'..=b'9') = reader.peek() {
                    self.value = self.value * 10.0 + f64::from(byte - b'0');
                    reader.take()?;
                    self.digits = self
                        .digits
                        .checked_add(1)
                        .ok_or(ControlledError::Overflow)?;
                    if !self.value.is_finite() || self.value > 1.0e300 {
                        return Err(self.error(reader));
                    }
                } else if reader.peek() == Some(b'.') {
                    reader.take()?;
                    self.phase = NumberPhase::Fraction;
                } else {
                    self.end_digits(reader)?;
                }
            }
            NumberPhase::Fraction => {
                if let Some(byte @ b'0'..=b'9') = reader.peek() {
                    self.value += f64::from(byte - b'0') * self.scale;
                    self.scale *= 0.1;
                    reader.take()?;
                    self.digits = self
                        .digits
                        .checked_add(1)
                        .ok_or(ControlledError::Overflow)?;
                } else {
                    self.end_digits(reader)?;
                }
            }
            NumberPhase::Exponent => {
                if matches!(reader.peek(), Some(b'e' | b'E')) {
                    self.exponent_probe = reader
                        .index
                        .checked_add(1)
                        .ok_or(ControlledError::Overflow)?;
                    self.phase = NumberPhase::ExponentSign;
                } else {
                    self.phase = NumberPhase::Finish;
                }
            }
            NumberPhase::ExponentSign => {
                match reader.bytes.get(self.exponent_probe) {
                    Some(b'-') => {
                        self.exponent_negative = true;
                        self.exponent_probe = self
                            .exponent_probe
                            .checked_add(1)
                            .ok_or(ControlledError::Overflow)?;
                    }
                    Some(b'+') => {
                        self.exponent_probe = self
                            .exponent_probe
                            .checked_add(1)
                            .ok_or(ControlledError::Overflow)?
                    }
                    _ => (),
                }
                self.phase = NumberPhase::ExponentProbe;
            }
            NumberPhase::ExponentProbe => {
                // Invalid exponent lookahead must leave the reader at 'e', as
                // in the accepted parser, rather than consuming a malformed suffix.
                if matches!(reader.bytes.get(self.exponent_probe), Some(b'0'..=b'9')) {
                    reader.index = self.exponent_probe;
                    self.phase = NumberPhase::ExponentDigits;
                } else {
                    self.phase = NumberPhase::Finish;
                }
            }
            NumberPhase::ExponentDigits => {
                if let Some(byte @ b'0'..=b'9') = reader.peek() {
                    if self.exponent < 10_000 {
                        self.exponent = self
                            .exponent
                            .saturating_mul(10)
                            .saturating_add(i32::from(byte - b'0'));
                    }
                    reader.take()?;
                } else {
                    if self.exponent_negative {
                        self.exponent = -self.exponent;
                    }
                    self.value *= 10.0_f64.powi(self.exponent);
                    self.phase = NumberPhase::Finish;
                }
            }
            NumberPhase::Finish => {
                if self.negative {
                    self.value = -self.value;
                }
                if !self.value.is_finite() || self.value.abs() > 1.0e6 {
                    return Err(self.error(reader));
                }
                return Ok(Some(PathNumber {
                    source: Span {
                        start: reader.base + self.start,
                        end: reader.absolute(),
                    },
                    value: self.value,
                }));
            }
        }
        Ok(None)
    }
    fn end_digits(&mut self, reader: &mut Reader<'_>) -> Result<(), ControlledError<Diagnostic>> {
        if self.digits == 0 {
            reader.index = self.start;
            return Err(self.error(reader));
        }
        self.phase = NumberPhase::Exponent;
        Ok(())
    }
    fn error(&self, reader: &Reader<'_>) -> ControlledError<Diagnostic> {
        ControlledError::Original(failure(DiagnosticKind::Path, reader.absolute()))
    }
}
