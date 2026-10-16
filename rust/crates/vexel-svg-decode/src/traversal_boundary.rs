//! Request boundaries that keep scene mutations inside their owning transaction.
use crate::{
    decode::Decoder,
    source::attribute,
    tasks::{Finish, Task},
};
use vexel_scene::*;
use vexel_svg_css::Property;

impl Decoder<'_, '_> {
    pub(crate) fn shape_request_boundary(&self, task: &Task<'_>) -> bool {
        if self.custom_request.is_some_and(|request| {
            self.shape_request_top()
                .is_some_and(|(_, active, _)| active == request)
        }) {
            return false;
        }
        if matches!(
            task,
            Task::Markers { .. }
                | Task::MarkerDone { .. }
                | Task::ExpansionPost { .. }
                | Task::Text { .. }
                | Task::Finish(Finish { group: Some(_), .. })
                | Task::Finish(Finish {
                    reference: Some(_),
                    ..
                })
        ) {
            return true;
        }
        if matches!(task, Task::PatternPlan { .. }) && !self.scoped_static {
            return true;
        }
        let clip_finish = match task {
            Task::Finish(node) => {
                node.style
                    .value(Property::ClipPath)
                    .is_some_and(|value| !value.bytes.eq_ignore_ascii_case(b"none"))
                    || attribute(
                        self.shared.source(),
                        self.shared.tokens(),
                        node.element,
                        b"clip-path",
                        None,
                    )
                    .and_then(|value| value.slice(self.shared.source()))
                    .is_some_and(|value| !value.eq_ignore_ascii_case(b"none"))
            }
            _ => false,
        };
        clip_finish
            || matches!(
                task,
                Task::DefinitionDone(work)
                    if !matches!(work.descriptor, DefinitionDescriptor::Pattern(_))
            )
    }
}
