use std::net::{Shutdown, SocketAddr};
use std::rc::Rc;
use std::time::Duration;

use crate::fs::{File, OpenOptions};
use crate::net::{TcpStream, UdpSocket};
use crate::task::JoinHandle;
use crate::time::{sleep, timeout};
use crate::utils::futures::not_thread_safe;

macro_rules! assert_not_impl {
    ($e:expr, $($t:path),+ $(,)*) => {{
        let x = $e;
        let _ = move || -> () {
            struct Check<T: ?Sized>(T);
            trait AmbiguousIfImpl<A> { fn some_item(&self) { } }

            impl<T: ?Sized> AmbiguousIfImpl<()> for Check<T> { }
            impl<T: ?Sized $(+ $t)*> AmbiguousIfImpl<u8> for Check<T> { }

            Check(x).some_item()
        };
  }  };
}

fn join_handle(jh: JoinHandle<()>, jh2: JoinHandle<()>) {
    assert_not_impl!(jh, Send, Sync);
    assert_not_impl!(jh2.catch_unwind(), Send, Sync);
}

fn timeout_assertions(dur: Duration) {
    assert_not_impl!(sleep(dur), Send, Sync);
    assert_not_impl!(timeout(dur, async {}), Send, Sync);
}

fn file_assertions(mut file: File, b: Vec<u8>) {
    assert_not_impl!(file.read_at(b.clone(), 0), Send, Sync);
    assert_not_impl!(file.write_at(b.clone(), 0), Send, Sync);
    assert_not_impl!(file.read(b.clone()), Send, Sync);
    assert_not_impl!(file.write(b.clone()), Send, Sync);
    assert_not_impl!(file.sync_all(), Send, Sync);
    assert_not_impl!(file.sync_data(), Send, Sync);
    assert_not_impl!(file.metadata(), Send, Sync);
    assert_not_impl!(file.close(), Send, Sync);

    let options = OpenOptions::new();
    assert_not_impl!(options.open(""), Send, Sync);
}

fn udpsocket_assertions(mut socket: UdpSocket, b: Vec<u8>, addr: SocketAddr) {
    assert_not_impl!(socket.connect(addr), Send, Sync);
    assert_not_impl!(socket.read(b.clone()), Send, Sync);
    assert_not_impl!(socket.write(b.clone()), Send, Sync);
    assert_not_impl!(socket.send_to(b.clone(), addr), Send, Sync);
    assert_not_impl!(socket.recv(b.clone()), Send, Sync);
}

fn tcpstream_assertions(mut socket: TcpStream, b: Vec<u8>, s: Shutdown) {
    assert_not_impl!(socket.read(b.clone()), Send, Sync);
    assert_not_impl!(socket.write(b.clone()), Send, Sync);
    assert_not_impl!(socket.write_all(b.clone()), Send, Sync);
    assert_not_impl!(socket.shutdown(s), Send, Sync);
    assert_not_impl!(socket.close(), Send, Sync);
}
