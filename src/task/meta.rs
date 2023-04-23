use crate::runtime::Runtime;

/// Task related metadata.
#[derive(Clone)]
pub(crate) struct Metadata {
    /// unique id for the task.
    pub id: u64,
    /// a reference to the runtime where the task was spawned.
    pub rt: Runtime,
    /// whether to ignore abort signals.
    ///
    /// During shutdown all tasks are signaled for abort, but
    /// not all task should be aborted, since some of them are
    /// waiting for cancellation completion events from io-uring.
    ///
    /// Those tasks are marked with ignore_abort so they don't get
    /// aborted and respawned on a loop.
    pub ignore_abort: bool,
}
