//! Translates a cluster configuration file into a comma-separated list of
//! peer IP addresses (for use with `pdsh`).

use std::error::Error;
use std::net::IpAddr;
use std::ops::Range;

use clap::Parser;
use nos::ctrl::{Cluster, Role};

/// ClusterConfig-to-IPAddrList parser.
#[derive(Parser, Debug)]
#[command(author, about, long_about = None)]
pub struct Args {
    /// Deduplicate the nodes
    #[clap(short, long)]
    pub no_dup: bool,

    /// Only print the number of nodes.
    #[clap(short, long)]
    pub count: bool,

    /// Select the nodes to print.
    #[clap(short, long, default_value = None)]
    pub selection: Option<String>,

    /// Path to the configuration file.
    pub conf_file: String,
}

/// Parse the node selection string.
///
/// The selection string is a comma-separated list of indices or ranges.
/// Ranges are in the Rust format, i.e., `a..b`, `a..`, and `..b`.
fn parse_selection(s: &str) -> Vec<Range<usize>> {
    let mut list = Vec::new();

    for part in s.split(',') {
        let range: Vec<&str> = part.split("..").collect();
        match range.len() {
            1 => {
                let idx = range[0].parse().unwrap();
                list.push(idx..(idx + 1));
            }
            2 => {
                let start = if range[0].is_empty() {
                    0
                } else {
                    range[0].parse().unwrap()
                };

                let end = if range[1].is_empty() {
                    usize::MAX
                } else {
                    range[1].parse().unwrap()
                };
                list.push(start..end);
            }
            _ => {
                panic!("invalid range: {}", part);
            }
        }
    }
    list
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let cluster = Cluster::load_toml_file(&args.conf_file, Role::Client)?;
    let mut peers = cluster
        .peers()
        .iter()
        .map(|p| p.ip().to_string())
        .collect::<Vec<_>>();

    if args.no_dup {
        peers.sort_by_cached_key(|p| p.parse::<IpAddr>().unwrap());
        peers.dedup();
    }

    if let Some(ref selection) = args.selection {
        let ranges = parse_selection(selection);
        peers = peers
            .into_iter()
            .enumerate()
            .filter(|(i, _)| ranges.iter().any(|r| r.contains(i)))
            .map(|(_, p)| p)
            .collect();
    }

    if args.count {
        println!("{}", peers.len());
    } else {
        println!("{}", peers.join(","));
    }

    Ok(())
}
