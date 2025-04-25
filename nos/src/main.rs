use anyhow::Result;
use clap::Parser;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use nos::Args;

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    nos::run(args)
}
