# Implementation Plan: lez-ecdsa PoC

Companion to `SPEC.md`. Each task here is sized to one focused session,
has acceptance criteria, and has a verification step the build phase
runs before claiming "done."

## Overview

Replace the scaffold's counter example with a single-instruction ECDSA
verifier (`secp256k1` ecrecover over `keccak256`), measure its cost in
the RISC Zero zkVM under SPeL (`logos-co/spel`, main HEAD), wire CI,
and push to `github.com/fryorcraken/lez-ecdsa`.

## Architecture Decisions

- **Framework:** `logos-co/spel` (`spel-framework`), tracked at git
  `branch = "main"`. The scaffold's default `lez-framework` (jimmy-claw,
  unmaintained) is replaced wholesale.
- **Crypto primitives:** RISC Zero patched `k256` + `tiny-keccak` +
  `sha2` + `crypto-bigint` via `[patch.crates-io]`. Pure-Rust
  alternatives would inflate cycles ~100×.
- **Project layout:** keep the scaffold's shape (`src/lib.rs` for the
  host-side `#[lez_program]` mirror used for IDL gen, `methods/guest`
  for the proven binary, `src/bin/run_*.rs` for the host runner). Don't
  introduce a separate `core` crate — IDL gen already happens via
  `src/lib.rs`.
- **Test vectors:** synthetic only, generated host-side with a fixed
  seed. No captured RedStone payloads in this PoC.
- **Cost output:** if `lgs` CLI exposes cycle counts, use that; else
  drop to `risc0_zkvm::default_prover()` directly in the bench binary.

## Dependency graph

```
Task 1 (spel swap)
    │
    ├── Task 2 (baseline build)
    │       │
    │       ├── Task 3 (git init + first commit)
    │       │
    │       └── Task 4 (crypto patches)
    │               │
    │               └── Task 5 (replace counter → verifier)
    │                       │
    │                       └── Task 6 (build + IDL regen)
    │                               │
    │                               ├── Task 7 (test-vector gen)
    │                               │       │
    │                               │       └── Task 8 (bench harness)
    │                               │               │
    │                               │               └── Task 9 (record numbers)
    │                               │
    │                               └── Task 10 (CI workflow)
    │                                       │
    │                                       └── Task 11 (gh repo create + push)
```

## Task List

### Phase 1: Foundation (spel + build + git)

#### Task 1 — Swap workspace from lez-framework → spel-framework@main

**Description:** Replace all three `lez-framework*` git deps in the
workspace with `spel-framework*` (logos-co/spel) at `branch = "main"`.
Update Rust source to use `spel_framework::prelude::*` and rename
`LezResult`/`LezOutput`/`LezError` → `SpelResult`/`SpelOutput`/`SpelError`.

**Acceptance criteria:**
- [ ] No reference to `lez-framework`, `lez_framework`, or
      `jimmy-claw/lez-framework` remains in any `Cargo.toml` or `.rs` file.
- [ ] All workspace `Cargo.toml` files reference
      `https://github.com/logos-co/spel.git` with `branch = "main"`.
- [ ] `crates/lez-client-gen/src/main.rs` calls
      `spel_client_gen::generate_from_idl_json` (or whatever the spel
      equivalent exports).

**Verification:**
- [ ] `grep -r "lez-framework\|lez_framework\|LezResult\|LezOutput\|LezError" --include="*.rs" --include="*.toml" .` returns no hits (excluding `target/`).
- [ ] `cargo metadata --format-version 1 | jq '.packages[] | select(.name | startswith("spel"))'` lists the spel crates.

**Dependencies:** None (after `lgs setup` finishes the baseline infra build).

**Files likely touched:**
- `Cargo.toml`
- `methods/guest/Cargo.toml`
- `crates/lez-client-gen/Cargo.toml`
- `methods/guest/src/bin/lez_counter.rs` (or its replacement)
- `src/lib.rs`
- `src/bin/run_lez_counter.rs`
- `crates/lez-client-gen/src/main.rs`

**Estimated scope:** Medium (3-5 files, mechanical rename across most).

---

#### Task 2 — Baseline build with spel passes

**Description:** Verify the unmodified-otherwise scaffold compiles end
to end after the spel swap. Surfaces any API drift between spel and
lez-framework before we add our own code.

**Acceptance criteria:**
- [ ] `lgs build` exits 0.
- [ ] `cargo build --workspace` exits 0.
- [ ] `cargo test --workspace --no-run` exits 0 (compile tests).

**Verification:**
- [ ] Above three commands run clean from a fresh shell.

**Dependencies:** Task 1.

**Files likely touched:** None (this is verification only). If it
fails, edits in this task fix the compatibility breakage and stay
within the scaffold's existing files.

**Estimated scope:** Small.

---

#### Task 3 — git init + first commit + remote-less

**Description:** Initialize git in the project, ensure `.gitignore`
excludes `target/`, `.scaffold/`, `.env.local`, `.claude/`, and
`fixtures/`. First commit is the post-spel-swap scaffold + SPEC.md +
PLAN.md. **Do NOT push yet** — Task 11 handles remote creation.

**Acceptance criteria:**
- [ ] `git status` shows a clean tree on `main` after the commit.
- [ ] `target/`, `.scaffold/`, `.env.local`, `.claude/`, `fixtures/`
      not tracked.
- [ ] No private keys, secrets, or `.env*` files staged.

**Verification:**
- [ ] `git log --oneline` shows the initial commit.
- [ ] `git ls-files | grep -E '(\.env|\.scaffold|target|\.claude|fixtures)'` returns nothing.

**Dependencies:** Task 2.

**Files likely touched:**
- `.gitignore` (already updated)
- New `.git/` (created by `git init`)

**Estimated scope:** Small.

---

### Checkpoint: Foundation
- [ ] `lgs build` green with spel main HEAD
- [ ] git repo initialized, clean tree, no secrets
- [ ] Ready to start the verifier itself

---

### Phase 2: Verifier core

#### Task 4 — Add RISC Zero crypto patches

**Description:** Add `[patch.crates-io]` block to the workspace
`Cargo.toml` with the RISC Zero accelerated forks of `k256`, `sha2`,
`crypto-bigint`, and `tiny-keccak`. Tags pulled from
`risc0/risc0/examples/ecdsa/k256/methods/guest/Cargo.toml`.

**Acceptance criteria:**
- [ ] `[patch.crates-io]` block present in workspace `Cargo.toml` with
      these four entries (exact tags TBD against risc0-zkvm 3.0.5):
      ```
      sha2          = git "risc0/RustCrypto-hashes",          tag "sha2-v0.10.9-risczero.0"
      k256          = git "risc0/RustCrypto-elliptic-curves", tag "k256/v0.13.4-risczero.1"
      crypto-bigint = git "risc0/RustCrypto-crypto-bigint",   tag "v0.5.5-risczero.0"
      tiny-keccak   = git "risc0/tiny-keccak",                tag "tiny-keccak/v2.0.2-risczero.0"
      ```

**Verification:**
- [ ] `cargo build --workspace` still passes after the patch block.
      (No code yet uses k256/keccak — this just verifies the patches
      resolve.)
- [ ] `cargo tree -p k256 --workspace` shows the patched git source.

**Dependencies:** Task 3.

**Files likely touched:**
- `Cargo.toml` (workspace)

**Estimated scope:** Small.

---

#### Task 5 — Replace counter program with ECDSA verifier

**Description:** Replace the scaffold's `lez_counter` program with a
single-instruction `lez_ecdsa` program. The instruction takes
`(message, signature, expected_signer)` as args and returns the
recovered Ethereum address + a `matches` boolean. Renames extend to
the host-side mirror in `src/lib.rs`, the runner binary, the IDL
filename, and the methods crate's `[[bin]]` entry.

**Acceptance criteria:**
- [ ] `methods/guest/src/bin/lez_ecdsa.rs` exists and contains one
      `#[lez_program] mod lez_ecdsa { #[instruction] pub fn verify(...) }`.
- [ ] The instruction accepts `(authority: AccountWithMetadata,
      message: Vec<u8>, signature: [u8; 65], expected_signer: [u8; 20])`
      and returns `SpelResult<...>` whose output includes the recovered
      20-byte address and a `bool`.
- [ ] keccak256 + ecrecover use the patched crates (no pure-Rust SHA-3
      crate added to deps).
- [ ] `src/lib.rs` mirror is updated to match (same `mod lez_ecdsa`).
- [ ] `src/bin/run_lez_ecdsa.rs` exists (renamed from
      `run_lez_counter.rs`); it loads the renamed `LEZ_ECDSA_ELF`.
- [ ] `methods/guest/Cargo.toml` `[[bin]] name = "lez_ecdsa"`.
- [ ] No `lez_counter` references remain anywhere except possibly the
      old `idl/lez_counter.json` (which gets replaced/removed in Task 6).

**Verification:**
- [ ] `grep -r "lez_counter" --include="*.rs" --include="*.toml" .`
      returns 0 hits.

**Dependencies:** Task 4.

**Files likely touched:**
- `methods/guest/src/bin/lez_ecdsa.rs` (new — renamed)
- `methods/guest/Cargo.toml`
- `src/lib.rs`
- `src/bin/run_lez_ecdsa.rs` (renamed)
- `Cargo.toml` (binary references in `[dependencies]`)
- delete `methods/guest/src/bin/lez_counter.rs`

**Estimated scope:** Medium (5 files touched).

---

#### Task 6 — Build + regenerate IDL

**Description:** Run `lgs build` and `lgs build idl` (or whatever the
scaffold uses) to regenerate the IDL JSON for the new program. Confirm
the IDL reflects the new instruction signature.

**Acceptance criteria:**
- [ ] `lgs build` exits 0 with the verifier program compiled.
- [ ] `idl/lez_ecdsa.json` exists and contains an `instructions` entry
      named `verify` with args `message`, `signature`, `expected_signer`.
- [ ] `idl/lez_counter.json` deleted.

**Verification:**
- [ ] `jq '.instructions[].name' idl/lez_ecdsa.json` prints `"verify"`.

**Dependencies:** Task 5.

**Files likely touched:**
- `idl/lez_ecdsa.json` (generated)
- delete `idl/lez_counter.json`

**Estimated scope:** Small.

---

### Checkpoint: Verifier core compiles
- [ ] `lgs build` green
- [ ] IDL regenerated and matches the new program
- [ ] Commit point — make a "verifier core compiles" commit

---

### Phase 3: Cost measurement

#### Task 7 — Test-vector generator

**Description:** Add `src/bin/gen_test_vector.rs` host binary. Uses
non-patched `k256` (host-side, no zkVM) to deterministically generate
one secp256k1 keypair from a fixed seed, sign a fixed message
(`b"hello redstone"` keccak'd), produce
`(message, signature_65, expected_address_20)`, and write to
`fixtures/test_vector.json` (gitignored).

**Acceptance criteria:**
- [ ] `cargo run --bin gen_test_vector` produces
      `fixtures/test_vector.json`.
- [ ] The recovered address inside the JSON matches an independent
      reference (`cast wallet address` over the same private key, OR
      derived inline via the `ethers` crate's `LocalWallet::address()`).
      Cross-check is asserted in the binary itself, panics on mismatch.
- [ ] Same seed → same output bytes across runs (determinism).

**Verification:**
- [ ] Run twice; diff `fixtures/test_vector.json` shows no changes.

**Dependencies:** Task 6.

**Files likely touched:**
- `src/bin/gen_test_vector.rs` (new)
- `Cargo.toml` (host-side dep on k256, possibly hex)

**Estimated scope:** Small.

---

#### Task 8 — Cost-measurement harness

**Description:** Add `src/bin/bench_verify.rs`. Loads the test vector,
invokes the guest via `risc0_zkvm::default_prover()`, prints
`total_cycles`, `user_cycles`, wall-clock prove time, and receipt size
in bytes. Run with `RISC0_DEV_MODE=0` to produce real numbers; if the
env var is `1`, the binary exits with a loud warning that numbers
would be fake.

**Acceptance criteria:**
- [ ] `RISC0_DEV_MODE=0 cargo run --release --bin bench_verify` prints
      all four numbers and `matches = true` for the synthetic vector.
- [ ] `RISC0_DEV_MODE=1 cargo run --release --bin bench_verify` prints
      a `WARN: dev mode = fake numbers` line and exits 1.
- [ ] Flipping one byte of `signature.r` in `fixtures/test_vector.json`
      → bench prints `matches = false` (does NOT crash).

**Verification:**
- [ ] All three scenarios above produce the expected output by hand.

**Dependencies:** Task 7.

**Files likely touched:**
- `src/bin/bench_verify.rs` (new)
- `Cargo.toml` (workspace dep on bincode for receipt-size measurement,
  if not already pulled)

**Estimated scope:** Small-Medium.

---

#### Task 9 — Record results in README

**Description:** Run the bench 5 times (mean + range), record numbers
in `README.md` under a "Results" section with: machine spec
(CPU/RAM), risc0-zkvm version, pinned spel commit SHA, total cycles
(mean), prove time (mean ± stddev), receipt size in bytes.

**Acceptance criteria:**
- [ ] `README.md` has a `## Results` section with the table.
- [ ] The pinned spel SHA is the actual commit `Cargo.lock` resolved
      (read from `cargo metadata`).

**Verification:**
- [ ] Numbers in README are reproducible: re-run bench, numbers should
      land within the recorded range.

**Dependencies:** Task 8.

**Files likely touched:**
- `README.md`

**Estimated scope:** Small.

---

### Checkpoint: Cost measured
- [ ] We have actual numbers in `README.md`
- [ ] Commit point — make a "cost measured" commit

---

### Phase 4: CI + push

#### Task 10 — CI workflow runs locally

**Description:** The CI file `.github/workflows/ci.yml` already drafted
covers fmt + clippy + cargo build. Validate it locally with `act` if
available, else dry-run the steps by hand.

**Acceptance criteria:**
- [ ] `cargo fmt --all --check` exits 0.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` exits 0.
- [ ] `cargo build --workspace` exits 0.
- [ ] CI file is valid YAML (`yq . .github/workflows/ci.yml`).

**Verification:**
- [ ] Above commands all pass locally before pushing.

**Dependencies:** Task 9.

**Files likely touched:**
- `.github/workflows/ci.yml` (already exists; may need fixes if local
  runs reveal issues — e.g., toolchain pinning)

**Estimated scope:** Small.

---

#### Task 11 — Create remote + push (REQUIRES USER CONFIRMATION)

**Description:** Create `github.com/fryorcraken/lez-ecdsa` and push the
`main` branch. **Visibility (public/private) MUST be confirmed with
the user before `gh repo create`** — exposure is irreversible if wrong.

**Acceptance criteria:**
- [ ] User confirms public vs private.
- [ ] `gh repo create fryorcraken/lez-ecdsa --<visibility> --source . --remote origin` succeeds.
- [ ] `git push -u origin main` succeeds.
- [ ] CI run starts on GitHub Actions and the three jobs (fmt, clippy,
      build) eventually go green on the default branch.

**Verification:**
- [ ] `gh repo view fryorcraken/lez-ecdsa` shows the repo.
- [ ] `gh run list --limit 1` shows a CI run.
- [ ] `gh run view <id> --log-failed` is empty (or nonexistent if all green).

**Dependencies:** Task 10.

**Files likely touched:** None (remote ops only).

**Estimated scope:** Small.

---

### Checkpoint: Shipped
- [ ] Repo lives at `github.com/fryorcraken/lez-ecdsa`
- [ ] CI is green on `main`
- [ ] README has the cost numbers
- [ ] All SPEC.md acceptance criteria checked

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| spel API differs from lez-framework in non-trivial ways (e.g., return type signatures) | High | Task 2 surfaces this immediately; if compile fails, we fix per-error rather than guessing upfront. |
| risc0 patch tags don't match risc0-zkvm 3.0.5 ABI | Medium | Task 4 verifies patches resolve before any verifier code is written. If incompatible, bump to a matching tag found via `gh search` on risc0/* tags. |
| `lgs build` doesn't expose cycle counts | Low | Task 8 falls back to direct `risc0_zkvm::default_prover()` API — already planned. |
| CI rust toolchain doesn't match `rust-toolchain.toml` (1.92.0) | Medium | `dtolnay/rust-toolchain@stable` action picks up `rust-toolchain.toml` automatically; if not, hardcode `with: toolchain: 1.92.0`. |
| Public push exposes work prematurely | High | Task 11 explicitly asks the user. Default to `--private` when in doubt. |
| `lgs setup` fails (network, build error) | Medium | Currently being verified. If it fails persistently, fallback is `cargo build --workspace` directly without `lgs build`. |

## Open Questions (need user input before Task 11)

1. Repo visibility for `fryorcraken/lez-ecdsa`: public or private?

## Parallelization

This plan is mostly serial (each task depends on the prior). The only
genuinely parallel pair is:

- Task 7 (test-vector gen) and Task 10 (CI workflow validation) — both
  depend on Task 6 but are independent of each other.
