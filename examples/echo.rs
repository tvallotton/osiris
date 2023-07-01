use osiris::buf::IoBuf;
use osiris::detach;
use osiris::net::{TcpListener, TcpStream};
use osiris::task::{yield_now, JoinHandle};
use osiris::time::{sleep, timeout};
use std::io::Result;
use std::mem::transmute;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

async fn handle_client(stream: TcpStream, client: SocketAddr) -> Result<()> {
    let buf = vec![0; 1048];

    println!(
        "server read    : {client:?} {:?}",
        std::thread::current().id()
    );
    let (n, buf) = stream.read(buf).await;
    let buf = buf.slice(..n?);
    println!(
        "server write   : {client:?} {:?}",
        std::thread::current().id()
    );
    stream.write_all(buf).await.0?;
    println!(
        "server close   : {client:?} {:?}",
        std::thread::current().id()
    );
    stream.close().await?;
    println!(
        "server exit    : {client:?} {:?}",
        std::thread::current().id()
    );
    Ok(())
}

#[osiris::main(scale = 8)]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    // run server
    detach(async move {
        loop {
            let (stream, client) = listener.accept().await.unwrap();
            println!("server accept  : {client}");
            detach(async move { handle_client(stream, client).await.unwrap() });
        }
    });

    spawn_clients(1).await;
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
    println!("client connect : {:?}", std::thread::current().id());
    let stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
    let msg = format!("the code is: {}", fastrand::u128(..));
    println!("client write   : {:?}", std::thread::current().id());
    stream.write_all(msg.clone().into_bytes()).await.0.unwrap();

    let buf = vec![0; 2048];
    println!("client read    : {:?}", std::thread::current().id());

    let (n, buf) = timeout(Duration::from_secs(30), stream.read(buf))
        .await
        .unwrap();

    let buf = buf.slice(0..n.unwrap());
    assert_eq!(std::str::from_utf8(&buf).unwrap(), msg);
    println!("client close   : {:?}", std::thread::current().id());
    stream.close().await.unwrap();
    // dbg!(id);
}
