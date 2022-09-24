#![warn(clippy::undocumented_unsafe_blocks)]


#[macro_use]
mod macros;
pub mod runtime;
mod task;




#[test]
fn smoke_test() {


    let rt = runtime::Runtime::new(); 
    rt.block_on(async {
        println!("asd"); 
        task::yield_now().await; 
        task::yield_now().await; 
    })

}