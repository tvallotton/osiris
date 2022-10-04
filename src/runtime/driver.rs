use io_uring::{Builder, IoUring};

use std::io; 

const DEFAULT_ENTRIES : u32 = 2048; 

pub(crate) struct Driver {
    io_uring: IoUring, 
}


impl Driver {
    pub fn new() -> io::Result<Driver> {
        let io_uring = io_uring::IoUring::builder().build(DEFAULT_ENTRIES)?; 
        let driver = Driver { io_uring }; 
        Ok(driver)
    }
}
