//! Original decimal and length cursor, with explicit source offsets and cancellation.
use crate::source::charge;
use vexel_scene::{Diagnostic, ErrorKind, LimitKind, Result, WorkControl, WorkKind};
use vexel_xml::XmlSpan;
pub fn value_limits() -> vexel_svg_values::ValueLimits {
    vexel_svg_values::ValueLimits {
        source_bytes: 1 << 30,
        attributes: 4096,
    }
}
fn ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}
#[derive(Debug)]
pub struct NumberReader<'src> {
    pub source: &'src [u8],
    pub index: usize,
    pub end: usize,
}
impl<'src> NumberReader<'src> {
    pub fn new(source: &'src [u8], span: XmlSpan) -> Result<Self> {
        let end = span.end().ok_or(Diagnostic {
            kind: ErrorKind::InvalidArgument,
            offset: span.offset,
        })?;
        if span.slice(source).is_none() || source.len() > 1 << 30 {
            return Err(Diagnostic {
                kind: ErrorKind::InvalidArgument,
                offset: span.offset,
            });
        }
        Ok(Self {
            source,
            index: span.offset,
            end,
        })
    }
    pub fn peek(&self) -> Option<u8> {
        if self.index < self.end {
            self.source.get(self.index).copied()
        } else {
            None
        }
    }
    fn take(&mut self, control: &mut dyn WorkControl) -> Result<()> {
        charge(control, WorkKind::SourceRead, 1, self.index)?;
        self.index = self.index.checked_add(1).ok_or(Diagnostic {
            kind: ErrorKind::Limit(LimitKind::Arithmetic),
            offset: self.index,
        })?;
        Ok(())
    }
    pub fn skip_space(&mut self, control: &mut dyn WorkControl) -> Result<()> {
        while self.peek().is_some_and(ws) {
            self.take(control)?
        }
        Ok(())
    }
    pub fn skip_path_separators(&mut self, control: &mut dyn WorkControl) -> Result<()> {
        while self.peek().is_some_and(|b| ws(b) || b == b',') {
            self.take(control)?
        }
        Ok(())
    }
    pub fn path_operand_boundary(
        &mut self,
        previous: bool,
        arc_flag: bool,
        command: u8,
        argument: usize,
        control: &mut dyn WorkControl,
    ) -> Result<bool> {
        let before = self.index;
        self.skip_path_separators(control)?;
        Ok(!previous
            || self.index != before
            || matches!(self.peek(), Some(b'+' | b'-' | b'.'))
            || command == b'A' && (argument == 4 || argument == 5 && arc_flag))
    }
    pub fn separator(&mut self, control: &mut dyn WorkControl) -> Result<bool> {
        let before = self.index;
        self.skip_space(control)?;
        let spaced = self.index != before;
        if self.peek() == Some(b',') {
            self.take(control)?;
            self.skip_space(control)?;
            return Ok(self.peek().is_some_and(|b| b != b','));
        }
        Ok(spaced && self.index < self.end)
    }
    pub fn finished(&mut self, control: &mut dyn WorkControl) -> Result<bool> {
        self.skip_space(control)?;
        Ok(self.index == self.end)
    }
    pub fn number(&mut self, control: &mut dyn WorkControl) -> Result<f64> {
        self.skip_space(control)?;
        let start = self.index;
        let negative = self.peek() == Some(b'-');
        if matches!(self.peek(), Some(b'+' | b'-')) {
            self.take(control)?
        }
        let mut value = 0.0;
        let mut digits = 0usize;
        while let Some(b @ b'0'..=b'9') = self.peek() {
            value = value * 10.0 + f64::from(b - b'0');
            self.take(control)?;
            digits += 1;
            if !value.is_finite() || value > 1e300 {
                return Err(self.number_error(start));
            }
        }
        if self.peek() == Some(b'.') {
            self.take(control)?;
            let mut scale = 0.1;
            while let Some(b @ b'0'..=b'9') = self.peek() {
                value += f64::from(b - b'0') * scale;
                scale *= 0.1;
                self.take(control)?;
                digits += 1;
            }
        }
        if digits == 0 {
            self.index = start;
            return Err(self.number_error(start));
        }
        let mut exponent = 0i32;
        let mut exponent_negative = false;
        if matches!(self.peek(), Some(b'e' | b'E')) {
            let mut look = self.index + 1;
            if look < self.end && matches!(self.source[look], b'+' | b'-') {
                look += 1
            }
            if look < self.end && self.source[look].is_ascii_digit() {
                self.take(control)?;
                if matches!(self.peek(), Some(b'+' | b'-')) {
                    exponent_negative = self.peek() == Some(b'-');
                    self.take(control)?;
                }
                while let Some(b @ b'0'..=b'9') = self.peek() {
                    if exponent < 10000 {
                        exponent = exponent * 10 + i32::from(b - b'0')
                    }
                    self.take(control)?;
                }
            }
        }
        if exponent_negative {
            exponent = -exponent
        }
        value *= 10f64.powf(f64::from(exponent));
        if negative {
            value = -value
        }
        if !value.is_finite() || value.abs() > 1e6 {
            return Err(self.number_error(start));
        }
        Ok(value)
    }
    pub fn arc_flag(&mut self, control: &mut dyn WorkControl) -> Result<bool> {
        self.skip_space(control)?;
        let at = self.index;
        match self.peek() {
            Some(b'0') => {
                self.take(control)?;
                Ok(false)
            }
            Some(b'1') => {
                self.take(control)?;
                Ok(true)
            }
            _ => Err(Diagnostic {
                kind: ErrorKind::Path,
                offset: at,
            }),
        }
    }
    pub fn length(&mut self, reference: f64, control: &mut dyn WorkControl) -> Result<f64> {
        let mut value = self.number(control)?;
        let suffix = self.index;
        while self
            .peek()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'%')
        {
            self.take(control)?
        }
        let unit = &self.source[suffix..self.index];
        match unit {
            b"" => return Ok(value),
            b"%" => {
                if !reference.is_finite() || reference <= 0.0 {
                    return Err(self.number_error(suffix));
                }
                value *= reference / 100.0
            }
            _ if unit.eq_ignore_ascii_case(b"px") => {}
            _ if unit.eq_ignore_ascii_case(b"pt") => value *= 96.0 / 72.0,
            _ if unit.eq_ignore_ascii_case(b"mm") => value *= 96.0 / 25.4,
            _ if unit.eq_ignore_ascii_case(b"cm") => value *= 96.0 / 2.54,
            _ if unit.eq_ignore_ascii_case(b"in") => value *= 96.0,
            _ if unit.eq_ignore_ascii_case(b"em") => value *= 16.0,
            _ => return Err(self.number_error(suffix)),
        }
        if !value.is_finite() {
            return Err(self.number_error(suffix));
        }
        Ok(value)
    }
    fn number_error(&self, offset: usize) -> Diagnostic {
        Diagnostic {
            kind: ErrorKind::Number,
            offset,
        }
    }
}
pub fn skip_number_space(source: &[u8], mut at: usize) -> usize {
    while source.get(at).is_some_and(|b| ws(*b)) {
        at += 1
    }
    at
}
pub fn next_number_separator(source: &[u8], at: usize) -> bool {
    source.get(at).is_some_and(|b| ws(*b) || *b == b',')
}
pub fn read_arc_flag(source: &[u8], at: usize) -> Result<(bool, usize)> {
    let mut r = NumberReader::new(
        source,
        XmlSpan {
            offset: at,
            length: source.len().checked_sub(at).ok_or(Diagnostic {
                kind: ErrorKind::InvalidArgument,
                offset: at,
            })?,
        },
    )?;
    let flag = r.arc_flag(&mut NoStop)?;
    Ok((flag, r.index))
}
pub fn read_number(source: &[u8], span: XmlSpan) -> Result<f64> {
    read_number_controlled(source, span, &mut NoStop)
}
pub fn read_number_controlled(
    source: &[u8],
    span: XmlSpan,
    control: &mut dyn WorkControl,
) -> Result<f64> {
    let mut r = NumberReader::new(source, span)?;
    let n = r.number(control)?;
    if !r.finished(control)? {
        return Err(Diagnostic {
            kind: ErrorKind::Number,
            offset: r.index,
        });
    }
    Ok(n)
}
pub fn read_length(source: &[u8], span: XmlSpan, reference: f64) -> Result<f64> {
    read_length_controlled(source, span, reference, &mut NoStop)
}
pub fn read_length_controlled(
    source: &[u8],
    span: XmlSpan,
    reference: f64,
    control: &mut dyn WorkControl,
) -> Result<f64> {
    let mut r = NumberReader::new(source, span)?;
    let n = r.length(reference, control)?;
    if !r.finished(control)? {
        return Err(Diagnostic {
            kind: ErrorKind::Number,
            offset: r.index,
        });
    }
    Ok(n)
}
pub fn read_dimension(source: &[u8], span: XmlSpan) -> Result<f64> {
    read_length(source, span, 0.0)
}
pub fn read_number_list(source: &[u8], span: XmlSpan, count: usize) -> Result<Vec<f64>> {
    let mut r = NumberReader::new(source, span)?;
    if count == 0 || count > span.length {
        return Err(Diagnostic {
            kind: ErrorKind::InvalidArgument,
            offset: span.offset,
        });
    }
    let mut v = Vec::new();
    v.try_reserve_exact(count).map_err(|_| Diagnostic {
        kind: ErrorKind::OutOfMemory,
        offset: span.offset,
    })?;
    for i in 0..count {
        if i != 0 && !r.separator(&mut NoStop)? {
            return Err(Diagnostic {
                kind: ErrorKind::Number,
                offset: r.index,
            });
        }
        v.push(r.number(&mut NoStop)?);
    }
    if !r.finished(&mut NoStop)? {
        return Err(Diagnostic {
            kind: ErrorKind::Number,
            offset: r.index,
        });
    }
    Ok(v)
}
struct NoStop;
impl WorkControl for NoStop {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), vexel_base::Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), vexel_base::Stop> {
        Ok(())
    }
}
