#[cfg(target_os = "linux")]
use osiris::{
    block_on,
    fs::{File, OpenOptions},
};

#[cfg(target_os = "linux")]
#[test]
fn open_file() {
    use std::io;

    use osiris::spawn;
    let time = std::time::Instant::now();
    block_on(async {
        let mut handles = vec![];
        for i in 0..1000 {
            let filename = format!("files/file_{i}.txt");
            let h = spawn(async move { File::create(filename).await });
            handles.push(h);
        }

        for handle in handles {
            handle.await?;
        }

        io::Result::Ok(())
    })
    .unwrap()
    .unwrap();
    println!("{:?}", time.elapsed());
}
