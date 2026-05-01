# lez-ecdsa

A proof-of-concept guest program for the
[Logos Execution Zone (LEZ)](https://github.com/logos-blockchain/logos-execution-zone)
that performs **secp256k1 ECDSA signature verification over keccak256
hashes** inside the [RISC Zero zkVM](https://risczero.com/) — the cryptographic
kernel needed to verify [RedStone oracle](https://docs.redstone.finance/) data
in pull mode.

The current goal is **measurement, not deployment**: how many cycles, how much
prove time, how big a receipt for one `ecrecover` over `keccak256(message)`?
Those numbers gate every downstream decision (multi-sig threshold cost, push
vs pull mode shape, eventual framework wrapping).

See [`SPEC.md`](./SPEC.md) for acceptance criteria and
[`PLAN.md`](./PLAN.md) for the task breakdown.

## Status

Bare LEZ guest, no `spel-framework` / `lez-framework` wrapper. The original
plan was to use [`logos-co/spel`](https://github.com/logos-co/spel) for
Anchor-style ergonomics, but its dep tree pulls
`bonsai-sdk → reqwest → rustls → ring` into the `riscv32` guest target, where
`ring`'s build script can't cross-compile (see upstream issue
[logos-co/spel#165](https://github.com/logos-co/spel/issues/165)). Once that
lands, the verifier kernel here can be wrapped in `#[lez_program]` /
`#[instruction]` macros without changing the cryptographic core.

| Component | Version |
|---|---|
| LEZ (`nssa`, `nssa_core`, `wallet`, `common`) | `v0.2.0-rc3` |
| `risc0-zkvm` | `3.0.5` |
| Patched crypto | `k256 v0.13.4-risczero.1`, `tiny-keccak v2.0.2-risczero.0`, `sha2 v0.10.9-risczero.0`, `crypto-bigint v0.5.5-risczero.0` |
| Rust toolchain | pinned via `rust-toolchain.toml` (`1.92.0`) |

## Layout

```
.
├── methods/
│   ├── Cargo.toml                       # risc0-build harness for the guest
│   ├── build.rs                         # calls risc0_build::embed_methods()
│   └── guest/
│       ├── Cargo.toml                   # guest deps (risc0-zkvm, k256, tiny-keccak)
│       └── src/bin/lez_ecdsa.rs         # the verifier guest binary
├── src/
│   ├── lib.rs                           # shared VerifyInput type
│   └── bin/bench_verify.rs              # host: synthetic vector + prove + measure
├── Cargo.toml                           # workspace + [patch.crates-io] for risc0 forks
├── rust-toolchain.toml                  # pins toolchain
├── scaffold.toml                        # logos-scaffold metadata (not required for cargo build)
├── .github/workflows/ci.yml             # fmt + clippy + build CI
├── SPEC.md                              # acceptance criteria, boundaries, non-goals
├── PLAN.md                              # task breakdown
└── README.md                            # this file
```

## Prerequisites

- Rust toolchain (the project pins `1.92.0` via `rust-toolchain.toml`; rustup
  picks it up automatically).
- The RISC Zero RISC-V guest toolchain. Install via
  [`rzup`](https://dev.risczero.com/api/zkvm/install):
  ```bash
  curl -L https://risczero.com/install | bash
  rzup install
  ```
  This puts `riscv32-unknown-elf-gcc` and the `riscv32im-risc0-zkvm-elf`
  Rust target into `~/.risc0/`.
- (Optional) [`logos-scaffold`](https://github.com/logos-co/logos-scaffold)
  if you want to deploy via `lgs deploy` later. Not needed for the local
  PoC.

## Build

```bash
cargo build --workspace
```

This compiles the host CLI (`bench_verify`) and, via `risc0-build`, the
RISC-V guest binary embedded as `LEZ_ECDSA_ELF`.

## Run the bench yourself

```bash
RISC0_DEV_MODE=0 cargo run --release --bin bench_verify
```

What it does:

1. Generates a deterministic secp256k1 keypair from a hardcoded 32-byte seed.
2. Signs the message `b"hello redstone"` (keccak256-hashed) with that key.
3. Hands `(message, signature, expected_signer_address)` to the guest.
4. Inside the guest, `ecrecover` reconstructs the signing pubkey, derives
   the Ethereum-style 20-byte address (`keccak256(pubkey)[12..]`), and commits
   `(recovered_address, matches_bool)` to the receipt journal.
5. The host decodes the journal, prints `total_cycles`, `user_cycles`,
   `prove_time`, and `receipt_size`, and exits 0 iff `matches == true`.

`RISC0_DEV_MODE=1` skips actual proving and produces fake numbers — the
bench refuses to run in that mode (exits 1 with a `WARN` line).

Expected output shape:

```
expected_signer = 0x7402fee841da96a1fc4056d778bbfa1dea509ba9
---
matches      = true
recovered    = 0x7402fee841da96a1fc4056d778bbfa1dea509ba9
total_cycles = <N>
user_cycles  = <M>
prove_time   = <duration>
receipt_size = <bytes>
```

### Tweak the bench

The synthetic vector lives in `make_test_vector()` in
`src/bin/bench_verify.rs`. Change the seed (`sk_bytes`) or the message to
test other inputs. Flipping a single byte of `signature.r` is a quick way
to confirm the verifier doesn't trivially pass — the bench will print
`matches = false` (or panic with `ecrecover failed` if the signature
doesn't decode at all).

## Verify against an independent reference

The recovered address printed above can be cross-checked against any
independent secp256k1 library. With [`foundry`](https://getfoundry.sh):

```bash
# The seed used in make_test_vector(), as a hex private key:
cast wallet address \
  0x4c0ac86f124da091c73eb855296efb1000c44d68a9a36e2d83b15577916eabcd
# Should print 0x7402fee841da96a1fc4056d778bbfa1dea509ba9
```

If the addresses match, the entire chain (key derivation, hashing,
signing, verifying, recovering) is consistent.

## Lint and test

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

CI (`.github/workflows/ci.yml`) runs the same three commands plus a
debug `cargo build --workspace` on every push and PR.

## Results

End-to-end runs on commodity hardware. Single-run numbers per cell;
mean ± stddev over 5 runs is a follow-up. Machine: **AMD Ryzen 9 7940HS,
16 threads, 60 GiB RAM, CPU prove (no CUDA, no Bonsai)**. Stack:
risc0-zkvm 3.0.5, LEZ v0.2.0-rc3, patched k256/tiny-keccak/sha2/crypto-bigint.

| Variant | N | user cycles | total cycles | prove time | receipt size |
|---|---:|---:|---:|---:|---:|
| `recover_from_prehash` (v1) | 1 | 639,802 | 1,048,576 | 253.8 s | 493 KiB |
| `recover_from_prehash` (v1) | 3 | 1,881,476 | 2,097,152 | 381.5 s | 768 KiB |
| `verify_prehash` (v2, current) | 1 | **341,111** | 524,288 | **130.3 s** | 480 KiB |
| `verify_prehash` (v2, current) | 3 | **993,219** | 1,114,112 | **225.1 s** | 709 KiB |

**Optimization that landed (v1 → v2):** swap `VerifyingKey::recover_from_prehash`
for `verify_prehash` against a known pubkey. This drops the "find which
pubkey matches this `r`" step entirely. For RedStone-style consumers
(allowlist of known signers, identified by pubkey) this is the natural
shape — recovery semantics are only needed when the signer is unknown.

The trade-off: input is `pubkey: Vec<u8>` (33-byte SEC1-compressed) per
signer instead of `expected_signer: [u8; 20]` (Ethereum address). On-chain
storage of the allowlist is +13 B/signer.

Per-signature kernel cost (verify variant):

- ~331K user cycles per ECDSA verify (linear with N).
- 1 sig fits in a single 2¹⁹-cycle segment.
- 3 sigs straddles 2 segments (one full 2²⁰, one tiny ~65K).

Headline take: **~47% fewer cycles, ~45% less prove time, ~5–8% smaller
receipt** vs the recover-based first cut. Single-sig private TX is now
~2 min CPU prove; 3-sig is ~3:45 CPU prove.

To reproduce:

```bash
RISC0_DEV_MODE=0 LEZ_ECDSA_SIGNERS=1 cargo run --release --bin bench_verify
RISC0_DEV_MODE=0 LEZ_ECDSA_SIGNERS=3 cargo run --release --bin bench_verify
```

## Roadmap

This PoC ships only the cryptographic kernel. Deliberately deferred (see
[`SPEC.md`](./SPEC.md) §7):

- RedStone payload parsing — synthetic test vector only for now.
- Multi-sig threshold (M-of-N) — exactly one ECDSA verify here.
- Push mode (write to PDA) and pull mode (output for tail call) — once
  cost is acceptable, both modes get wired up.
- SPeL framework wrapping — pending [logos-co/spel#165](https://github.com/logos-co/spel/issues/165).
- Deployment via `lgs deploy` — local proving only.

## License

MIT or Apache-2.0 (per workspace `Cargo.toml`).
