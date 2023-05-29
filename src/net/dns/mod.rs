use std::io::Result;
use std::net::IpAddr;

mod unix;

/// Preform a DNS lookup, retrieving the IP addresses and other necessary information.
pub async fn lookup(name: &str) -> Result<impl Iterator<Item = IpAddr>> {
    // Try to parse the name as an IP address.
    if let Ok(ip) = name.parse::<IpAddr>() {
        return Ok(Either::Left(Some(ip).into_iter()));
    }

    Ok(Either::Right(None.into_iter()))
}

enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> Iterator for Either<L, R>
where
    L: Iterator,
    R: Iterator<Item = L::Item>,
{
    type Item = L::Item;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Either::Left(l) => l.next(),
            Either::Right(r) => r.next(),
        }
    }
}
