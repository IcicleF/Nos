use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use anyhow::Result;

/// Thread number configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NThreads {
    /// Number of RPC server (foreground) threads.
    pub rpc: usize,

    /// Number of encoder (background) threads.
    pub ec: usize,
}

impl NThreads {
    /// Default number of RPC server (foreground) threads.
    pub const DEFAULT_RPC_THREADS: usize = 4;

    /// Default number of encoder (background) threads.
    pub const DEFAULT_EC_THREADS: usize = 1;

    /// Create a new thread number configuration.
    pub fn new(rpc: usize, ec: usize) -> Self {
        Self { rpc, ec }
    }

    /// Load TOML thread number configuration.
    ///
    /// All the configuration entries should be placed in the `nthreads` table.
    /// Irrelevant fields will be ignored, and you can put the configuration
    /// in your own mixed TOML configuration.
    ///
    /// The `nthreads` table can contain the following fields:
    ///
    /// - `rpc`: number of RPC server (foreground) threads (default to 4).
    /// - `ec`: number of encoder (background) threads (default to 1).
    /// - `gc`: number of garbage collector (background) threads (default to 1).
    ///
    /// See the following example:
    ///
    /// ```rust
    /// # use nos::ctrl::NThreads;
    /// let toml = "";
    /// assert_eq!(NThreads::load_toml(toml).unwrap(), NThreads::default());
    ///
    /// let toml = r#"
    ///     [nthreads]
    ///     rpc = 8
    ///     ec = 4
    /// "#;
    /// assert_eq!(NThreads::load_toml(toml).unwrap(), NThreads::new(8, 4));
    /// ```
    pub fn load_toml(toml: &str) -> Result<Self> {
        let toml = toml::from_str::<toml::Value>(toml)?;
        let conf = toml.get("nthreads");

        match conf {
            Some(conf) => {
                let conf = conf.as_table().ok_or_else(|| {
                    anyhow::anyhow!("bad configuration: thread number config must be a table")
                })?;
                let rpc_threads = conf
                    .get("rpc")
                    .map(|v| {
                        v.as_integer().map(|x| x as usize).ok_or_else(|| {
                            anyhow::anyhow!(
                                "bad configuration: number of RPC threads must be an integer"
                            )
                        })
                    })
                    .unwrap_or(Ok(Self::DEFAULT_RPC_THREADS))?;

                let ec_threads = conf
                    .get("ec")
                    .map(|v| {
                        v.as_integer().map(|x| x as usize).ok_or_else(|| {
                            anyhow::anyhow!(
                                "bad configuration: number of encoder threads must be an integer"
                            )
                        })
                    })
                    .unwrap_or(Ok(Self::DEFAULT_EC_THREADS))?;

                Ok(Self::new(rpc_threads, ec_threads))
            }
            None => Ok(Self::default()),
        }
    }

    /// Load cluster configuration from a TOML file.
    pub fn load_toml_file(path: impl AsRef<Path>) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut toml_str = String::new();
        file.read_to_string(&mut toml_str)?;

        Self::load_toml(&toml_str)
    }
}

impl Default for NThreads {
    fn default() -> Self {
        Self::new(Self::DEFAULT_RPC_THREADS, Self::DEFAULT_EC_THREADS)
    }
}
