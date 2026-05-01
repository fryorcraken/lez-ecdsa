# lez-ecdsa

PoC: secp256k1 ECDSA signature verification over keccak256 inside a
Logos Execution Zone (LEZ) guest program, on the RISC Zero zkVM.

The goal is to **measure cost** (cycle count, prove time, receipt size)
of one `ecrecover` over a keccak256 hash. This number gates downstream
decisions about RedStone integration shape, multi-sig threshold cost,
and proposed framework extensions.

See `SPEC.md` for acceptance criteria, `PLAN.md` for the task breakdown.

## Status

- **Framework:** bare LEZ guest (no `spel-framework`, no `lez-framework`).
  `logos-co/spel` was the original target but its dep tree pulls
  `bonsai-sdk → reqwest → rustls → ring` into the `riscv32` guest target,
  where `ring` fails to cross-compile. Filed upstream:
  [logos-co/spel#165](https://github.com/logos-co/spel/issues/165). Once
  resolved, this project can adopt spel macros without changing the
  cryptographic kernel.
- **LEZ:** pinned to `v0.2.0-rc3`.
- **risc0-zkvm:** `3.0.5`, with patched `k256`, `tiny-keccak`, `sha2`,
  `crypto-bigint` (RISC Zero accelerated forks).

## Layout

```
.
├── methods/guest/src/bin/lez_ecdsa.rs   # Guest: ecrecover over keccak256
├── methods/guest/Cargo.toml              # Guest deps (no spel)
├── methods/                              # risc0-build harness
├── src/lib.rs                            # Shared VerifyInput type
├── src/bin/bench_verify.rs               # Host: synth vector + prove + measure
├── SPEC.md                               # Acceptance criteria
└── PLAN.md                               # Task breakdown
```

## Build

```bash
cargo build --workspace
```

## Run the bench

```bash
RISC0_DEV_MODE=0 cargo run --release --bin bench_verify
```

`RISC0_DEV_MODE=1` disables actual proving and produces fake numbers
— the bench refuses to run in that mode.

## Results

_TODO: filled in after first bench run completes._

| Metric | Value |
|---|---|
| Total cycles | _pending_ |
| User cycles | _pending_ |
| Prove time (mean of 5) | _pending_ |
| Receipt size | _pending_ |
| risc0-zkvm | 3.0.5 |
| LEZ | v0.2.0-rc3 |
| Machine | _pending_ |
