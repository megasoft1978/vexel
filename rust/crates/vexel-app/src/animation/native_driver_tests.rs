use crate::animation::native_driver::NativeDriver;
use crate::animation::playback::ManualClock;
use vexel_timeline::LoopMode;

#[test]
fn samples_native_playback_by_flooring_and_clamping_integer_ticks() {
    let mut driver = NativeDriver::new(ManualClock::default());
    driver.duration_ticks = 24;
    driver.ticks_per_second = 24;
    driver.loop_mode = LoopMode::Once;

    assert_eq!(driver.sample_tick(0.0), 0);
    assert_eq!(driver.sample_tick(1.0 / 24.0 - 0.0001), 0);
    assert_eq!(driver.sample_tick(1.0 / 24.0), 1);
    assert_eq!(driver.sample_tick(1.0), 23);
}

#[test]
fn ping_pong_reflects_the_integer_tick_sequence() {
    let mut driver = NativeDriver::new(ManualClock::default());
    driver.duration_ticks = 4;
    driver.ticks_per_second = 1;
    driver.loop_mode = LoopMode::PingPong;

    assert_eq!(driver.sample_tick(0.0), 0);
    assert_eq!(driver.sample_tick(2.0), 2);
    assert_eq!(driver.sample_tick(3.0), 3);
    assert_eq!(driver.sample_tick(4.0), 2);
    assert_eq!(driver.sample_tick(5.0), 1);
    assert_eq!(driver.sample_tick(6.0), 0);
}
