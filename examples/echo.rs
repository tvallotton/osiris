use osiris::buf::IoBuf;
use osiris::detach;
use osiris::net::{TcpListener, TcpStream};
use osiris::task::yield_now;
use std::io::Result;

async fn handle_client(stream: TcpStream) -> Result<()> {
    let buf = vec![0; 1048];
    let (n, buf) = stream.read(buf).await;
    let buf = buf.slice(..n?);
    stream.write_all(buf).await.0?;
    stream.close().await
}
const N: usize = 1000;

#[osiris::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let time = std::time::Instant::now();
    // detach(async {
    //     loop {
    //         osiris::task::yield_now().await;
    //     }
    // });
    // run server
    detach(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            detach(handle_client(stream));
        }
    });
    let mut clients = vec![];
    for _ in 0..N {
        let client = detach(async move {
            //osiris::time::sleep(std::time::Duration::from_secs(2));
            let stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
            let msg = format!("the code is: {}", fastrand::u128(..));
            stream.write_all(msg.clone().into_bytes()).await.0.unwrap();
            let buf = vec![0; 2048];

            let (n, buf) = stream.read(buf).await;
            let buf = buf.slice(0..n.unwrap());
            assert_eq!(std::str::from_utf8(&buf).unwrap(), msg);
            // println!("{i}: {:?}", time.elapsed());
        });
        clients.push(client);
        osiris::task::yield_now().await;
    }
    yield_now().await;
    for client in clients {
        client.await;
    }

    Ok(())
}
