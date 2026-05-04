//! Local-prove bench: NSSA-wraps each guest's input, proves, records cycles
//! + wall-clock prove time + receipt size.
//!
//! `--all` runs the full (scheme × N) matrix plus the noop calibration
//! baseline. Output goes to `results/results.json` and
//! `results/README-snippet.md`.
//!
//! Requires `RISC0_DEV_MODE=0` for real numbers.

use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use lez_signature_bench::{Scheme, VerifyInput, make_test_vector};
use lez_signature_bench_methods::{
    ECDSA_P256_ELF, ECDSA_SECP256K1_ELF, ED25519_ELF, NOOP_ELF, SCHNORR_SECP256K1_ELF,
};
use risc0_zkvm::{ExecutorEnv, default_prover};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(about = "Local-prove bench across signature schemes inside the NSSA-wrapped guest.")]
struct Cli {
    /// Scheme slug (ecdsa-secp256k1 | schnorr-secp256k1 | ed25519 | ecdsa-p256). Ignored with --all.
    #[arg(long)]
    scheme: Option<String>,
    /// Number of signers in the synthetic fixture. Ignored with --all.
    #[arg(long)]
    n: Option<usize>,
    /// Run the full (scheme × N) matrix + noop baseline.
    #[arg(long)]
    all: bool,
    /// N values to sweep when --all is set.
    #[arg(long, value_delimiter = ',', default_values_t = [1usize, 3])]
    ns: Vec<usize>,
    /// Output directory for results.json + README snippet.
    #[arg(long, default_value = "results")]
    out_dir: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Row {
    scheme: String,
    n: usize,
    total_cycles: u64,
    user_cycles: u64,
    paging_cycles: u64,
    segments: usize,
    prove_seconds: f64,
    receipt_bytes: usize,
}

fn elf_for(scheme: Scheme) -> &'static [u8] {
    match scheme {
        Scheme::EcdsaSecp256k1 => ECDSA_SECP256K1_ELF,
        Scheme::SchnorrSecp256k1 => SCHNORR_SECP256K1_ELF,
        Scheme::Ed25519 => ED25519_ELF,
        Scheme::EcdsaP256 => ECDSA_P256_ELF,
    }
}

/// Build an `ExecutorEnv` matching what `nssa_core::read_nssa_inputs` expects.
/// Uses zero `ProgramId`, no caller, empty pre-states. The guest reads the
/// `VerifyInput` from the NSSA-encoded instruction data.
fn build_env<'a>(input: &'a VerifyInput) -> ExecutorEnv<'a> {
    let self_program_id: [u32; 8] = [0; 8];
    let caller_program_id: Option<[u32; 8]> = None;
    let pre_states: Vec<nssa_core::account::AccountWithMetadata> = Vec::new();
    let instruction_data: Vec<u32> = risc0_zkvm::serde::to_vec(input).expect("encode VerifyInput");

    ExecutorEnv::builder()
        .write(&self_program_id)
        .unwrap()
        .write(&caller_program_id)
        .unwrap()
        .write(&pre_states)
        .unwrap()
        .write(&instruction_data)
        .unwrap()
        .build()
        .unwrap()
}

fn run_one(label: &str, n: usize, elf: &'static [u8], input: &VerifyInput) -> Row {
    let env = build_env(input);
    let t0 = Instant::now();
    let prove_info = default_prover().prove(env, elf).expect("prove");
    let prove_seconds = t0.elapsed().as_secs_f64();
    let receipt_bytes = bincode::serialize(&prove_info.receipt)
        .expect("receipt bincode")
        .len();
    let stats = &prove_info.stats;
    Row {
        scheme: label.to_string(),
        n,
        total_cycles: stats.total_cycles,
        user_cycles: stats.user_cycles,
        paging_cycles: stats.paging_cycles,
        segments: stats.segments,
        prove_seconds,
        receipt_bytes,
    }
}

fn print_row(r: &Row) {
    println!(
        "{:<22} n={} cycles(total/user/paging)={}/{}/{} segs={} prove={:.2}s receipt={}B",
        r.scheme,
        r.n,
        r.total_cycles,
        r.user_cycles,
        r.paging_cycles,
        r.segments,
        r.prove_seconds,
        r.receipt_bytes,
    );
}

fn write_outputs(rows: &[Row], out_dir: &PathBuf) {
    std::fs::create_dir_all(out_dir).expect("create out_dir");

    let json = serde_json::to_string_pretty(rows).expect("serialize");
    std::fs::write(out_dir.join("results.json"), json).expect("write results.json");

    let mut md = String::new();
    md.push_str("| scheme | N | total cycles | user cycles | segments | prove time (s) | receipt size (B) |\n");
    md.push_str("|---|---:|---:|---:|---:|---:|---:|\n");
    for r in rows {
        md.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {:.2} | {} |\n",
            r.scheme,
            r.n,
            r.total_cycles,
            r.user_cycles,
            r.segments,
            r.prove_seconds,
            r.receipt_bytes,
        ));
    }
    std::fs::write(out_dir.join("README-snippet.md"), md).expect("write snippet");
    println!(
        "\nwrote {}/results.json and README-snippet.md",
        out_dir.display()
    );
}

fn main() {
    let cli = Cli::parse();

    if std::env::var("RISC0_DEV_MODE").as_deref() == Ok("1") {
        eprintln!("warning: RISC0_DEV_MODE=1 — numbers below are NOT real measurements.");
    }

    if cli.all {
        let mut rows: Vec<Row> = Vec::new();

        // Calibration: noop baseline always at N=1 (cleaner anchor).
        let baseline_input = make_test_vector(Scheme::EcdsaSecp256k1, 1);
        rows.push(run_one("noop", 1, NOOP_ELF, &baseline_input));
        print_row(rows.last().unwrap());

        for &scheme in Scheme::ALL {
            let elf = elf_for(scheme);
            for &n in &cli.ns {
                let input = make_test_vector(scheme, n);
                let row = run_one(scheme.slug(), n, elf, &input);
                print_row(&row);
                rows.push(row);
            }
        }

        write_outputs(&rows, &cli.out_dir);
    } else {
        let scheme_str = cli
            .scheme
            .as_deref()
            .expect("--scheme required without --all");
        let n = cli.n.expect("--n required without --all");

        if scheme_str == "noop" {
            let baseline_input = make_test_vector(Scheme::EcdsaSecp256k1, n);
            let row = run_one("noop", n, NOOP_ELF, &baseline_input);
            print_row(&row);
        } else {
            let scheme = Scheme::parse(scheme_str).expect("scheme");
            let input = make_test_vector(scheme, n);
            let row = run_one(scheme.slug(), n, elf_for(scheme), &input);
            print_row(&row);
        }
    }
}
