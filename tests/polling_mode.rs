use osiris::join;
use osiris::net::{TcpListener, TcpStream};
use osiris::runtime::{Config, Mode};

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
    let stream = TcpStream::connect("localhost:9000").await.unwrap();

    stream.write(CLIENT_MSG).await.0.unwrap();

    let buf = vec![0; 20];
    let (written, buf) = stream.read(buf).await;
    let size = written.unwrap();
    assert_eq!(&buf[..size], SERVER_MSG);
}

async fn server() {
    let listener = TcpListener::bind("localhost:9000").await.unwrap();
    let (client, _) = listener.accept().await.unwrap();
    let buf = vec![0; 20];

    let (written, buf) = client.read(buf).await;
    let size = written.unwrap();
    assert_eq!(&buf[..size], CLIENT_MSG);

    client.write(SERVER_MSG).await.0.unwrap();
    client.close().await.unwrap();
}
