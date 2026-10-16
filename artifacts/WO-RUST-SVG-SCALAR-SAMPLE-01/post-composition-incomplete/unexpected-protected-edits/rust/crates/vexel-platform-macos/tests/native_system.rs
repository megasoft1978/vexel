use std::fs::File;
use std::os::fd::AsFd;
use std::os::unix::net::UnixStream;

use vexel_platform_macos::{
    MAX_RANDOM_BYTES, SystemError, current_uid, fill_random, peer_credentials,
};

#[test]
fn empty_random_request_succeeds() {
    assert_eq!(fill_random(&mut []), Ok(()));
}

#[test]
fn small_random_request_fills_the_buffer() {
    let mut bytes = [0xA5; 16];

    assert_eq!(fill_random(&mut bytes), Ok(()));
    assert_ne!(bytes, [0xA5; 16]);
}

#[test]
fn maximum_random_request_is_accepted() {
    let mut bytes = [0xA5; MAX_RANDOM_BYTES];

    assert_eq!(fill_random(&mut bytes), Ok(()));
    assert_ne!(bytes, [0xA5; MAX_RANDOM_BYTES]);
}

#[test]
fn random_request_over_the_limit_is_rejected_without_changing_output() {
    let mut bytes = [0xA5; MAX_RANDOM_BYTES + 1];

    assert_eq!(
        fill_random(&mut bytes),
        Err(SystemError::RandomRequestTooLarge {
            requested: MAX_RANDOM_BYTES + 1,
            maximum: MAX_RANDOM_BYTES,
        })
    );
    assert_eq!(bytes, [0xA5; MAX_RANDOM_BYTES + 1]);
}

#[test]
fn connected_unix_socket_reports_the_current_user() {
    let (left, _right) = UnixStream::pair().expect("create local socket pair");

    let (peer_uid, _peer_gid) = peer_credentials(left.as_fd()).expect("read peer credentials");
    assert_eq!(peer_uid, current_uid());
}

#[test]
fn non_socket_descriptor_returns_native_error_classification() {
    let file = File::open("/dev/null").expect("open local null device");

    assert!(matches!(
        peer_credentials(file.as_fd()),
        Err(SystemError::PeerCredentialsErrno(errno)) if errno != 0
    ));
}
