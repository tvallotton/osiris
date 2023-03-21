use std::collections::HashMap;
use std::task::Waker;

pub struct ThreadPoolHandle {
    wakers: HashMap<Waker>,
}
