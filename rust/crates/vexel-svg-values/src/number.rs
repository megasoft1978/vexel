use vexel_base::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumberError {
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueError {
    InvalidSpan,
    SourceLimit,
    OutOfMemory,
    Number(NumberError),
    UnsupportedUnit { offset: usize },
}

pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    base: usize,
    pub index: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8], base: usize) -> Self {
        Self {
            bytes,
            base,
            index: 0,
        }
    }
    pub(crate) fn absolute(&self) -> usize {
        self.base.saturating_add(self.index)
    }
    pub(crate) fn skip_space(&mut self) {
        while self.index < self.bytes.len() && is_space(self.bytes[self.index]) {
            self.index += 1;
        }
    }
    pub(crate) fn number(&mut self) -> Result<f64, NumberError> {
        self.skip_space();
        let start = self.index;
        let negative = match self.bytes.get(self.index) {
            Some(b'-') => {
                self.index += 1;
                true
            }
            Some(b'+') => {
                self.index += 1;
                false
            }
            _ => false,
        };
        let mut value = 0.0;
        let mut digits = 0usize;
        while let Some(byte @ b'0'..=b'9') = self.bytes.get(self.index).copied() {
            value = value * 10.0 + f64::from(byte - b'0');
            self.index += 1;
            digits += 1;
            if !value.is_finite() || value > 1.0e300 {
                return Err(NumberError {
                    offset: self.absolute(),
                });
            }
        }
        if self.bytes.get(self.index) == Some(&b'.') {
            self.index += 1;
            let mut scale = 0.1;
            while let Some(byte @ b'0'..=b'9') = self.bytes.get(self.index).copied() {
                value += f64::from(byte - b'0') * scale;
                scale *= 0.1;
                self.index += 1;
                digits += 1;
            }
        }
        if digits == 0 {
            self.index = start;
            return Err(NumberError {
                offset: self.absolute(),
            });
        }
        if matches!(self.bytes.get(self.index), Some(b'e' | b'E')) {
            let mut lookahead = self.index + 1;
            if matches!(self.bytes.get(lookahead), Some(b'+' | b'-')) {
                lookahead += 1;
            }
            if matches!(self.bytes.get(lookahead), Some(b'0'..=b'9')) {
                self.index += 1;
                let exponent_negative = match self.bytes.get(self.index) {
                    Some(b'-') => {
                        self.index += 1;
                        true
                    }
                    Some(b'+') => {
                        self.index += 1;
                        false
                    }
                    _ => false,
                };
                let mut exponent = 0i32;
                while let Some(byte @ b'0'..=b'9') = self.bytes.get(self.index).copied() {
                    if exponent < 10_000 {
                        exponent = exponent
                            .saturating_mul(10)
                            .saturating_add(i32::from(byte - b'0'));
                    }
                    self.index += 1;
                }
                if exponent_negative {
                    exponent = -exponent;
                }
                value *= 10.0_f64.powi(exponent);
            }
        }
        if negative {
            value = -value;
        }
        if !value.is_finite() || value.abs() > 1.0e6 {
            return Err(NumberError {
                offset: self.absolute(),
            });
        }
        Ok(value)
    }

    pub(crate) fn separator(&mut self) -> bool {
        let before = self.index;
        self.skip_space();
        let had_space = self.index != before;
        if self.bytes.get(self.index) == Some(&b',') {
            self.index += 1;
            self.skip_space();
            return self.index < self.bytes.len() && self.bytes[self.index] != b',';
        }
        had_space && self.index < self.bytes.len()
    }

    pub(crate) fn finish(&mut self) -> Result<(), NumberError> {
        self.skip_space();
        if self.index == self.bytes.len() {
            Ok(())
        } else {
            Err(NumberError {
                offset: self.absolute(),
            })
        }
    }
}

pub(crate) fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

fn validate(source: &[u8], span: Span, limits: crate::ValueLimits) -> Result<&[u8], ValueError> {
    let limits = limits.effective();
    if source.len() > limits.source_bytes {
        return Err(ValueError::SourceLimit);
    }
    span.slice(source).ok_or(ValueError::InvalidSpan)
}

/// Allocation-free cursor over one checked span in the caller's original source.
/// Failed reads retain the reader position reached by the shared parser; callers
/// can inspect `absolute()` or recover at their own token boundary.
pub struct NumberCursor<'src> {
    reader: Reader<'src>,
}

impl<'src> NumberCursor<'src> {
    pub fn new(
        source: &'src [u8],
        span: Span,
        limits: crate::ValueLimits,
    ) -> Result<Self, ValueError> {
        let bytes = validate(source, span, limits)?;
        Ok(Self {
            reader: Reader::new(bytes, span.start),
        })
    }
    pub fn number(&mut self) -> Result<f64, ValueError> {
        self.reader.number().map_err(ValueError::Number)
    }
    pub fn length(&mut self, reference: f64) -> Result<f64, ValueError> {
        parse_length_reader(&mut self.reader, reference, false)
    }
    pub fn separator(&mut self) -> bool {
        self.reader.separator()
    }
    /// Consume accepted XML whitespace and report whether the checked span ended.
    pub fn finished(&mut self) -> bool {
        self.reader.skip_space();
        self.reader.index == self.reader.bytes.len()
    }
    pub fn absolute(&self) -> usize {
        self.reader.absolute()
    }
}

pub fn parse_number(source: &[u8], span: Span) -> Result<f64, ValueError> {
    parse_number_with_limits(source, span, crate::ValueLimits::default())
}
pub fn parse_number_with_limits(
    source: &[u8],
    span: Span,
    limits: crate::ValueLimits,
) -> Result<f64, ValueError> {
    let bytes = validate(source, span, limits)?;
    let mut reader = Reader::new(bytes, span.start);
    let value = reader.number().map_err(ValueError::Number)?;
    reader.finish().map_err(ValueError::Number)?;
    Ok(value)
}

pub fn parse_number_list(source: &[u8], span: Span, count: usize) -> Result<Vec<f64>, ValueError> {
    parse_number_list_with_limits(source, span, count, crate::ValueLimits::default())
}
pub fn parse_number_list_with_limits(
    source: &[u8],
    span: Span,
    count: usize,
    limits: crate::ValueLimits,
) -> Result<Vec<f64>, ValueError> {
    let bytes = validate(source, span, limits)?;
    if count == 0 || count > 64 {
        return Err(ValueError::Number(NumberError { offset: span.start }));
    }
    let mut reader = Reader::new(bytes, span.start);
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| ValueError::OutOfMemory)?;
    for index in 0..count {
        if index != 0 && !reader.separator() {
            return Err(ValueError::Number(NumberError {
                offset: reader.absolute(),
            }));
        }
        values.push(reader.number().map_err(ValueError::Number)?);
    }
    reader.finish().map_err(ValueError::Number)?;
    Ok(values)
}

pub fn parse_length(
    source: &[u8],
    span: Span,
    percentage_reference: f64,
) -> Result<f64, ValueError> {
    parse_length_with_limits(
        source,
        span,
        percentage_reference,
        crate::ValueLimits::default(),
    )
}
pub fn parse_length_with_limits(
    source: &[u8],
    span: Span,
    percentage_reference: f64,
    limits: crate::ValueLimits,
) -> Result<f64, ValueError> {
    validate(source, span, limits)?;
    let mut reader = Reader::new(
        span.slice(source).ok_or(ValueError::InvalidSpan)?,
        span.start,
    );
    parse_length_reader(&mut reader, percentage_reference, true)
}

fn parse_length_reader(
    reader: &mut Reader<'_>,
    percentage_reference: f64,
    strict_finish: bool,
) -> Result<f64, ValueError> {
    let mut value = reader.number().map_err(ValueError::Number)?;
    let suffix = reader.index;
    while matches!(
        reader.bytes.get(reader.index),
        Some(b'a'..=b'z' | b'A'..=b'Z' | b'%')
    ) {
        reader.index += 1;
    }
    let unit = &reader.bytes[suffix..reader.index];
    let multiplier = match unit {
        b"" => 1.0,
        b"%" => {
            if !percentage_reference.is_finite() || percentage_reference <= 0.0 {
                return Err(ValueError::Number(NumberError {
                    offset: reader.base + suffix,
                }));
            }
            percentage_reference / 100.0
        }
        _ if unit.eq_ignore_ascii_case(b"px") => 1.0,
        _ if unit.eq_ignore_ascii_case(b"pt") => 96.0 / 72.0,
        _ if unit.eq_ignore_ascii_case(b"mm") => 96.0 / 25.4,
        _ if unit.eq_ignore_ascii_case(b"cm") => 96.0 / 2.54,
        _ if unit.eq_ignore_ascii_case(b"in") => 96.0,
        _ if unit.eq_ignore_ascii_case(b"em") => 16.0,
        _ => {
            return Err(ValueError::UnsupportedUnit {
                offset: reader.base + suffix,
            });
        }
    };
    value *= multiplier;
    if strict_finish {
        reader.finish().map_err(ValueError::Number)?;
    }
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ValueError::Number(NumberError {
            offset: reader.absolute(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn span(source: &[u8]) -> Span {
        Span {
            start: 0,
            end: source.len(),
        }
    }

    #[test]
    fn svg_decimal_grammar_and_separator_rules() {
        assert_eq!(parse_number(b"+.5e2", span(b"+.5e2")), Ok(50.0));
        assert_eq!(parse_number(b"-2.", span(b"-2.")), Ok(-2.0));
        assert!(parse_number(b"NaN", span(b"NaN")).is_err());
        assert!(parse_number(b"1e", span(b"1e")).is_err());
        assert_eq!(
            parse_number_list(b"1 -2, .5E+1", span(b"1 -2, .5E+1"), 3),
            Ok(vec![1.0, -2.0, 5.0])
        );
        assert!(parse_number_list(b"1,,2", span(b"1,,2"), 2).is_err());
        assert!(parse_number_list(b"1,", span(b"1,"), 1).is_err());
        assert!(parse_number_list(b"1\x0c2", span(b"1\x0c2"), 2).is_err());
        assert!(parse_number_list(b"1 2", span(b"1 2"), 1).is_err());
    }

    #[test]
    fn lengths_preserve_svg_units_and_report_absolute_failure_offsets() {
        assert_eq!(parse_length(b"50%", span(b"50%"), 200.0), Ok(100.0));
        assert_eq!(parse_length(b"3px", span(b"3px"), 100.0), Ok(3.0));
        assert_eq!(parse_length(b"7", span(b"7"), 100.0), Ok(7.0));
        assert_eq!(parse_length(b"9pt", span(b"9pt"), 100.0), Ok(12.0));
        assert_eq!(parse_length(b"25.4mm", span(b"25.4mm"), 100.0), Ok(96.0));
        assert_eq!(
            parse_length(b"2CM", span(b"2CM"), 100.0),
            Ok(2.0 * 96.0 / 2.54)
        );
        assert_eq!(parse_length(b"1in", span(b"1in"), 100.0), Ok(96.0));
        assert_eq!(parse_length(b"1em", span(b"1em"), 100.0), Ok(16.0));
        assert!(matches!(
            parse_length(b"2rem", span(b"2rem"), 100.0),
            Err(ValueError::UnsupportedUnit { offset: 1 })
        ));
        assert!(parse_number(b"1e999", span(b"1e999")).is_err());
        let source = b"x 1e999";
        assert_eq!(
            parse_number(
                source,
                Span {
                    start: 2,
                    end: source.len()
                }
            ),
            Err(ValueError::Number(NumberError { offset: 7 }))
        );
    }

    #[test]
    fn cursor_matches_parsers_and_keeps_absolute_position_on_errors() {
        let source = b"xx +.5e2px, 25% \tbad";
        let limits = crate::ValueLimits {
            source_bytes: 128,
            attributes: 3,
        };
        let mut cursor = NumberCursor::new(
            source,
            Span {
                start: 3,
                end: source.len(),
            },
            limits,
        )
        .unwrap();
        assert_eq!(cursor.length(200.0), Ok(50.0));
        assert!(cursor.separator());
        assert_eq!(cursor.length(80.0), Ok(20.0));
        assert!(cursor.separator());
        assert_eq!(
            cursor.number(),
            Err(ValueError::Number(NumberError { offset: 17 }))
        );
        assert_eq!(cursor.absolute(), 17);
        assert!(!cursor.finished());

        let source = b"a 1 -2, .5E+1";
        let mut cursor = NumberCursor::new(
            source,
            Span {
                start: 2,
                end: source.len(),
            },
            crate::ValueLimits::default(),
        )
        .unwrap();
        assert_eq!(cursor.number(), Ok(1.0));
        assert!(cursor.separator());
        assert_eq!(cursor.number(), Ok(-2.0));
        assert!(cursor.separator());
        assert_eq!(cursor.number(), Ok(5.0));
        assert!(cursor.finished());
        assert_eq!(cursor.absolute(), source.len());

        let source = b"xx 50%   ";
        let span = Span {
            start: 3,
            end: source.len(),
        };
        let mut cursor = NumberCursor::new(source, span, crate::ValueLimits::default()).unwrap();
        let cursor_error = cursor.length(0.0).unwrap_err();
        assert_eq!(cursor_error, parse_length(source, span, 0.0).unwrap_err());
        assert_eq!(cursor_error, ValueError::Number(NumberError { offset: 5 }));
        assert!(cursor.finished());
        assert_eq!(cursor.absolute(), source.len());

        let source = b"a 2rem";
        let span = Span {
            start: 2,
            end: source.len(),
        };
        let mut cursor = NumberCursor::new(source, span, crate::ValueLimits::default()).unwrap();
        let cursor_error = cursor.length(10.0).unwrap_err();
        assert_eq!(cursor_error, parse_length(source, span, 10.0).unwrap_err());
        assert_eq!(cursor_error, ValueError::UnsupportedUnit { offset: 3 });
        assert_eq!(cursor.absolute(), 6);

        assert!(matches!(
            NumberCursor::new(
                b"123",
                Span { start: 0, end: 3 },
                crate::ValueLimits {
                    source_bytes: 2,
                    attributes: 0
                }
            ),
            Err(ValueError::SourceLimit)
        ));
        assert!(matches!(
            NumberCursor::new(
                b"123",
                Span { start: 2, end: 4 },
                crate::ValueLimits::default()
            ),
            Err(ValueError::InvalidSpan)
        ));
    }
}
