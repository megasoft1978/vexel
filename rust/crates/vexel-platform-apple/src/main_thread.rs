use objc2::MainThreadMarker;

/// Opaque proof that this value was acquired on Apple's process main thread.
///
/// `MainThread` is neither `Send` nor `Sync`, inherited from objc2's marker.
/// It carries no native pointer and remains valid for the lifetime of the
/// process main thread.
///
/// ```compile_fail
/// use vexel_platform_apple::MainThread;
/// fn require_send<T: Send>() {}
/// require_send::<MainThread>();
/// ```
/// ```compile_fail
/// use vexel_platform_apple::MainThread;
/// fn require_sync<T: Sync>() {}
/// require_sync::<MainThread>();
/// ```
#[derive(Clone, Copy, Debug)]
pub struct MainThread(MainThreadMarker);

/// Failure to acquire the main-thread capability from the calling thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MainThreadError {
    _private: (),
}

impl MainThread {
    /// Acquires the capability only when objc2 confirms this is the main
    /// thread through its `MainThreadMarker::new` check (`pthread_main_np`).
    pub fn acquire() -> Result<Self, MainThreadError> {
        MainThreadMarker::new()
            .map(Self)
            .ok_or(MainThreadError { _private: () })
    }

    /// Gives sibling native implementation modules the binding marker while
    /// keeping it out of the crate's public API.
    #[allow(dead_code)] // Used by the native modules introduced in follow-on work orders.
    pub(crate) fn marker(&self) -> MainThreadMarker {
        self.0
    }
}
