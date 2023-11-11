use std::io::ErrorKind;

use osiris::net::{TcpListener, TcpStream};
use osiris::spawn;

#[osiris::test]
async fn connection_refused() {
    let err = TcpStream::connect("127.0.0.1:10000").await.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::ConnectionRefused, "{err}");
}

#[osiris::test]
async fn connection_successful() {
    let listener = TcpListener::bind("127.0.0.1:7000").await.unwrap();
    let task = spawn(async {
        TcpStream::connect("127.0.0.1:7000").await.unwrap();
    });
    listener.accept().await.unwrap();
    task.await;
}
