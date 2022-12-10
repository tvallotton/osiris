// use osiris::runtime::{block_on, Config};
// use osiris::task::yield_now;
// use tokio::join;

// const TASKS: usize = 10000;
// const WORK: usize = 100;

// async fn async_work() {
//     for _ in 0..WORK {
//         yield_now().await;
//     }
// }

// #[test]
// fn bench_osiris() {
//     use osiris::task::spawn;
//     let time = std::time::Instant::now();
//     let mut config = Config::default();
//     config.init_capacity = TASKS * 16;
//     let rt = config.build().unwrap();
//     rt.block_on(async {
//         let mut tasks = vec![];
//         for _ in 0..TASKS {
//             tasks.push(spawn(async { join!(async_work(), async_work()) }));
//             println!("osiris");
//             yield_now().await;
//         }
//         for task in tasks {
//             task.await;
//         }
//     })
//     .unwrap();
//     println!("osiris: {:?}", time.elapsed());
// }

// #[test]
// fn bench_tokio() {
//     use tokio::runtime::Builder;
//     use tokio::spawn;
//     let time = std::time::Instant::now();
//     Builder::new_multi_thread()
//         .build()
//         .unwrap()
//         .block_on(async {
//             let mut tasks = vec![];
//             for _ in 0..TASKS {
//                 tasks.push(spawn(async { join!(async_work(), async_work()) }));
//                 yield_now().await;
//             }
//             for task in tasks {
//                 task.await.ok();
//             }
//         });

//     println!("tokio: {:?}", time.elapsed());
// }
