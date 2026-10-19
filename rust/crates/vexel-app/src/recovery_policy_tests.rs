use super::RecoveryPolicy;
use std::time::{Duration, Instant};

fn instant() -> Instant {
    Instant::now()
}

#[test]
fn clean_session_is_never_eligible() {
    let now = instant();
    let policy = RecoveryPolicy::new();

    assert!(!policy.is_dirty());
    assert!(!policy.is_due(now + Duration::from_secs(60)));
}

#[test]
fn first_snapshot_waits_for_initial_debounce() {
    let edited = instant();
    let mut policy = RecoveryPolicy::new();
    policy.mark_dirty(edited);

    assert!(!policy.is_due(edited + Duration::from_millis(1999)));
    assert!(policy.is_due(edited + Duration::from_secs(2)));
}

#[test]
fn later_edits_do_not_postpone_first_snapshot() {
    let edited = instant();
    let mut policy = RecoveryPolicy::new();
    policy.mark_dirty(edited);
    policy.mark_dirty(edited + Duration::from_secs(1));

    assert!(policy.is_due(edited + Duration::from_secs(2)));
}

#[test]
fn scheduled_attempts_observe_fifteen_second_cadence() {
    let edited = instant();
    let mut policy = RecoveryPolicy::new();
    policy.mark_dirty(edited);
    let first = edited + Duration::from_secs(2);
    assert!(policy.is_due(first));
    policy.record_scheduled(first);

    assert!(!policy.is_due(first + Duration::from_millis(14_999)));
    assert!(policy.is_due(first + Duration::from_secs(15)));
}

#[test]
fn failed_attempt_retries_on_the_same_bounded_cadence() {
    let edited = instant();
    let mut policy = RecoveryPolicy::new();
    policy.mark_dirty(edited);
    let attempt = edited + Duration::from_secs(2);
    policy.record_scheduled(attempt);

    assert!(!policy.is_due(attempt + Duration::from_secs(14)));
    assert!(policy.is_due(attempt + Duration::from_secs(15)));
}

#[test]
fn becoming_clean_resets_debounce_and_cadence() {
    let edited = instant();
    let mut policy = RecoveryPolicy::new();
    policy.mark_dirty(edited);
    policy.record_scheduled(edited + Duration::from_secs(2));
    policy.mark_clean();

    assert!(!policy.is_dirty());
    assert!(!policy.is_due(edited + Duration::from_secs(100)));

    let edited_again = edited + Duration::from_secs(101);
    policy.mark_dirty(edited_again);
    assert!(!policy.is_due(edited_again + Duration::from_millis(1999)));
    assert!(policy.is_due(edited_again + Duration::from_secs(2)));
}

#[test]
fn pending_snapshot_is_latest_wins_and_bounded_to_one_value() {
    let mut pending = None;
    assert!(!RecoveryPolicy::coalesce_pending(&mut pending, 1));
    assert_eq!(pending, Some(1));
    assert!(RecoveryPolicy::coalesce_pending(&mut pending, 2));
    assert_eq!(pending, Some(2));
}
