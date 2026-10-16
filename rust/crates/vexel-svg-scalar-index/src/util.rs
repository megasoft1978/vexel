use vexel_scene::{Diagnostic, ErrorKind, LimitKind, Result, Stop, WorkControl};

pub(crate) const BOUNDARY_WORK_LIMIT: usize = 200_000_000;
pub(crate) const PIXEL_RUN_LIMIT: usize = 8 * 1024 * 1024;
pub(crate) const SCANLINE_VALUES_PER_ROW: usize = 5;
pub(crate) const SCANLINE_MAX_TREE_DEPTH: i32 = 64;

pub(crate) fn add(left: usize, right: usize) -> Result<usize> {
    left.checked_add(right)
        .ok_or_else(|| limit(LimitKind::Arithmetic))
}

pub(crate) fn mul(left: usize, right: usize) -> Result<usize> {
    left.checked_mul(right)
        .ok_or_else(|| limit(LimitKind::Arithmetic))
}

#[cfg(test)]
pub(crate) fn oom() -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::OutOfMemory,
        offset: 0,
    }
}

pub(crate) fn limit(kind: LimitKind) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::Limit(kind),
        offset: 0,
    }
}

pub(crate) fn poll(control: &mut dyn WorkControl, kind: vexel_scene::WorkKind) -> Result<()> {
    control.charge(kind, 1).map_err(|stop| match stop {
        Stop::Canceled => Diagnostic {
            kind: ErrorKind::Canceled,
            offset: 0,
        },
        Stop::Limit => limit(LimitKind::Work),
        Stop::Overflow => limit(LimitKind::Arithmetic),
    })
}

#[cfg(test)]
pub(crate) fn reserve<T>(values: &mut Vec<T>, additional: usize) -> Result<()> {
    values.try_reserve_exact(additional).map_err(|_| oom())
}

pub(crate) fn invalid(offset: usize) -> Diagnostic {
    Diagnostic {
        kind: ErrorKind::InvalidArgument,
        offset,
    }
}

pub(crate) fn source_offset(view: vexel_scene::SceneView<'_>, shape: usize) -> usize {
    view.shapes.get(shape).map_or(0, |s| s.source.element.start)
}

pub(crate) fn pixel_range(low: f64, high: f64, extent: u32) -> Option<(u32, u32)> {
    if extent == 0 || !low.is_finite() || !high.is_finite() || high < low {
        return None;
    }
    let first = (low - 0.875).ceil() - 1.0;
    let last = (high - 0.125).floor() + 1.0;
    if last < 0.0 || first >= extent as f64 {
        return None;
    }
    let first = if first <= 0.0 { 0 } else { first as u32 };
    let last = if last >= extent as f64 {
        extent - 1
    } else {
        last as u32
    };
    (first <= last).then_some((first, last))
}

pub(crate) fn boundary_pixel_range(low: f64, high: f64, extent: u32) -> Option<(u32, u32)> {
    if extent == 0 || !low.is_finite() || !high.is_finite() || high < low {
        return None;
    }
    let first = (low - 1.0).ceil() - 1.0;
    let last = high.floor() + 1.0;
    if last < 0.0 || first >= extent as f64 {
        return None;
    }
    let first = if first <= 0.0 { 0 } else { first as u32 };
    let last = if last >= extent as f64 {
        extent - 1
    } else {
        last as u32
    };
    (first <= last).then_some((first, last))
}
