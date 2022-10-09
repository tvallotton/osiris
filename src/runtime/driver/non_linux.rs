use std::io::Result;

use crate::runtime::config::Config; 
pub struct Driver;

impl Driver {
    pub fn new(_config: Config) -> Result<Driver> {
        Ok(Driver)
    }
}
