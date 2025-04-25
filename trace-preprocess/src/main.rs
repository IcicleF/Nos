use std::fs::{File, OpenOptions};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{prelude::*, BufReader, BufWriter};

use clap::Parser;

/// Preprocessor for Twemcache trace files.
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Input trace filename base.
    #[clap(short, long)]
    pub input: String,

    /// Output filename base.
    #[clap(short, long)]
    pub output: String,

    /// Number of partitions.
    #[clap(short, long)]
    pub num_partitions: usize,
}

fn main() {
    let args = Args::parse();
    println!("Processing trace file: {}", args.input);

    let mut oufs = Vec::with_capacity(args.num_partitions);
    for i in 0..args.num_partitions {
        let filename = format!("{}-{}", args.output, i);

        // Create or append.
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&filename)
            .unwrap();
        oufs.push(BufWriter::new(file));
    }

    let inf = BufReader::new(File::open(args.input).unwrap());
    for (i, line) in inf.lines().enumerate() {
        let line = line.unwrap();
        let parts = line.split(',').collect::<Vec<_>>();

        let key = {
            let mut hasher = DefaultHasher::new();
            parts[1].hash(&mut hasher);
            hasher.finish() as u32
        };
        let len = parts[2].parse::<usize>().unwrap() + parts[3].parse::<usize>().unwrap();
        let ty = if parts[5] == "get" { 0u8 } else { 1u8 };

        let ouf = &mut oufs[i % args.num_partitions];
        writeln!(ouf, "{} {} {}", ty, key, len).unwrap();

        if (i + 1) % 1000000 == 0 {
            println!("Processed {} lines", i + 1);
            for ouf in oufs.iter_mut() {
                ouf.flush().unwrap();
            }
        }
    }
}
