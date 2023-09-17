use crate::runtime::{current_unwrap, ThreadPool, THREAD_POOL};

use super::JoinHandle;

pub fn spawn_blocking<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    let rt = current_unwrap("spawn_blocking");
    THREAD_POOL
        .get_or_init(|| ThreadPool::new(rt.config))
        .spawn_blocking(f)
}
