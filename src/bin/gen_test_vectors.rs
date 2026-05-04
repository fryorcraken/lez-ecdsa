use std::path::PathBuf;

use clap::Parser;
use lez_signature_bench::{Scheme, make_test_vector};

#[derive(Parser, Debug)]
#[command(about = "Write a deterministic JSON fixture for one (scheme, n) point.")]
struct Cli {
    #[arg(long)]
    scheme: String,
    #[arg(long)]
    n: usize,
    #[arg(long, default_value = "fixtures")]
    out_dir: PathBuf,
}

fn main() {
    let cli = Cli::parse();
    let scheme = Scheme::parse(&cli.scheme).expect("scheme");
    let v = make_test_vector(scheme, cli.n);

    std::fs::create_dir_all(&cli.out_dir).expect("create out_dir");
    let path = cli
        .out_dir
        .join(format!("{}_n{}.json", scheme.slug(), cli.n));
    let json = serde_json::to_string_pretty(&v).expect("serialize");
    std::fs::write(&path, json).expect("write fixture");
    println!("wrote {}", path.display());
}
