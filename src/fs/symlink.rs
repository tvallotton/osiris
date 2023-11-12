use std::io::Result;
use std::path::{Path, PathBuf};

use crate::reactor::op;

use super::cstr;

pub async fn symlink(original: impl AsRef<Path>, link: impl AsRef<PathBuf>) -> Result<()> {
    let original = cstr(original.as_ref())?;
    let link = cstr(link.as_ref())?;
    op::symlink(original, link).await
}
