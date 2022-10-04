#![feature(generic_associated_types)]


pub use runtime::block_on;
pub use task::spawn;

#[macro_use]
mod macros;
pub mod runtime;
pub mod task;
pub mod io; 

#[test]
fn smoke_test() {
foo(); 

    block_on(async {
        let task = task::spawn(async {
            println!("hello from task");
        });

        task.await;
        println!("hello from entry point");
    })
}

fn foo() {
    task::spawn(async {}); 

}