use clap::Parser;
use lez_ecdsa::make_test_vector;
use lez_ecdsa_methods::LEZ_ECDSA_ELF;
use nssa::{AccountId, program::Program};
use wallet::{PrivacyPreservingAccount, WalletCore};

#[derive(Parser, Debug)]
#[command(about = "Submit a privacy-preserving transaction that runs the lez_ecdsa verifier and time it end-to-end.")]
struct Cli {
    /// Optional path to a guest binary on disk; defaults to the embedded LEZ_ECDSA_ELF.
    #[arg(long)]
    program_path: Option<String>,

    /// Account ID to use as the PrivateOwned account for this private TX.
    #[arg(long)]
    account_id: String,

    /// How many signers to generate in the synthetic RedStone-shaped payload.
    #[arg(long, default_value_t = 3)]
    num_signers: usize,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let bytecode: Vec<u8> = if let Some(path) = &cli.program_path {
        std::fs::read(path).unwrap_or_else(|e| panic!("read program at `{path}`: {e}"))
    } else {
        LEZ_ECDSA_ELF.to_vec()
    };
    let program = Program::new(bytecode).expect("parse program");

    let account_id: AccountId = cli
        .account_id
        .strip_prefix("Private/")
        .or_else(|| cli.account_id.strip_prefix("Public/"))
        .unwrap_or(&cli.account_id)
        .parse()
        .expect("parse account_id");
    let accounts = vec![PrivacyPreservingAccount::PrivateOwned(account_id)];

    let input = make_test_vector(cli.num_signers);
    let serialized =
        Program::serialize_instruction(input).expect("serialize VerifyInput as instruction");

    let wallet_core = WalletCore::from_env().expect("WalletCore from env");

    println!("submitting privacy-preserving tx with {} signer(s)...", cli.num_signers);
    let t0 = std::time::Instant::now();
    wallet_core
        .send_privacy_preserving_tx(accounts, serialized, &program.into())
        .await
        .expect("send_privacy_preserving_tx");
    let elapsed = t0.elapsed();

    println!("---");
    println!("num_signers     = {}", cli.num_signers);
    println!("end_to_end_time = {:?}", elapsed);
}
