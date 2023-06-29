use osiris::buf::IoBuf;
use osiris::detach;
use osiris::net::{TcpListener, TcpStream};
use osiris::task::{yield_now, JoinHandle};
use std::io::Result;
use std::mem::transmute;

async fn handle_client(stream: TcpStream) -> Result<()> {
    println!("{:?}", std::thread::current().id());
    let buf = vec![0; 1048];
    let (n, buf) = stream.read(buf).await;
    let buf = buf.slice(..n?);
    stream.write_all(buf).await.0?;
    stream.close().await
}

#[osiris::main(scale = true)]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    // run server
    detach(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            detach(handle_client(stream));
        }
    });

    spawn_clients(1000).await;
    Ok(())
}

fn spawn_clients(n: u32) -> JoinHandle<()> {
    detach(async move {
        let mut clients = vec![];
        for i in 0..n {
            let client = detach(run_client(i));
            clients.push(client);
            yield_now().await;
        }
        for client in clients {
            client.await;
        }
    })
}

async fn run_client(id: u32) {
    let stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
    let msg = format!("the code is: {}", fastrand::u128(..));
    stream.write_all(msg.clone().into_bytes()).await.0.unwrap();
    let buf = vec![0; 2048];

    let (n, buf) = stream.read(buf).await;
    let buf = buf.slice(0..n.unwrap());
    assert_eq!(std::str::from_utf8(&buf).unwrap(), msg);
    // dbg!(id);
}
