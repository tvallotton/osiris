use cfg_aliases::cfg_aliases;

fn main() {
    // Setup cfg aliases
    cfg_aliases! {
        io_uring: {
            all(feature = "io-uring", target_os="linux")
        },
        kqueue: {
            any(
                target_os = "macos",
                target_os = "freebsd",
                target_os = "dragonfly",
                target_os = "openbsd",
                target_os = "netbsd",
                target_os = "ios"
            )
        }
    }
}
