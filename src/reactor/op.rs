use std::{ffi::CString, io::Result, net::SocketAddr, mem::zeroed};

use io_uring::{
    opcode::{Close, Fsync, OpenAt, Read, Recv, Socket, Statx, UnlinkAt, Write, Connect, SendMsg},
    types::{Fd, FsyncFlags},
};
use libc::{AT_FDCWD, iovec, msghdr};

use crate::{buf::{IoBuf, IoBufMut}, net::utils::socket_addr};

use super::submit;

/// Attempts to close a file descriptor
pub async fn close(fd: i32) -> Result<()> {
    let sqe = Close::new(Fd(fd)).build();
    unsafe { submit(sqe, ()) }.await.0.map(|_| ())
}

/// Attempts to read from a file descriptor into the buffer
pub async fn read_at<B: IoBufMut>(fd: i32, mut buf: B, pos: i64) -> (Result<usize>, B) {
    let sqe = Read::new(Fd(fd), buf.stable_mut_ptr(), buf.bytes_total() as _)
        .offset64(pos)
        .build();
    let (cqe, mut buf) = unsafe { submit(sqe, buf).await };
    
    let Ok(cqe) = cqe else {
        return (cqe.map(|_| unreachable!()), buf); 
    }; 
    let len = cqe.result() as usize; 
    
    // initialized by io-uring
    unsafe{ buf.set_init(len) }; 
    
    (Ok(len), buf)
}

/// Attempts to write to a file descriptor
pub async fn write_at<B: IoBuf>(fd: i32, buf: B, pos: i64) -> (Result<usize>, B) {
    let sqe = Write::new(Fd(fd), buf.stable_ptr(), buf.bytes_init() as _)
        .offset64(pos)
        .build();
    let (cqe, buf) = unsafe { submit(sqe, buf).await };
    (cqe.map(|cqe| cqe.result() as usize), buf)
}

/// Performs an fsync call
pub async fn fsync(fd: i32, flags: FsyncFlags) -> Result<i32> {
    let sqe = Fsync::new(Fd(fd)).flags(flags).build();
    // Safety: no resource tracking needed
    let res = unsafe { submit(sqe, ()).await.0?.result() };
    Ok(res)
}
/// removes a file
pub async fn unlink_at(path: CString) -> Result<i32> {
    let sqe = UnlinkAt::new(Fd(AT_FDCWD), path.as_ptr()).build();
    let res = unsafe { submit(sqe, path) };
    Ok(res.await.0?.result())
}
/// Creates a socket
pub async fn socket(
    domain: i32,
    ty: i32,
    proto: i32,
    file_index: Option<io_uring::types::DestinationSlot>,
) -> Result<i32> {
    let sqe = Socket::new(domain, ty, proto)
        .file_index(file_index)
        .build();
    let fut = unsafe { submit(sqe, ()) };
    let res = fut.await.0?.result();
    Ok(res)
}

pub async fn recv<B: IoBufMut>(fd: i32, mut buf: B) -> (Result<usize>, B) {
    let len = buf.bytes_total() as u32;
    let ptr = buf.stable_mut_ptr();
    let sqe = Recv::new(Fd(fd), ptr, len).build();
    let (res, buf) = unsafe { submit(sqe, buf).await };
    let res = res.map(|r| r.result() as usize);
    (res, buf)
}

/// performs a statx "system call" on a file or path
pub async fn statx(fd: i32, path: Option<CString>) -> Result<libc::statx> {
    let pathname = path.as_ref().map(|x| x.as_ptr()).unwrap_or(b"\0".as_ptr());
    let statx = std::mem::MaybeUninit::<libc::statx>::uninit();
    let mut statx = Box::new(statx);
    let sqe = Statx::new(Fd(fd), pathname, statx.as_mut_ptr().cast())
        .mask(libc::STATX_ALL)
        .flags(if path.is_none() {
            libc::AT_EMPTY_PATH
        } else {
            0
        })
        .build();
    // Safety: both resources are guarded
    let (res, (_, statx)) = unsafe { submit(sqe, (path, statx)).await };
    // Safety: initialized by io-uring
    res.map(|_| unsafe { statx.assume_init_read() })
}

pub async fn connect(fd: i32, addr: SocketAddr) -> Result<()>{
    let (addr, len) = socket_addr(&addr); 
    let addr = Box::new(addr); 
    let sqe = Connect::new(Fd(fd), addr.as_ptr().cast(), len).build(); 
    let (cqe, _) = unsafe{ submit(sqe, addr).await }; 
    cqe?; 
    Ok(())
}

pub async fn send_to<B: IoBuf>(fd: i32, buf: B, addr: SocketAddr) -> (Result<usize>, B) {
        // we define the iovec from the buffer
        let msg_iov: iovec = iovec {
            iov_base: buf.stable_ptr().cast_mut().cast(),
            iov_len: buf.bytes_init(),
        };

        let msghdr: msghdr = unsafe { zeroed() };

        let (addr, len) = socket_addr(&addr);

        // we allocate everything once
        let mut msg = Box::new((msghdr, msg_iov, addr));

        // we set the address to point to the box
        msg.0.msg_name = &mut msg.2 as *mut _ as *mut _;
        msg.0.msg_namelen = len;

        // we set the iovec to point to the box
        msg.0.msg_iov = &mut msg.1;
        msg.0.msg_iovlen = 1;

        let sqe = SendMsg::new(Fd(fd), &msg.0).build();
        let (res, (_, buf)) = unsafe { submit(sqe, (msg, buf)).await };
        let res = res.map(|sqe| sqe.result() as usize);
        (res, buf)
}

pub async fn open_at(path: CString, flags: i32, mode: u32) -> Result<i32> {
    let entry = OpenAt::new(Fd(libc::AT_FDCWD), path.as_ptr())
        .flags(flags)
        .mode(mode)
        .build();

    // Safety: the resource (pathname) is submitted
    let (cqe, _) = unsafe { submit(entry, path) }.await;
    Ok(cqe?.result())
}
