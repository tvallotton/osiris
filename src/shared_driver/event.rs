use super::SharedDriver;

pub struct Event<T> {
    driver: SharedDriver,
    data: T,
    id: u64,
}
