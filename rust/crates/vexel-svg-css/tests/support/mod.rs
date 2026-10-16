use std::sync::atomic::AtomicBool;
use vexel_base::{BudgetControl, ControlledError, ControlledStep};
use vexel_svg_css::{CssBuildSession, CssError, CssPaint, CssSheet};
use vexel_xml::XmlToken;

// Adapter for verbatim baseline test bodies. Q=0 selects controlled one-shot;
// Q=1 selects real one-unit sessions, using the accepted budget implementation.
pub struct Sheet<'src, const Q: u64>(CssSheet<'src>);
fn original(error: ControlledError<CssError>) -> CssError {
    match error {
        ControlledError::Original(e) => e,
        ControlledError::OutOfMemory => CssError::OutOfMemory,
        e => panic!("unexpected unlimited-control failure: {e:?}"),
    }
}
impl<'src, const Q: u64> Sheet<'src, Q> {
    pub fn create(source: &'src [u8], tokens: &'src [XmlToken]) -> Result<Self, CssError> {
        let cancel = AtomicBool::new(false);
        let mut control = BudgetControl::new(&cancel, u64::MAX);
        let sheet = if Q == 0 {
            CssSheet::create_controlled(source, tokens, &mut control)
        } else {
            let mut session = CssBuildSession::new(source, tokens);
            loop {
                match session.step(Q, &mut control) {
                    ControlledStep::Pending { charged_units } => assert_eq!(charged_units, Q),
                    _ => break session.finish(),
                }
            }
        };
        sheet.map(Self).map_err(original)
    }
    pub fn compute(
        &mut self,
        element: usize,
        parent: Option<&CssPaint<'src>>,
    ) -> Result<CssPaint<'src>, CssError> {
        let cancel = AtomicBool::new(false);
        let mut control = BudgetControl::new(&cancel, u64::MAX);
        if Q == 0 {
            self.0
                .compute_controlled(element, parent, &mut control)
                .map_err(original)
        } else {
            let mut session = self.0.begin_compute(element, parent);
            loop {
                match session.step(Q, &mut control) {
                    ControlledStep::Pending { charged_units } => assert_eq!(charged_units, Q),
                    _ => return session.finish().map_err(original),
                }
            }
        }
    }
    pub fn rule_count(&self) -> usize {
        self.0.rule_count()
    }
    pub fn css_bytes(&self) -> usize {
        self.0.css_bytes()
    }
}
