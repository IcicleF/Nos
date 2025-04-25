use std::path::{Path, PathBuf};

/// Get hostname.
pub fn get_hostname() -> String {
    let hostname = hostname::get().unwrap();
    let hostname = hostname.into_string().unwrap();
    hostname.split('.').next().unwrap().to_owned()
}

/// Append hostname to the end of the filename.
pub fn append_hostname(filename: &str) -> PathBuf {
    let path = Path::new(filename);
    let mut path = path.to_path_buf();

    let filename = path
        .file_name()
        .unwrap()
        .to_os_string()
        .into_string()
        .unwrap();
    let segs = filename.split('.').collect::<Vec<_>>();
    let filename = if segs.len() > 1 {
        let mut filename = segs[..segs.len() - 1].join(".");
        filename.push_str(&format!("-{}.{}", get_hostname(), segs[segs.len() - 1]));
        filename
    } else {
        format!("{}-{}", filename, get_hostname())
    };
    path.set_file_name(filename);

    path
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This test is only valid on CloudLab with the given profile.
    #[test]
    fn on_cloudlab() {
        let hn = get_hostname();
        assert!(hn.starts_with("node"));
        assert!(hn.len() <= 6);
    }
}
