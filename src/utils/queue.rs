use std::collections::VecDeque;
use std::marker::PhantomData;
use std::task::Waker;

struct WakerQueue {
    queue: VecDeque<Waker>,
    id: u64, 
}

struct Handle<'a> {
    queue: &'a WakerQueue,
    id: u64,
}



impl WakerQueue<false> {


    pub fn push(&mut self, waker: Waker) - {

    }
}


