use io_uring::types::Fd;

mod open_options;

pub struct File {
    fd: Fd,
}
