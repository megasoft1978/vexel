//! Canonical lexical ownership. Only the owned tokenizer can populate tokens;
//! callers cannot pair another document's token spans with arbitrary bytes.
use crate::types::{invalid, limit, stopped};
use crate::*;
use vexel_xml::{StreamStatus, XmlError, XmlToken, XmlTokenStream};
#[derive(Debug)]
pub struct LexicalDocument {
    source: SourceOwner,
    tokens: Vec<XmlToken>,
}
impl LexicalDocument {
    pub fn parse(
        source: SourceOwner,
        max_token_storage_bytes: usize,
        control: &mut dyn WorkControl,
    ) -> Result<Self> {
        control.checkpoint().map_err(stopped)?;
        if source.len() > 1 << 30 {
            return Err(limit(LimitKind::SourceBytes));
        }
        let maximum = max_token_storage_bytes.min(256 * 1024 * 1024);
        if maximum < std::mem::size_of::<XmlToken>() {
            return Err(limit(LimitKind::TokenBytes));
        }
        let mut stream = XmlTokenStream::new(&source, maximum).map_err(|_| Diagnostic {
            kind: ErrorKind::OutOfMemory,
            offset: 0,
        })?;
        loop {
            let mut reason = None;
            let status = {
                let mut cancel = || match control.charge(WorkKind::SourceRead, 1) {
                    Ok(()) => false,
                    Err(stop) => {
                        reason.get_or_insert(stop);
                        true
                    }
                };
                stream.step_with_cancel(256, &mut cancel)
            };
            if let Some(stop) = reason {
                return Err(stopped(stop));
            }
            match status {
                StreamStatus::Ok => {}
                StreamStatus::Complete => break,
                StreamStatus::Canceled => return Err(stopped(Stop::Canceled)),
                StreamStatus::Failed => {
                    let d = stream.result().diagnostic;
                    return Err(Diagnostic {
                        kind: if d.category == XmlError::OutputExhausted {
                            ErrorKind::Limit(LimitKind::TokenBytes)
                        } else {
                            ErrorKind::Xml(d.category)
                        },
                        offset: d.offset,
                    });
                }
            }
        }
        let tokens = stream.take().ok_or_else(|| invalid(0))?;
        drop(stream);
        Ok(Self { source, tokens })
    }
    pub fn source_owner(&self) -> &SourceOwner {
        &self.source
    }
    pub fn source(&self) -> &[u8] {
        &self.source
    }
    pub fn tokens(&self) -> &[XmlToken] {
        &self.tokens
    }
    pub fn token_storage_bytes(&self) -> Result<usize> {
        crate::memory::bytes(&self.tokens)
    }
}
