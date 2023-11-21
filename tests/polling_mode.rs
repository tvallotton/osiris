use std::time::Duration;

use osiris::join;
use osiris::net::{TcpListener, TcpStream};
use osiris::runtime::{Config, Mode};
use osiris::time::sleep;

const CLIENT_MSG: &[u8] = b"client message";
const SERVER_MSG: &[u8] = b"server message";

#[test]
fn polling_mode() {
    let config = Config {
        mode: Mode::Polling { idle_timeout: 25 },
        ..Config::default()
    };

    config
        .build()
        .unwrap()
        .block_on(async {
            join!(server(), client());
        })
        .unwrap();
}

async fn client() {
    sleep(Duration::from_millis(10)).await;
    let mut stream = TcpStream::connect("localhost:9080").await.unwrap();

    stream.write(CLIENT_MSG).await.0.unwrap();

    let buf = vec![0; 20];
    let (written, buf) = stream.read(buf).await;
    let size = written.unwrap();
    assert_eq!(&buf[..size], SERVER_MSG);
}

async fn server() {
    let listener = TcpListener::bind("127.0.0.1:9080").await.unwrap();
    let (mut client, _) = listener.accept().await.unwrap();
    let buf = vec![0; 20];

    let (written, buf) = client.read(buf).await;
    let size = written.unwrap();
    assert_eq!(&buf[..size], CLIENT_MSG);

    client.write(SERVER_MSG).await.0.unwrap();
    client.close().await.unwrap();
}
