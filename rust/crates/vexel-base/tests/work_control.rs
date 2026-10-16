use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Barrier;
use vexel_base::{BudgetControl, ControlledError, ControlledStep, Stop, WorkControl, WorkKind, WorkLimits};

#[test]
fn object_safe_trait_and_every_kind_are_accounted_exactly() {
    let cancel = AtomicBool::new(false);
    let mut budget = BudgetControl::new(&cancel, u64::MAX);
    let control: &mut dyn WorkControl = &mut budget;
    for (index, kind) in WorkKind::ALL.into_iter().enumerate() {
        assert_eq!(control.charge(kind, (index + 1) as u64), Ok(()));
    }
    assert_eq!(control.checkpoint(), Ok(()));
    let stats = budget.stats();
    assert_eq!(stats.total_units(), (1..=21).sum());
    for (index, kind) in WorkKind::ALL.into_iter().enumerate() {
        assert_eq!(stats.units(kind), (index + 1) as u64);
    }
}

#[test]
fn zero_and_boundary_charges_and_caps_are_exact() {
    let cancel = AtomicBool::new(false);
    let limits = WorkLimits::unlimited().with_limit(WorkKind::Edge, 3);
    let mut budget = BudgetControl::with_limits(&cancel, 5, limits);
    assert_eq!(budget.charge(WorkKind::Edge, 0), Ok(()));
    assert_eq!(budget.stats().total_units(), 0);
    assert_eq!(budget.charge(WorkKind::Edge, 3), Ok(()));
    let before = budget.stats();
    assert_eq!(budget.charge(WorkKind::Edge, 1), Err(Stop::Limit));
    assert_eq!(budget.stats(), before);
    assert_eq!(budget.checkpoint(), Err(Stop::Limit));

    let mut total = BudgetControl::new(&cancel, 2);
    assert_eq!(total.charge(WorkKind::Media, 2), Ok(()));
    let before = total.stats();
    assert_eq!(total.charge(WorkKind::SourceRead, 1), Err(Stop::Limit));
    assert_eq!(total.stats(), before);
}

#[test]
fn overflow_is_checked_without_partial_counter_updates() {
    let cancel = AtomicBool::new(false);
    let mut budget = BudgetControl::new(&cancel, u64::MAX);
    assert_eq!(budget.charge(WorkKind::TokenVisit, u64::MAX), Ok(()));
    let before = budget.stats();
    assert_eq!(budget.charge(WorkKind::TokenVisit, 1), Err(Stop::Overflow));
    assert_eq!(budget.stats(), before);
    assert_eq!(budget.checkpoint(), Err(Stop::Overflow));
}

#[test]
fn first_stop_is_sticky_and_does_not_reset() {
    let cancel = AtomicBool::new(false);
    let mut budget = BudgetControl::new(&cancel, 0);
    assert_eq!(budget.charge(WorkKind::Edge, 1), Err(Stop::Limit));
    cancel.store(true, Ordering::Release);
    let before = budget.stats();
    assert_eq!(budget.charge(WorkKind::Media, 0), Err(Stop::Limit));
    assert_eq!(budget.checkpoint(), Err(Stop::Limit));
    assert_eq!(budget.stats(), before);

    let mut canceled = BudgetControl::new(&cancel, u64::MAX);
    assert_eq!(canceled.charge(WorkKind::Edge, 0), Err(Stop::Canceled));
    cancel.store(false, Ordering::Release);
    assert_eq!(canceled.checkpoint(), Err(Stop::Canceled));
    assert_eq!(canceled.stats().total_units(), 0);
}

#[test]
fn scoped_worker_sets_real_cancel_flag_and_checkpoint_observes_it() {
    let cancel = AtomicBool::new(false);
    let barrier = Barrier::new(2);
    std::thread::scope(|scope| {
        let worker_cancel = &cancel;
        let worker_barrier = &barrier;
        let worker = scope.spawn(move || {
            worker_cancel.store(true, Ordering::Release);
            worker_barrier.wait();
        });
        barrier.wait();
        worker.join().unwrap();
    });
    let mut budget = BudgetControl::new(&cancel, 8);
    assert_eq!(budget.checkpoint(), Err(Stop::Canceled));
    assert_eq!(budget.stats().total_units(), 0);
}

#[test]
fn checkpoint_observes_worker_cancel_before_worker_is_released() {
    let cancel = AtomicBool::new(false);
    let first_barrier = Barrier::new(2);
    let second_barrier = Barrier::new(2);
    let mut budget = BudgetControl::new(&cancel, 8);
    assert_eq!(budget.charge(WorkKind::TokenVisit, 3), Ok(()));
    let prior_stats = budget.stats();

    std::thread::scope(|scope| {
        let worker_cancel = &cancel;
        let worker_first = &first_barrier;
        let worker_second = &second_barrier;
        let worker = scope.spawn(move || {
            worker_cancel.store(true, Ordering::Release);
            worker_first.wait();
            worker_second.wait();
        });

        first_barrier.wait();
        assert_eq!(budget.checkpoint(), Err(Stop::Canceled));
        assert_eq!(budget.stats(), prior_stats);
        assert_eq!(budget.charge(WorkKind::Media, 1), Err(Stop::Canceled));
        assert_eq!(budget.stats(), prior_stats);

        second_barrier.wait();
        worker.join().unwrap();
    });
}

#[test]
fn controlled_types_preserve_each_stop_variant() {
    assert_eq!(ControlledError::<()>::from(Stop::Canceled), ControlledError::Canceled);
    assert_eq!(ControlledError::<()>::from(Stop::Limit), ControlledError::Limit);
    assert_eq!(ControlledError::<()>::from(Stop::Overflow), ControlledError::Overflow);
    assert_eq!(ControlledStep::<u8, ()>::Pending { charged_units: 4 }, ControlledStep::Pending { charged_units: 4 });
    assert_eq!(ControlledError::Original("source"), ControlledError::Original("source"));
    assert_eq!(ControlledError::<()>::OutOfMemory, ControlledError::OutOfMemory);
}
