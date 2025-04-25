//! Prepare string parser.

use anyhow::{anyhow, Result};

/// Preparation specifier.
#[derive(Debug, Clone, Copy)]
pub struct Prepare {
    pub i: usize,
    pub n: usize,
}

/// Parse a prepare string.
pub fn parse_prepare(str: impl AsRef<str>) -> Result<Option<Prepare>> {
    let str = str.as_ref();
    if str.is_empty() {
        return Ok(None);
    }

    let parts = str.split('/').collect::<Vec<_>>();
    if parts.len() != 2 {
        return Err(anyhow!("invalid prepare string: {}", str));
    }
    let i = parts[0].parse::<usize>()?;
    let n = parts[1].parse::<usize>()?;
    if n == 0 {
        return Err(anyhow!("n cannot be 0: {}", str));
    }
    if i >= n {
        return Err(anyhow!("i must be less than n: {}", str));
    }
    Ok(Some(Prepare { i, n }))
}
