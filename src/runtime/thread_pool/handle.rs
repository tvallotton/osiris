use crate::runtime::current_unwrap;

use super::lock_thread_pool;
use super::work::WorkResult;
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::{poll_fn, Future};
use std::panic::resume_unwind;
use std::rc::Rc;
use std::task::{Waker, Poll};

#[derive(Clone, Default)]
pub struct ThreadPoolHandle {
    wakers: Rc<RefCell<HashMap<u32, Waker>>>,
}

struct Handle<'a> {
    pool: &'a ThreadPoolHandle,
    id: u32,
}

impl<'a> Drop for Handle<'a> {
    fn drop(&mut self) {
        self.pool.wakers.borrow_mut().remove(&self.id);
        lock_thread_pool().results.remove(&self.id);
    }
}


pub fn spawn_blocking<F, T>(f: F) -> impl Future<Output = T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let rt = current_unwrap("spawn_blocking");
    let mut handle =None;
    let mut work = Some(f); 
    poll_fn(move |cx| {
        if let Some(work) = work.take() {
            handle = Some(rt.pool.push_work(work, cx.waker())); 
            return Poll::Pending; 
        }

        let Some(handle) = &mut handle else {
             unreachable!()
        };

        let Some(result) = lock_thread_pool().results.remove(&handle.id) else  {
            rt.pool.wakers.borrow_mut().insert(handle.id, cx.waker().clone()); 
            return Poll::Pending; 
        }; 

        match result.res {
            Ok(v) => Poll::Ready(*v.downcast().unwrap()),
            Err(e) => resume_unwind(e),
        }
    })
}
impl ThreadPoolHandle {
    fn push_work<F, T>(&self, f: F, waker: &Waker) -> Handle
    where    
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static 
    {
        
        let pool = lock_thread_pool(); 
        pool.id += 1; 
        let id = pool.id; 
        drop(pool); 
        self.wakers.borrow_mut().insert(id, waker.clone()); 
        Handle {
            pool: self, 
            id, 
        }
    }

    
}