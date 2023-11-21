use std::io::Result;
use std::path::PathBuf;

use crate::reactor::op;

use super::cstr;

pub async fn symlink(original: impl Into<PathBuf>, link: impl Into<PathBuf>) -> Result<()> {
    let original = cstr(original)?;
    let link = cstr(link)?;
    op::symlink(original, link).await
}
