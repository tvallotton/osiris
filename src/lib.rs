#![warn(clippy::undocumented_unsafe_blocks)]

use crate::runtime::{current, current_unwrap};
pub use task::spawn;
#[macro_use]
mod macros;
pub mod runtime;
mod task;

#[test]
fn smoke_test() {
    fn int_to_ptr(n: usize) -> *const () {
        let p: *const u8 = core::ptr::null();
        p.wrapping_add(n) as *const ()
    }

    fn ptr_to_int(n: *const ()) -> usize {
        n as usize
    }
    
    

    let rt = runtime::Runtime::new();
    rt.block_on(async {
        println!("1.1");

        let handle = spawn(async {
            println!("2.1");
            task::yield_now().await;
            println!("2.2");
        });
        task::yield_now().await;
        println!("1.2");
        task::yield_now().await;
        println!("1.3");
        task::yield_now().await;
        println!("1.4");
    })
}
