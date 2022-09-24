use super::Task;
use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;

pub struct JoinHandle<T> {
    task: Pin<Rc<dyn Task>>,
    _t: PhantomData<T>,
}

impl<T> JoinHandle<T> {
    pub(crate) fn new(task: Pin<Rc<dyn Task>>) -> JoinHandle<T> {
        JoinHandle {
            task,
            _t: PhantomData::default(),
        }
    }
}
