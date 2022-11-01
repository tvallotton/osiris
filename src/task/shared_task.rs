use super::Task;
use std::pin::Pin;
use std::rc::Rc;

struct SharedTask {
    task: Pin<Rc<dyn Task>>,
}

impl Drop for SharedTask {
    fn drop(&mut self) {
        self.task.as_ref().abort();
    }
}
