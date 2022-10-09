use std::hint::unreachable_unchecked;

use osiris::runtime::block_on;
use osiris::spawn;
use osiris::task::yield_now;

#[test]
fn bar() {
    use std::cell::Cell;
    use std::rc::Rc;
    // let start = std::time::Instant::now();
    let cell = Rc::new(Cell::new(0));
    block_on(async {
        for _ in 0..100 {
            spawn({
                let cell = cell.clone();
                async move {
                    let mut tasks = vec![];
                    for i in 0..100 {
                        let task = spawn({
                            let cell = cell.clone();
                            async move {
                                cell.set(cell.get() + 1);
                                yield_now().await;
                                cell.set(cell.get() + 1);
                                yield_now().await;
                                cell.set(cell.get() + 1);
                                yield_now().await;
                                cell.set(cell.get() / 1);
                            }
                        });
                        tasks.push(task);
                    }
                    for task in tasks {
                        task.await;
                        panic!("asd");
                    }
                }
            });
            yield_now().await;
        }
        yield_now().await;
        yield_now().await;
        yield_now().await;
    })
    .unwrap();
    println!("{cell:?}");
    // println!("{:?}", start.elapsed());
}

// #[test]
// fn hello_world() {
//     block_on(async {
//         osiris::task::yield_now().await;
//         println!("hello world");
//     })
//     .unwrap();
// }
