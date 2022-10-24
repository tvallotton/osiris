use std::marker::PhantomData;

pub struct Event<T> {
    _pd: PhantomData<T>,
}
