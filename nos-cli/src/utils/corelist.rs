//! Parse the command-line core list.

/// Parse the core list.
///
/// Core list is a comma-separated list of cores or core ranges.
/// A core range is a pair of cores separated by a hyphen.
/// For example, `0-3,5,7-9` is a core list that includes cores 0, 1, 2, 3, 5, 7, 8, and 9.
pub fn parse_core_list(str: impl AsRef<str>) -> anyhow::Result<Vec<usize>> {
    let corelist = str.as_ref();
    let mut list = Vec::new();

    for part in corelist.split(',') {
        let range: Vec<&str> = part.split('-').collect();

        match range.len() {
            1 => {
                list.push(range[0].parse()?);
            }
            2 => {
                let start = range[0].parse()?;
                let end = range[1].parse()?;

                if start > end {
                    return Err(anyhow::anyhow!("invalid core range: {}", part));
                }

                for core in start..=end {
                    list.push(core);
                }
            }
            _ => {
                return Err(anyhow::anyhow!("invalid core range: {}", part));
            }
        }
    }

    Ok(list)
}
