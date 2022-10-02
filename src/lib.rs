#![warn(clippy::undocumented_unsafe_blocks)]

pub use runtime::block_on;
pub use task::spawn;

#[macro_use]
mod macros;
pub mod runtime;
pub mod task;

#[test]
fn smoke_test() {
    block_on(async {
        let task = task::spawn(async {
            println!("hello from task");
        });

        task.await;
        println!("hello from entry point");
    })
}
