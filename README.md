# lez-signature-bench

> ⚠️ **AI-generated codebase — research bench only.** This repository
> was produced with AI assistance to get a rough indication of
> signature-verification cost on RISC Zero / LEZ. It has not been
> audited. **It MUST NOT be used in any mainnet program** or any
> production system. Use it for measurement, comparison, and
> intuition-building — nothing else.

A comparative benchmark of common signature-verification schemes
inside the [Logos Execution Zone (LEZ)](https://github.com/logos-blockchain/logos-execution-zone)
guest model on the [RISC Zero zkVM](https://risczero.com/). Each scheme
runs inside an NSSA-wrapped guest using RISC Zero's accelerated curve
arithmetic, and is benchmarked end-to-end through the local prover.

The goal is *data*, not deployment: cycles, prove time, and receipt
size for each scheme so a developer choosing a verification primitive
for oracle / multi-sig / passkey workloads on LEZ can predict cost
on consumer hardware.

See [`SPEC.md`](./SPEC.md) for acceptance criteria and
[`PLAN.md`](./PLAN.md) for the task breakdown.

## Schemes in scope

| Scheme | Curve | Prehash | RISC0 patch source |
|---|---|---|---|
| ECDSA secp256k1 | secp256k1 | keccak256 | `k256/v0.13.4-risczero.1` |
| Schnorr secp256k1 (BIP-340) | secp256k1 | sha256 | (same fork as ECDSA) |
| Ed25519 | Curve25519 | none (sha512 internal) | `curve25519-4.1.3-risczero.0` |
| ECDSA P-256 | NIST P-256 | sha256 | `p256/v0.13.2-risczero.1` |

All four schemes hit RISC Zero's `bigint2` / curve precompile path —
verified in the Phase 1 spike. Schnorr secp256k1 routes through the
same `ProjectivePoint::lincomb` accelerated path that ECDSA secp256k1
uses (no separate code path); Ed25519 uses curve25519-dalek's
`backend/serial/risc0` module.

## Results

Real-prove run, single-pass per cell on an idle machine. NSSA-wrapped
guests, `RISC0_DEV_MODE=0`, one synthetic same-message fixture per
`(scheme, N)`.

**Machine:** AMD Ryzen 9 7940HS (16 threads, AMD Radeon 780M iGPU,
60 GiB RAM), Linux 6.19, CPU prover (no CUDA, no Bonsai).
**Stack:** risc0-zkvm 3.0.5, LEZ v0.2.0-rc3, Rust 1.92.0.

| scheme | N | total cycles | user cycles | segments | prove time (s) | receipt size (B) |
|---|---:|---:|---:|---:|---:|---:|
| `noop` | 1 | 131 072 | 52 705 | 1 | 23.15 | 245 306 |
| `ecdsa-secp256k1` | 1 | 524 288 | 372 875 | 1 | 141.47 | 492 375 |
| `ecdsa-secp256k1` | 3 | 1 310 720 | 1 072 214 | 2 | 260.13 | 763 049 |
| `schnorr-secp256k1` | 1 | 524 288 | 342 849 | 1 | 77.82 | 269 234 |
| `schnorr-secp256k1` | 3 | 1 114 112 | 998 530 | 2 | 166.46 | 505 140 |
| `ed25519` | 1 | 1 048 576 | 918 700 | 1 | 153.97 | 282 482 |
| `ed25519` | 3 | 3 145 728 | 2 726 208 | 3 | 451.82 | 846 678 |
| `ecdsa-p256` | 1 | 524 288 | 270 305 | 1 | 71.44 | 269 242 |
| `ecdsa-p256` | 3 | 1 048 576 | 770 910 | 1 | 141.36 | 284 074 |

The `noop` row is the NSSA-wrap-only calibration baseline (52 705 user
cycles with empty pre-states, no crypto). Subtract it from any other
row for the per-scheme verify cost in cycles. Note `ecdsa-secp256k1`
shows a much larger receipt than other schemes at N=1; this is driven
by the segment-padding power-of-two and may also reflect the ELF that
keccak256 + k256 path pulls in. Re-runs may vary ±10% on prove time.

### Per-signature cycle deltas (subtracting noop)

| scheme | user cycles / sig (N=1) | user cycles / sig (≈ from N=3) |
|---|---:|---:|
| ecdsa-secp256k1 | 320 170 | 339 836 |
| schnorr-secp256k1 | 290 144 | 315 275 |
| ecdsa-p256       | 217 600 | 239 401 |
| ed25519          | 865 995 | 891 168 |

**Headline takes:**

- **P-256 ECDSA is the cheapest** verify on this stack — about
  **32% fewer cycles per sig than secp256k1 ECDSA** at N=1. The same
  RISC0 `bigint2` curve precompile applies and the field is similar
  cost; what differs is keccak256 (k256 path) vs sha256 (p256 path)
  for the message digest.
- **Schnorr secp256k1 ≈ 9% cheaper than ECDSA secp256k1** per sig.
  The expected win from skipping the modular inversion is partially
  offset by Schnorr's `ProjectivePoint::lincomb(G, s, P, -e)` doing
  one combined mul vs ECDSA's separate mul + recovery. Same precompile
  path, smaller win than naive theory predicts.
- **Ed25519 is by far the most expensive** here — **2.7× the user
  cycles** of secp256k1 ECDSA. The RISC0 curve25519-dalek backend is
  available and active, but Edwards arithmetic plus the in-algorithm
  sha512 (no zkVM precompile) dominates.
- **Multi-sig scaling is roughly linear** for all schemes — about
  **+320–340K user cycles per added secp256k1 sig**, **+870K** per
  Ed25519 sig. No batch-verify shortcuts here (out of scope per
  [`SPEC.md`](./SPEC.md) §9).

### Decision note: budget → scheme on this laptop

Given the prove times above on a CPU-only Ryzen 9 7940HS:

| TX prove budget | What fits |
|---|---|
| **30 s** | only the `noop` baseline (no crypto) |
| **60 s** | nothing in scope |
| **90 s** | `ecdsa-p256` n=1 (71 s); `schnorr-secp256k1` n=1 (78 s) |
| **3 min** | adds `ecdsa-secp256k1` n=1 (141 s), `ecdsa-p256` n=3 (141 s), `ed25519` n=1 (154 s) |
| **5 min** | adds `schnorr-secp256k1` n=3 (166 s), `ecdsa-secp256k1` n=3 (260 s) |
| **8 min** | adds `ed25519` n=3 (452 s) |

For interactive RedStone-style oracle UX (3-of-N pulls, sub-30 s),
**no scheme fits on CPU**. CUDA / Bonsai would compress this
dramatically; CPU alone is too heavy for low-latency UX.

For batch / async workloads (sub-5 min acceptable), **secp256k1 Schnorr
or P-256 at N=3** is the budget pick. If keys / addresses must stay
secp256k1 (Ethereum compat), Schnorr is the natural step up from ECDSA.

## Methodology

Each row is one **real prove pass** through `risc0_zkvm::default_prover()`
with `RISC0_DEV_MODE=0`, against the NSSA-wrapped guest binary for that
scheme. Inputs are written in the exact shape `nssa_core::read_nssa_inputs`
expects (`self_program_id`, `caller_program_id`, `pre_states`, NSSA-encoded
instruction). Cycles and segment count come from `ProveInfo.stats`;
prove time is wall-clock around the `prove(...)` call; receipt size is
`bincode::serialize(&receipt).len()`.

The bench does **not** submit through a real LEZ private TX — that
requires a running devnet account and is the next milestone. See
SPEC §1 for what end-to-end coverage adds (sequencer roundtrip,
privacy-preserving wrapping). The numbers above isolate the **inner
proving cost**, which is what changes between schemes.

Synthetic fixtures only — no captured oracle payloads. All signers
share the same message (`b"hello redstone"`) per row, signed with
deterministic seeds.

## Build

```bash
cargo build --workspace --release
```

`risc0-build` cross-compiles five guest ELFs (one per scheme + the
noop baseline) for `riscv32im-risc0-zkvm-elf` and embeds them as
`{ECDSA_SECP256K1,SCHNORR_SECP256K1,ED25519,ECDSA_P256,NOOP}_ELF`.

## Run the bench

```bash
# One scheme + N
RISC0_DEV_MODE=0 cargo run --release --bin bench -- \
  --scheme ecdsa-secp256k1 --n 1

# Full matrix → results/results.json + results/README-snippet.md
RISC0_DEV_MODE=0 cargo run --release --bin bench -- --all

# Generate just the JSON fixture for one (scheme, N) point
cargo run --release --bin gen_test_vectors -- \
  --scheme schnorr-secp256k1 --n 3
```

`RISC0_DEV_MODE=1` skips actual proving and prints fake numbers.
The bench logs a warning when this is set; never quote those numbers.

The matrix run on the reference machine takes **~28 minutes** end-to-end.

## Lint and test

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace      # 9 host tests: byte-stable wire format,
                            # positive + negative round-trip per scheme
```

CI (`.github/workflows/ci.yml`) runs the fmt + clippy + build set on
every push and PR. Bench runs are local only.

## Prerequisites

- Rust toolchain 1.92.0 (auto-picked via `rust-toolchain.toml`).
- The RISC Zero RISC-V guest toolchain. Install via
  [`rzup`](https://dev.risczero.com/api/zkvm/install):
  ```bash
  curl -L https://risczero.com/install | bash
  rzup install
  ```
- ~30 minutes of idle CPU time for a full matrix run.

## Layout

```
.
├── methods/
│   ├── build.rs                          # risc0_build::embed_methods()
│   └── guest/
│       ├── src/lib.rs                    # shared VerifyInput + per-scheme verifier modules
│       └── src/bin/
│           ├── ecdsa_secp256k1.rs        # NSSA-wrapped guest, one per scheme
│           ├── schnorr_secp256k1.rs
│           ├── ed25519.rs
│           ├── ecdsa_p256.rs
│           └── noop.rs                   # NSSA-wrap-only calibration
├── src/
│   ├── lib.rs                            # Scheme enum + host-side fixtures + verify_all
│   ├── verifier/                         # host-callable verify per scheme (mirrors guest)
│   └── bin/
│       ├── bench.rs                      # local-prove bench (single + --all)
│       └── gen_test_vectors.rs           # write JSON fixture for one (scheme, N)
├── Cargo.toml                            # workspace + [patch.crates-io] for risc0 crypto forks
├── results/                              # gitignored — matrix output lives here
├── fixtures/                             # gitignored
├── SPEC.md                               # acceptance criteria + boundaries
├── PLAN.md                               # 5-phase task breakdown
└── README.md                             # this file
```

## Roadmap

Deliberately deferred (see [`SPEC.md`](./SPEC.md) §9):

- **End-to-end private-TX timing on a real LEZ devnet.** The bench
  currently isolates inner proving cost; the full-TX path adds NSSA
  framing, privacy-preserving circuit, and sequencer roundtrip — a
  ~60–70% multiplier in earlier passes. Reproducible once a devnet
  account_id is available; no scheme switch is needed.
- **Threshold cryptography** (Schnorr / Ed25519 / BLS threshold sigs).
- **Batch verification** for Schnorr and Ed25519 — could collapse
  3-of-N cost meaningfully; out of the headline matrix.
- **N-sweep** beyond {1, 3} for the winning scheme.
- **Multi-machine numbers.** Cycles generalize; prove time doesn't.
- **RedStone payload parsing.**

## License

MIT or Apache-2.0 (per workspace `Cargo.toml`).
