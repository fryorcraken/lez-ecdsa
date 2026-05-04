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

| scheme | N | total cycles | user cycles | segments | prove time | receipt size (B) |
|---|---:|---:|---:|---:|---:|---:|
| `noop` | 1 | 131 072 | 52 705 | 1 | 23.15 s (~0:23) | 245 306 |
| `ecdsa-secp256k1` | 1 | 524 288 | 372 875 | 1 | 141.47 s (~2:21) | 492 375 |
| `ecdsa-secp256k1` | 3 | 1 310 720 | 1 072 214 | 2 | 260.13 s (~4:20) | 763 049 |
| `schnorr-secp256k1` | 1 | 524 288 | 342 849 | 1 | 77.82 s (~1:18) | 269 234 |
| `schnorr-secp256k1` | 3 | 1 114 112 | 998 530 | 2 | 166.46 s (~2:46) | 505 140 |
| `ed25519` | 1 | 1 048 576 | 918 700 | 1 | 153.97 s (~2:34) | 282 482 |
| `ed25519` | 3 | 3 145 728 | 2 726 208 | 3 | 451.82 s (~7:32) | 846 678 |
| `ecdsa-p256` | 1 | 524 288 | 270 305 | 1 | 71.44 s (~1:11) | 269 242 |
| `ecdsa-p256` | 3 | 1 048 576 | 770 910 | 1 | 141.36 s (~2:21) | 284 074 |

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
| **30 s** | only the `noop` baseline (~0:23, no crypto) |
| **1 min** | nothing in scope |
| **1.5 min** | `ecdsa-p256` n=1 (~1:11); `schnorr-secp256k1` n=1 (~1:18) |
| **3 min** | adds `ecdsa-secp256k1` n=1 (~2:21), `ecdsa-p256` n=3 (~2:21), `ed25519` n=1 (~2:34), `schnorr-secp256k1` n=3 (~2:46) |
| **5 min** | adds `ecdsa-secp256k1` n=3 (~4:20) |
| **8 min** | adds `ed25519` n=3 (~7:32) |

For interactive RedStone-style oracle UX (3-of-N pulls, sub-30 s),
**no scheme fits on CPU**. CUDA / Bonsai would compress this
dramatically; CPU alone is too heavy for low-latency UX.

For batch / async workloads (sub-5 min acceptable), **secp256k1 Schnorr
or P-256 at N=3** is the budget pick. If keys / addresses must stay
secp256k1 (Ethereum compat), Schnorr is the natural step up from ECDSA.

### End-to-end private-TX time (against `lgs localnet`)

The numbers above isolate the inner proving cost. The number a user
actually feels — "click submit on a privacy-preserving transaction →
confirmation back from the sequencer" — adds NSSA framing, the
privacy-preserving circuit (which proves the account-state transition
on top of our verifier), and the sequencer roundtrip. Measured against
a fresh `lgs localnet` and a fresh `PrivateOwned` account, same
machine, same `RISC0_DEV_MODE=0`:

| scheme | N | local prove | E2E private TX | wrap overhead |
|---|---:|---:|---:|---:|
| `noop`              | 1 | 23.15 s (~0:23) | 103.46 s (~1:43) | +80.3 s (+347%) |
| `ecdsa-secp256k1`   | 1 | 141.47 s (~2:21) | 246.46 s (~4:06) | +105.0 s (+74%) |
| `ecdsa-secp256k1`   | 3 | 260.13 s (~4:20) | 446.12 s (~7:26) | +186.0 s (+72%) |
| `schnorr-secp256k1` | 1 | 77.82 s (~1:18) | 153.67 s (~2:33) | +75.9 s (+97%) |
| `schnorr-secp256k1` | 3 | 166.46 s (~2:46) | 322.95 s (~5:22) | +156.5 s (+94%) |
| `ed25519`           | 1 | 153.97 s (~2:34) | 282.95 s (~4:42) | +129.0 s (+84%) |
| `ed25519`           | 3 | 451.82 s (~7:32) | 669.66 s (~11:09) | +217.8 s (+48%) |
| `ecdsa-p256`        | 1 | 71.44 s (~1:11) | 152.50 s (~2:32) | +81.1 s (+113%) |
| `ecdsa-p256`        | 3 | 141.36 s (~2:21) | 298.82 s (~4:58) | +157.5 s (+111%) |

The privacy-preserving wrapping adds **roughly 75–220 s** per TX. It's
not a fixed overhead: there's a constant component (~80 s, visible on
the `noop` row) plus a variable component that scales with the inner
kernel's segment count. Larger kernels (e.g. `ed25519` N=3) see a lower
*percentage* overhead because the fixed component is amortized.

Scheme ranking carries over from local-prove to E2E unchanged:
`ecdsa-p256` ≈ `schnorr-secp256k1` < `ecdsa-secp256k1` < `ed25519`.
The wrap overhead compresses the spread (Ed25519 is ~1.9× ECDSA-k1 in
E2E vs ~2.7× in user cycles), so for end-user latency the gap is real
but smaller than the cycle deltas suggest.

Net for the headline RedStone shape — **3-of-N pull, end-to-end**:

| scheme | N=3 E2E |
|---|---:|
| `ecdsa-p256` | **4:58** (cheapest) |
| `schnorr-secp256k1` | 5:22 |
| `ecdsa-secp256k1` | 7:26 |
| `ed25519` | 11:09 |

For interactive UX (sub-30 s) **no scheme fits on CPU**. CUDA / Bonsai
would compress this; CPU alone is too heavy.

## Methodology

Each local-prove row is one **real prove pass** through
`risc0_zkvm::default_prover()` with `RISC0_DEV_MODE=0`, against the
NSSA-wrapped guest binary for that scheme. Inputs are written in the
exact shape `nssa_core::read_nssa_inputs` expects (`self_program_id`,
`caller_program_id`, `pre_states`, NSSA-encoded instruction). Cycles
and segment count come from `ProveInfo.stats`; prove time is wall-clock
around the `prove(...)` call; receipt size is
`bincode::serialize(&receipt).len()`.

Each E2E row submits a real privacy-preserving transaction via
`wallet::WalletCore::send_privacy_preserving_tx()` against a running
`lgs localnet` (sequencer in `risc0_dev_mode=true`, client in
`RISC0_DEV_MODE=0`). The wall clock wraps `send_privacy_preserving_tx`,
so it includes serialization, client-side proving (kernel +
privacy-preserving wrapping circuit), the sequencer roundtrip, and
confirmation.

Synthetic fixtures only — no captured oracle payloads. All signers
share the same message (`b"hello redstone"`) per row, signed with
deterministic seeds.

### Receipt-size note

`ecdsa-secp256k1`'s 492 KB local-prove receipt is roughly 1.8× any
other scheme's receipt at the same segment count. This is the keccak
coprocessor: `tiny-keccak`'s RISC0 patch routes each permutation
through `risc0_keccak_update`, and the coprocessor's STARK proof is
attached as a receipt assumption (~247 KB per keccak call vs ~24 KB
for sha256-precompile use). The cost is mostly inherent to using
keccak — switching to sha256 prehash would save the bytes but break
Ethereum compatibility.

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

# End-to-end matrix against a running lgs localnet
lgs localnet start
lgs wallet -- account new private    # note the printed Private/<id>
NSSA_WALLET_HOME_DIR="$PWD/.scaffold/wallet" RISC0_DEV_MODE=0 \
  cargo run --release --bin bench -- \
  --all --account-id Private/<your-id> --label e2e
# → results/results-e2e.json + results/README-snippet-e2e.md

# Generate just the JSON fixture for one (scheme, N) point
cargo run --release --bin gen_test_vectors -- \
  --scheme schnorr-secp256k1 --n 3
```

`RISC0_DEV_MODE=1` skips actual proving and prints fake numbers.
The bench logs a warning when this is set; never quote those numbers.

The local matrix takes **~28 minutes** on the reference machine; the
end-to-end matrix takes **~46 minutes** end-to-end (kernel + wrapping
+ sequencer roundtrip per row).

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

- **GPU / Bonsai prove.** All numbers above are CPU-only on the
  reference Ryzen 9. CUDA acceleration or Bonsai would collapse prove
  time substantially; left as future work.
- **Threshold cryptography** (Schnorr / Ed25519 / BLS threshold sigs).
- **Batch verification** for Schnorr and Ed25519 — could collapse
  3-of-N cost meaningfully; out of the headline matrix.
- **N-sweep** beyond {1, 3} for the winning scheme.
- **Multi-machine numbers.** Cycles generalize; prove time doesn't.
- **RedStone payload parsing.**

## License

MIT or Apache-2.0 (per workspace `Cargo.toml`).
