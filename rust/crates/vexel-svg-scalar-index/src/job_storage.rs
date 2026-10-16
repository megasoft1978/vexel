//! Admitted initialization and transaction copies; no payload Vec escapes a job.
use crate::util::*;
use vexel_scene::*;

pub(crate) fn initialize<T: JobElement>(
    job: &mut RasterJob,
    h: JobBuffer<T>,
    count: usize,
    value: T,
    control: &mut dyn WorkControl,
) -> Result<()> {
    job.reserve(h, count, control)?;
    let old = job.slice(h)?.len();
    let result = (|| {
        while job.slice(h)?.len() < count {
            job.extend(h, &[value], control)?;
        }
        Ok(())
    })();
    if result.is_err() {
        job.truncate(h, old)?;
    }
    result
}

pub(crate) fn save<T: JobElement>(
    job: &mut RasterJob,
    from: JobBuffer<T>,
    to: JobBuffer<T>,
    control: &mut dyn WorkControl,
) -> Result<()> {
    let len = job.slice(from)?.len();
    if job.slice(to)?.len() < len {
        return Err(limit(LimitKind::Scratch));
    }
    for i in 0..len {
        poll(control, WorkKind::SourceRead)?;
        let value = job.slice(from)?[i];
        job.slice_mut(to)?[i] = value;
    }
    Ok(())
}

/// Failure cleanup must finish after cancellation. Both slices already exist;
/// this bounded restoration allocates nothing and cannot fail through control.
pub(crate) fn restore<T: JobElement>(
    job: &mut RasterJob,
    from: JobBuffer<T>,
    to: JobBuffer<T>,
) -> Result<()> {
    let len = job.slice(to)?.len();
    if job.slice(from)?.len() < len {
        return Err(invalid(0));
    }
    for i in 0..len {
        let value = job.slice(from)?[i];
        job.slice_mut(to)?[i] = value;
    }
    Ok(())
}
