#![forbid(unsafe_code)]
//! Bounded, source-span based SVG values and viewport mappings.
//!
//! `AttributeList` adapts tokenizer token arrays without owning or rewriting
//! source. Callers retain original bytes and diagnostics remain byte offsets.

mod number;
mod transform;
mod viewport;

pub use number::{
    NumberCursor, NumberError, ValueError, parse_length, parse_length_with_limits, parse_number,
    parse_number_list, parse_number_list_with_limits, parse_number_with_limits,
};
pub use transform::{
    TransformError, affine_inverse, parse_transform, parse_transform_node,
    parse_transform_with_limits,
};
pub use viewport::{
    Aspect, MappingError, MappingErrorKind, NestedViewport, RootViewport, nested_viewport,
    parse_preserve_aspect, parse_preserve_aspect_with_limits, parse_root_viewport,
    parse_symbol_mapping, symbol_attributes_supported, viewbox_transform,
};

use vexel_base::Span;

pub const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_ATTRIBUTES: usize = 256;
pub const MAX_POLICY_SOURCE_BYTES: usize = 1024 * 1024 * 1024;
pub const MAX_POLICY_ATTRIBUTES: usize = 4096;

/// Caller-selected caps, clamped to the crate's hard safety limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueLimits {
    pub source_bytes: usize,
    pub attributes: usize,
}

impl Default for ValueLimits {
    fn default() -> Self {
        Self {
            source_bytes: MAX_SOURCE_BYTES,
            attributes: MAX_ATTRIBUTES,
        }
    }
}

impl ValueLimits {
    /// Clamp caller limits to the hard source and attribute safety caps.
    pub const fn effective(self) -> Self {
        Self {
            source_bytes: if self.source_bytes < MAX_POLICY_SOURCE_BYTES {
                self.source_bytes
            } else {
                MAX_POLICY_SOURCE_BYTES
            },
            attributes: if self.attributes < MAX_POLICY_ATTRIBUTES {
                self.attributes
            } else {
                MAX_POLICY_ATTRIBUTES
            },
        }
    }
}

/// One XML attribute represented by checked name/value ranges in original source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attribute {
    pub name: Span,
    pub value: Span,
    /// Namespace prefix range, if tokenized as a prefixed name.
    pub prefix: Option<Span>,
}

/// Bounded view of one element's attributes and source-backed element name.
#[derive(Debug, Clone, Copy)]
pub struct AttributeList<'a> {
    source: &'a [u8],
    element_name: Span,
    attributes: &'a [Attribute],
    limits: ValueLimits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterError {
    SourceLimit,
    TooManyAttributes,
    InvalidSpan,
    InvalidName,
}

impl<'a> AttributeList<'a> {
    /// Validate all spans against the original, borrowed source and retain the effective policy.
    pub fn new(
        source: &'a [u8],
        element_name: Span,
        attributes: &'a [Attribute],
    ) -> Result<Self, AdapterError> {
        Self::new_with_limits(source, element_name, attributes, ValueLimits::default())
    }

    pub fn new_with_limits(
        source: &'a [u8],
        element_name: Span,
        attributes: &'a [Attribute],
        limits: ValueLimits,
    ) -> Result<Self, AdapterError> {
        let limits = limits.effective();
        if source.len() > limits.source_bytes {
            return Err(AdapterError::SourceLimit);
        }
        if attributes.len() > limits.attributes {
            return Err(AdapterError::TooManyAttributes);
        }
        checked_slice(source, element_name).ok_or(AdapterError::InvalidSpan)?;
        for attribute in attributes {
            let name = checked_slice(source, attribute.name).ok_or(AdapterError::InvalidSpan)?;
            checked_slice(source, attribute.value).ok_or(AdapterError::InvalidSpan)?;
            if name.is_empty() || !name.iter().all(|byte| is_name_byte(*byte)) {
                return Err(AdapterError::InvalidName);
            }
            if let Some(prefix) = attribute.prefix {
                checked_slice(source, prefix).ok_or(AdapterError::InvalidSpan)?;
                if prefix.start != attribute.name.start || prefix.end >= attribute.name.end {
                    return Err(AdapterError::InvalidName);
                }
            }
        }
        Ok(Self {
            source,
            element_name,
            attributes,
            limits,
        })
    }

    pub fn source(self) -> &'a [u8] {
        self.source
    }
    pub fn element_name(self) -> Span {
        self.element_name
    }
    pub fn attributes(self) -> &'a [Attribute] {
        self.attributes
    }
    pub const fn limits(self) -> ValueLimits {
        self.limits
    }

    /// Look up an unprefixed attribute by exact, case-sensitive XML name.
    pub fn get(self, name: &[u8]) -> Option<Span> {
        self.attributes.iter().find_map(|attribute| {
            (attribute.prefix.is_none() && checked_slice(self.source, attribute.name) == Some(name))
                .then_some(attribute.value)
        })
    }

    pub(crate) fn value(self, span: Span) -> Option<&'a [u8]> {
        checked_slice(self.source, span)
    }
}

fn checked_slice(source: &[u8], span: Span) -> Option<&[u8]> {
    (span.start <= span.end).then_some(())?;
    source.get(span.start..span.end)
}

fn is_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':' | b'-' | b'.') || byte >= 0x80
}

#[cfg(test)]
mod adapter_tests {
    use super::*;

    #[test]
    fn validates_bounded_source_backed_attribute_spans_and_namespace_lookup() {
        let source = b"svg width 10px xlink:href #a";
        let attrs = [
            Attribute {
                name: Span { start: 4, end: 9 },
                value: Span { start: 10, end: 14 },
                prefix: None,
            },
            Attribute {
                name: Span { start: 15, end: 25 },
                value: Span { start: 26, end: 28 },
                prefix: Some(Span { start: 15, end: 20 }),
            },
        ];
        let list = AttributeList::new(source, Span { start: 0, end: 3 }, &attrs).unwrap();
        assert_eq!(list.get(b"width"), Some(Span { start: 10, end: 14 }));
        assert_eq!(list.get(b"href"), None);
        let invalid = [Attribute {
            name: Span { start: 4, end: 90 },
            value: Span { start: 10, end: 14 },
            prefix: None,
        }];
        assert_eq!(
            AttributeList::new(source, Span { start: 0, end: 3 }, &invalid).unwrap_err(),
            AdapterError::InvalidSpan
        );
    }

    #[test]
    fn enforces_source_and_attribute_count_caps() {
        let oversized_source = vec![0; MAX_SOURCE_BYTES + 1];
        assert_eq!(
            AttributeList::new(&oversized_source, Span { start: 0, end: 0 }, &[]).unwrap_err(),
            AdapterError::SourceLimit
        );
        let too_many = vec![
            Attribute {
                name: Span { start: 0, end: 1 },
                value: Span { start: 1, end: 1 },
                prefix: None
            };
            MAX_ATTRIBUTES + 1
        ];
        assert_eq!(
            AttributeList::new(b"a", Span { start: 0, end: 1 }, &too_many).unwrap_err(),
            AdapterError::TooManyAttributes
        );
    }

    #[test]
    fn caller_policy_clamps_hard_caps_and_validates_all_source_backed_spans() {
        let policy = ValueLimits {
            source_bytes: 4096,
            attributes: 300,
        };
        let source = vec![b'x'; 300];
        let attributes = vec![
            Attribute {
                name: Span { start: 0, end: 1 },
                value: Span { start: 1, end: 1 },
                prefix: None
            };
            257
        ];
        let list =
            AttributeList::new_with_limits(&source, Span { start: 0, end: 1 }, &attributes, policy)
                .unwrap();
        assert_eq!(list.limits(), policy);
        assert_eq!(
            AttributeList::new(&source, Span { start: 0, end: 1 }, &attributes).unwrap_err(),
            AdapterError::TooManyAttributes
        );

        let invalid = [Attribute {
            name: Span { start: 0, end: 1 },
            value: Span { start: 1, end: 301 },
            prefix: None,
        }];
        assert_eq!(
            AttributeList::new_with_limits(&source, Span { start: 0, end: 1 }, &invalid, policy)
                .unwrap_err(),
            AdapterError::InvalidSpan
        );
        let hard = ValueLimits {
            source_bytes: usize::MAX,
            attributes: usize::MAX,
        }
        .effective();
        assert_eq!(
            hard,
            ValueLimits {
                source_bytes: MAX_POLICY_SOURCE_BYTES,
                attributes: MAX_POLICY_ATTRIBUTES
            }
        );
        let at_hard_cap = vec![attributes[0]; MAX_POLICY_ATTRIBUTES];
        assert!(
            AttributeList::new_with_limits(&source, Span { start: 0, end: 1 }, &at_hard_cap, hard)
                .is_ok()
        );
        let too_many = vec![attributes[0]; MAX_POLICY_ATTRIBUTES + 1];
        assert_eq!(
            AttributeList::new_with_limits(&source, Span { start: 0, end: 1 }, &too_many, hard)
                .unwrap_err(),
            AdapterError::TooManyAttributes
        );
    }
}
