use crate::Track;
use crate::control::{Meter, StopAfter};
use crate::motion::{parse_motion, resolve_motion_path};
use crate::parse::contains_controlled;
use crate::parse_values::{attr0, element_span, number, number_list, path_shape, split};
use vexel_base::{Stop, WorkKind};
use vexel_xml::{XmlError, XmlLimits, XmlSpan, tokenize};

#[test]
fn attribute_and_element_scans_poll_inside_token_walks() {
    let mut source = String::from("<svg><animate");
    for i in 0..80 {
        source.push_str(&format!(" a{i}='v'"));
    }
    source.push_str(" wanted='x'/></svg>");
    let (tokens, report) = tokenize(source.as_bytes(), XmlLimits::default());
    assert_eq!(report.diagnostic.category, XmlError::None);
    let tokens = &tokens[..report.token_count];
    let animation = tokens
        .iter()
        .position(|t| t.local_name.slice(source.as_bytes()) == Some(b"animate"))
        .unwrap();
    let mut stop = StopAfter::new(WorkKind::TokenVisit, 17);
    let mut meter = Meter::controlled(&mut stop);
    assert_eq!(
        attr0(source.as_bytes(), tokens, animation, b"wanted", &mut meter),
        XmlSpan::default()
    );
    assert_eq!(meter.stop(), Some(Stop::Canceled));
    drop(meter);
    assert_eq!(stop.seen, 17);

    let xml = b"<svg><g><rect/></g></svg>";
    let (tokens, report) = tokenize(xml, XmlLimits::default());
    assert_eq!(report.diagnostic.category, XmlError::None);
    let g = tokens
        .iter()
        .position(|t| t.local_name.slice(xml) == Some(b"g"))
        .unwrap();
    let mut stop = StopAfter::new(WorkKind::TokenVisit, 2);
    let mut meter = Meter::controlled(&mut stop);
    assert!(element_span(&tokens, g, &mut meter).is_none());
    assert_eq!(meter.stop(), Some(Stop::Canceled));
}

#[test]
fn value_split_number_and_path_scans_poll_per_consumed_unit() {
    let mut source = String::from("1;");
    source.push_str(&"2;".repeat(80));
    let span = XmlSpan {
        offset: 0,
        length: source.len(),
    };
    let mut stop = StopAfter::new(WorkKind::SourceRead, 29);
    let mut meter = Meter::controlled(&mut stop);
    let mut allocation_failed = false;
    assert!(split(source.as_bytes(), span, &mut allocation_failed, &mut meter).is_none());
    assert_eq!(meter.stop(), Some(Stop::Canceled));

    let number_bytes = format!("1{}", " ".repeat(4096));
    let mut stop = StopAfter::new(WorkKind::SourceRead, 31);
    let mut meter = Meter::controlled(&mut stop);
    assert!(number_list(number_bytes.as_bytes(), 4, &mut meter).is_none());
    assert_eq!(meter.stop(), Some(Stop::Canceled));

    let scalar = "9".repeat(100);
    let mut stop = StopAfter::new(WorkKind::SourceRead, 31);
    let mut meter = Meter::controlled(&mut stop);
    assert!(
        number(
            scalar.as_bytes(),
            XmlSpan {
                offset: 0,
                length: scalar.len(),
            },
            &mut meter,
        )
        .is_none()
    );
    assert_eq!(meter.stop(), Some(Stop::Canceled));

    let path = format!("M0 0{}", " ".repeat(4096));
    let mut stop = StopAfter::new(WorkKind::SourceRead, 37);
    let mut meter = Meter::controlled(&mut stop);
    assert!(
        path_shape(
            path.as_bytes(),
            XmlSpan {
                offset: 0,
                length: path.len(),
            },
            &mut meter,
        )
        .is_none()
    );
    assert_eq!(meter.stop(), Some(Stop::Canceled));
}

#[test]
fn mpath_search_polls_while_scanning_ids_and_attributes() {
    let mut source = String::from("<svg><defs>");
    for i in 0..48 {
        source.push_str(&format!("<path id='p{i}' d='M0 0 L1 0'/>"));
    }
    source.push_str(
        "</defs><g><animateMotion dur='1s'><mpath href='#missing'/></animateMotion></g></svg>",
    );
    let (tokens, report) = tokenize(source.as_bytes(), XmlLimits::default());
    assert_eq!(report.diagnostic.category, XmlError::None);
    let tokens = &tokens[..report.token_count];
    let animation = tokens
        .iter()
        .position(|t| t.local_name.slice(source.as_bytes()) == Some(b"animateMotion"))
        .unwrap();
    let mut stop = StopAfter::new(WorkKind::ReferenceProbe, 19);
    let mut meter = Meter::controlled(&mut stop);
    assert!(resolve_motion_path(source.as_bytes(), tokens, animation, &mut meter).is_none());
    assert_eq!(meter.stop(), Some(Stop::Canceled));
    drop(meter);
    assert_eq!(stop.seen, 19);
}

#[test]
fn long_mpath_identifier_comparison_polls_inside_id_bytes() {
    let id = "i".repeat(4096);
    let source = format!(
        "<svg><defs><path id='{id}' d='M0 0 L1 0'/></defs><g><animateMotion dur='1s'><mpath href='#{id}'/></animateMotion></g></svg>"
    );
    let (tokens, report) = tokenize(source.as_bytes(), XmlLimits::default());
    assert_eq!(report.diagnostic.category, XmlError::None);
    let tokens = &tokens[..report.token_count];
    let animation = tokens
        .iter()
        .position(|t| t.local_name.slice(source.as_bytes()) == Some(b"animateMotion"))
        .unwrap();
    let mut stop = StopAfter::new(WorkKind::SourceRead, 53);
    let mut meter = Meter::controlled(&mut stop);
    assert!(resolve_motion_path(source.as_bytes(), tokens, animation, &mut meter).is_none());
    assert_eq!(meter.stop(), Some(Stop::Canceled));
    drop(meter);
    assert_eq!(stop.seen, 53);
}

#[test]
fn motion_whitespace_and_number_lexing_poll_during_path_scan() {
    let source = format!("M0 0{}", " ".repeat(4096));
    let mut track = Track::default();
    let mut allocation_failed = false;
    let mut stop = StopAfter::new(WorkKind::SourceRead, 23);
    let mut meter = Meter::controlled(&mut stop);
    assert!(!parse_motion(
        source.as_bytes(),
        XmlSpan {
            offset: 0,
            length: source.len(),
        },
        &mut track,
        &mut allocation_failed,
        &mut meter,
    ));
    assert_eq!(meter.stop(), Some(Stop::Canceled));
}

#[test]
fn style_substring_search_polls_while_matching_text() {
    let text = format!("{}@keyframx{}", "x".repeat(2048), "y".repeat(2048));
    let mut stop = StopAfter::new(WorkKind::SourceRead, 97);
    let mut meter = Meter::controlled(&mut stop);
    assert_eq!(
        contains_controlled(text.as_bytes(), b"@keyframes", &mut meter),
        None
    );
    assert_eq!(meter.stop(), Some(Stop::Canceled));
}
