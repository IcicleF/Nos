use anyhow::Result;
use clap::Parser;

use nos_cli::Args;

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    nos_cli::run(args)
}
