
#![forbid(undocumented_unsafe_blocks)]

pub use runtime::block_on;
pub use task::spawn;

#[macro_use]
mod macros;
pub mod io;
pub mod runtime;
pub mod task;

#[test]
fn foo() {
    use std::cell::Cell;
    use std::rc::Rc;
    // let start = std::time::Instant::now();
    let cell = Rc::new(Cell::new(0u64));
    block_on(async {
        for i in 0..100 {
            spawn({
                let cell = cell.clone();
                async move {
                    task::yield_now().await;
                    cell.set(cell.get() + 1);
                    task::yield_now().await;
                    cell.set(cell.get() + 1);
                    task::yield_now().await;
                }
            });
            task::yield_now().await;
        }
    })
    .unwrap();
    println!("{cell:?}");
    // println!("{:?}", start.elapsed());
}
