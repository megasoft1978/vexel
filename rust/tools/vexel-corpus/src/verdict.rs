//! Per-file verdicts and the documented `ComposeError` mapping.

use vexel_app::compose::ComposeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verdict {
    Ok,
    Unsupported,
    Limit,
    Xml,
    Timeout,
    RssExceeded,
    Skipped,
    Crash,
    GuardFailed,
}

impl Verdict {
    pub const ALL: [Verdict; 9] = [
        Verdict::Ok,
        Verdict::Unsupported,
        Verdict::Limit,
        Verdict::Xml,
        Verdict::Timeout,
        Verdict::RssExceeded,
        Verdict::Skipped,
        Verdict::Crash,
        Verdict::GuardFailed,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Verdict::Ok => "ok",
            Verdict::Unsupported => "unsupported",
            Verdict::Limit => "limit",
            Verdict::Xml => "xml",
            Verdict::Timeout => "timeout",
            Verdict::RssExceeded => "rss_exceeded",
            Verdict::Skipped => "skipped",
            Verdict::Crash => "crash",
            Verdict::GuardFailed => "guard_failed",
        }
    }

    pub fn parse(name: &str) -> Option<Verdict> {
        Verdict::ALL.into_iter().find(|v| v.name() == name)
    }
}

/// The documented table (also reproduced in `result.md`):
///
/// | `ComposeError`                     | verdict       |
/// |------------------------------------|---------------|
/// | `Ok(Composed)`                     | `ok`          |
/// | `Unsupported`, `Hidden`            | `unsupported` |
/// | `LimitExceeded`                    | `limit`       |
/// | `Malformed`, `InvalidUtf8`         | `xml`         |
/// | `InvalidSize`, `Cancelled`         | `crash`       |
///
/// `InvalidSize` and `Cancelled` cannot occur for a fixed valid size and a
/// never-cancelled run; seeing one is a tool or pipeline defect, so it is
/// reported as `crash` with an `internal:` detail rather than hidden.
pub fn from_compose_error(error: &ComposeError) -> (Verdict, String) {
    let detail = error.to_string();
    match error {
        ComposeError::Unsupported { .. } | ComposeError::Hidden => (Verdict::Unsupported, detail),
        ComposeError::LimitExceeded => (Verdict::Limit, detail),
        ComposeError::Malformed { .. } | ComposeError::InvalidUtf8 { .. } => (Verdict::Xml, detail),
        ComposeError::InvalidSize | ComposeError::Cancelled => {
            (Verdict::Crash, format!("internal: {detail}"))
        }
    }
}

/// `skipped` when the source is over the threshold (it is then never read).
pub fn skip_for_size(bytes: u64, max_bytes: u64) -> Option<(Verdict, String)> {
    (bytes > max_bytes).then(|| {
        (
            Verdict::Skipped,
            format!("not read: {bytes} bytes exceeds --max-bytes {max_bytes}"),
        )
    })
}

/// Non-zero exit or signal -> `crash`, recording the exit status.
pub fn from_exit(code: Option<i32>, abnormal: bool) -> Option<(Verdict, String)> {
    match (code, abnormal) {
        (Some(0), false) => None,
        (Some(c), false) => Some((Verdict::Crash, format!("exit status {c}"))),
        (Some(c), true) => Some((
            Verdict::Crash,
            format!("terminated abnormally (signal), exit status {c}"),
        )),
        (None, _) => Some((Verdict::Crash, "terminated by signal".to_string())),
    }
}

/// Post-hoc peak above the limit -> `rss_exceeded`.
pub fn from_rss(peak_bytes: Option<u64>, limit_mib: u64) -> Option<(Verdict, String)> {
    let limit = limit_mib.saturating_mul(1024 * 1024);
    peak_bytes.filter(|p| *p > limit).map(|p| {
        (
            Verdict::RssExceeded,
            format!("peak {p} bytes above --rss-limit-mib {limit_mib}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_compose_error_variant_maps() {
        let cases = [
            (ComposeError::InvalidSize, Verdict::Crash),
            (ComposeError::InvalidUtf8 { valid_up_to: 3 }, Verdict::Xml),
            (
                ComposeError::Malformed {
                    detail: "XML",
                    offset: 1,
                },
                Verdict::Xml,
            ),
            (
                ComposeError::Unsupported {
                    feature: "filter",
                    offset: None,
                    element: None,
                },
                Verdict::Unsupported,
            ),
            (ComposeError::Hidden, Verdict::Unsupported),
            (ComposeError::LimitExceeded, Verdict::Limit),
            (ComposeError::Cancelled, Verdict::Crash),
        ];
        for (error, want) in cases {
            assert_eq!(from_compose_error(&error).0, want, "{error:?}");
        }
    }

    #[test]
    fn non_compose_verdicts() {
        assert_eq!(skip_for_size(10, 10), None);
        assert_eq!(skip_for_size(11, 10).unwrap().0, Verdict::Skipped);
        assert_eq!(from_exit(Some(0), false), None);
        assert_eq!(from_exit(Some(101), false).unwrap().0, Verdict::Crash);
        assert_eq!(from_exit(None, false).unwrap().0, Verdict::Crash);
        assert_eq!(from_exit(Some(1), true).unwrap().0, Verdict::Crash);
        assert_eq!(
            from_rss(Some(2 * 1024 * 1024), 1).unwrap().0,
            Verdict::RssExceeded
        );
        assert_eq!(from_rss(Some(1024 * 1024), 1), None);
        assert_eq!(from_rss(None, 1), None);
    }

    #[test]
    fn names_round_trip() {
        for v in Verdict::ALL {
            assert_eq!(Verdict::parse(v.name()), Some(v));
        }
        assert_eq!(Verdict::parse("path"), None);
    }
}
