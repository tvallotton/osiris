use std::{io::Result, path::Path};

use crate::reactor::op;

use super::cstr;

pub async fn symlink(original: impl AsRef<Path>, link: impl AsRef<Path>) -> Result<()> {
    let original = cstr(original.as_ref())?;
    let link = cstr(link.as_ref())?;
    op::symlink(original, link).await
}
