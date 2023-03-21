use std::collections::HashMap;
use std::rc::Rc;
use std::task::Waker;

use crate::runtime::current_unwrap;

#[derive(Clone, Default)]
pub struct ThreadPoolHandle {
    wakers: Rc<HashMap<i64, Waker>>,
}

pub async fn spawn_blocking<F, T>(f: F) -> T
where
    F: FnOnce() -> T + Send,
    T: Send,
{
    let pool = current_unwrap("spawn_blocking").pool;
    todo!()
}
