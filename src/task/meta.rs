use crate::runtime::Runtime;

/// Task related metadata.
#[derive(Clone)]
pub(crate) struct Metadata {
    /// unique id for the task.
    pub id: u64,
    /// a reference to the runtime where the task was spawned.
    pub rt: Runtime,
}
