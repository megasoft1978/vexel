//! Canonical late animation installation after successful complete traversal.
use crate::Decoder;
use vexel_base::Stop;
use vexel_scene::*;
impl Decoder<'_, '_> {
    pub(crate) fn finalize_animation(&mut self, control: &mut dyn WorkControl) -> Result<()> {
        let shared = &self.shared;
        let tokens = shared.tokens();
        let animation =
            vexel_svg_animation::parse_owned_controlled(shared.source_owner(), tokens, control)
                .map_err(|e| match e {
                    vexel_base::ControlledError::Original(
                        vexel_svg_animation::ParseError::TrackLimit { offset },
                    ) => Diagnostic {
                        kind: ErrorKind::Limit(LimitKind::Animation),
                        offset,
                    },
                    vexel_base::ControlledError::Original(
                        vexel_svg_animation::ParseError::Allocation,
                    ) => Diagnostic {
                        kind: ErrorKind::OutOfMemory,
                        offset: 0,
                    },
                    vexel_base::ControlledError::Canceled => {
                        crate::source::stopped(Stop::Canceled, 0)
                    }
                    vexel_base::ControlledError::Limit => crate::source::stopped(Stop::Limit, 0),
                    vexel_base::ControlledError::Overflow => {
                        crate::source::stopped(Stop::Overflow, 0)
                    }
                    vexel_base::ControlledError::OutOfMemory => Diagnostic {
                        kind: ErrorKind::OutOfMemory,
                        offset: 0,
                    },
                })?;
        let generation = self.generation();
        self.builder
            .as_mut()
            .ok_or_else(|| crate::invalid(0))?
            .finalize_decode(animation, generation, control)
            .map_err(|e| e.diagnostic)
    }
}
