use std::{collections::HashMap, future::Future, rc::Rc};

pub(crate) struct Executor {
    tasks: HashMap<usize, Rc<dyn Task>>,
    task_id: usize,
}

trait Task {}

impl Executor {
    pub fn new() -> Executor {
        Executor {
            tasks: HashMap::new(),
            task_id: 0,
        }
    }

    pub fn block_on<F>(&mut self) -> F::Output
    where
        F: Future,
    {
        todo!()
    }
}
