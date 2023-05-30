use std::{net::IpAddr, rc::Rc};

/// Structural form of `resolv.conf`.
#[derive(Clone, Debug)]
pub struct ResolvConf {
    /// The list of name servers.
    name_servers: Vec<IpAddr>,

    /// Maximum number of segments in the domain name.
    ndots: u8,

    /// Maximum timeout in seconds.
    timeout: u8,

    /// Maximum number of retries.
    attempts: u8,

    /// The search domain to use.
    search: Option<String>,
}

impl Default for ResolvConf {
    fn default() -> Self {
        ResolvConf {
            name_servers: vec![],
            ndots: 1,
            timeout: 5,
            attempts: 2,
            search: None,
        }
    }
}

impl ResolvConf {
    pub fn load() -> Rc<Self> {
        thread_local! {
            static CONF: Rc<ResolvConf> = {
                let mut conf = ResolvConf::default();
                conf.load_from_file();
                Rc::new(conf)
            };
        }
        CONF.with(Rc::clone)
    }

    fn load_from_file(&mut self) {
        // we do this load synchronously to avoid mutexes on the thread local.
        // it should be fine since we only do this once.
        let Ok(conf) = std::fs::read_to_string("/etc/resolv.conf") else { return; };
        for mut line in conf.lines() {
            if let Some(cmmt) = line.find('#') {
                line = &line[..cmmt];
            }

            let mut columns = line.split_ascii_whitespace();
            let Some(key) = columns.next() else { continue };
            let Some(value) = columns.next() else { continue };
            println!("{key:?}");

            match key {
                "search" => {
                    self.search = Some(value.into());
                }
                "nameserver" => {
                    let Ok(ip) =  value.parse() else { continue };
                    self.name_servers.push(ip);
                }
                "options" => {
                    if let Some(ndots) = value.strip_prefix("ndots:") {
                        let Ok(ndots) = ndots.parse() else { continue };
                        self.ndots = ndots;
                    }
                    if let Some(timeout) = value.strip_prefix("timeout:") {
                        let Ok(timeout) = timeout.parse() else { continue };
                        self.timeout = timeout;
                    }

                    if let Some(ndots) = value.strip_prefix("attempts:") {
                        let Ok(ndots) = ndots.parse() else { continue };
                        self.ndots = ndots;
                    }
                }
                _ => continue,
            }
        }
    }
}
