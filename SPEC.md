# SPEC — lez-ecdsa PoC

Companion to the approved plan at
`~/.claude/plans/snuggly-sauteeing-puppy.md`. The plan describes *what* to
build and *how*; this spec defines *done* in terms the implementation phase
will be checked against.

## 1. Objective

Build a single-instruction SPeL `#[lez_program]` on the Logos Execution
Zone that does **one** secp256k1 ECDSA `ecrecover` over a keccak256 hash,
and **measure its cost** (cycle count, prove time, receipt size) inside
the RISC Zero zkVM.

The PoC produces *data*, not a production component. The measurement is
the deliverable: it gates downstream decisions about RedStone integration
shape (push vs pull mode), multi-sig threshold cost, and proposed SPeL
framework extensions.

**Target reader of the output:** the project owner and any future
contributor scoping RedStone-on-LEZ work. The bench output + a short
results note are the primary artifacts a reader needs.

## 2. Commands

After scaffolding (`logos-scaffold new lez-ecdsa --template lez-framework`)
and the SPeL HEAD upgrade (see §3 below), these are the supported commands:

| Command | Purpose |
|---|---|
| `lgs build` | Build the workspace (guest + host). Must pass on every change. |
| `cargo run --release --bin gen_test_vector` | Produce a deterministic synthetic test vector to `fixtures/test_vector.json` (gitignored). |
| `cargo run --release --bin bench_verify` | Run the verifier under the RISC Zero prover; print cycles, prove time, receipt size. **Must be run with `RISC0_DEV_MODE=0`** to produce real numbers. |
| `lgs prove` | If the CLI exposes equivalent metrics, prefer this over `bench_verify`. Decided at impl time. |
| `cargo fmt --all --check` | Format check — runs in CI on every push/PR. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Lint — runs in CI on every push/PR. |

## 3. Project structure

Per the plan, plus one upgrade step:

```
lez-ecdsa/
├── methods/guest/src/bin/lez_ecdsa.rs    # #[lez_program] + #[instruction]
├── methods/guest/Cargo.toml              # [patch.crates-io] for risc0 k256/keccak
├── lez_ecdsa_core/src/lib.rs             # VerifyInput / VerifyOutput
├── examples/src/bin/gen_test_vector.rs   # host-side test vector generator
├── examples/src/bin/bench_verify.rs      # cost harness
├── fixtures/                             # gitignored — test vectors live here
├── SPEC.md                               # this file
├── README.md                             # results table goes here after first run
└── .gitignore
```

**SPeL upgrade after install (added requirement):** after the scaffold
generates the workspace, replace the pinned `spel` / `spel_framework`
crates.io dependency with a git dependency pointing at
`https://github.com/logos-co/spel` `branch = "main"`. This is to track
upstream during the PoC. Pin the resolved commit in `Cargo.lock` and
record the SHA in the README results note so measurements stay
reproducible.

## 4. Code style

- **Language:** Rust, edition matches whatever `lez-framework` template
  pins. Do not bump edition.
- **Formatting:** `cargo fmt` — run before every commit.
- **Lints:** `cargo clippy --workspace -- -D warnings`. No `#[allow]`
  pragmas without an inline `// reason: ...` comment.
- **No comments unless they explain WHY** something non-obvious is
  happening (per repo CLAUDE.md guidance). Don't narrate what the code
  does.
- **No new abstractions in the PoC.** One instruction, two structs, two
  binaries. If a helper is tempting, inline it.
- **Synthetic test data is deterministic.** Test-vector generator uses a
  fixed seed; the same seed produces the same `(message, sig, address)`
  on every run.

## 5. Testing strategy

1. **Sanity test (host):** `gen_test_vector` cross-checks the recovered
   address against an independent reference (e.g. `cast wallet address`
   over the same private key, or a second ECDSA library). If they
   disagree, the vector is broken — abort.
2. **Happy path (guest):** `bench_verify` runs the verifier on the
   synthetic vector → expects `matches = true`, recovered address ==
   expected address.
3. **Negative path (guest):** flip one byte of `signature.r` in the
   vector → expects either `matches = false` (with a different
   recovered address) or an `InvalidInput` error from the verifier.
   This proves the kernel isn't trivially passing.
4. **CI** runs `cargo fmt --check` and `cargo clippy -D warnings` on
   every push and PR. `cargo build --workspace` runs as a third job.
   `lgs build` and `bench_verify` are NOT in CI yet — `lgs setup` is
   too heavy for a free-tier runner; gating on them lands once we
   know how `lgs` behaves. CI lives in `.github/workflows/ci.yml`.

Test vectors are **synthetic only** in this PoC. Captured RedStone
signatures are deferred until the RedStone payload-parsing milestone
(see §7).

## 6. Boundaries

### Always
- **Run `lgs build` and `bench_verify` (with `RISC0_DEV_MODE=0`) before
  claiming any task is done.** No green checkmarks on partial work.
- Surface assumptions explicitly when picking SPeL macro syntax,
  `risc0_zkvm` patch tags, or `lgs` CLI flag names — those are the
  three points the plan flagged as needing first-touch confirmation.
- When `RISC0_DEV_MODE=1` is used (e.g. for a fast smoke run), the
  bench output and any commit message referencing numbers must say so
  prominently. Dev-mode numbers are not real measurements.

### Ask first
- Any command that mutates shared state: `lgs deploy`, `git push`,
  `gh repo create`, publishing, Cargo `cargo publish`, etc. The
  initial `git init` + first push to `fryorcraken/lez-ecdsa` is
  authorized; subsequent pushes after the initial one are not
  pre-authorized.
- Bumping the `risc0_zkvm` major version, or bumping the SPeL git
  dependency to a different branch / fork.

### Never
- Commit raw private keys, signing secrets, mnemonic seeds, or
  unredacted production RedStone signer keys. Test signing keys
  must be either deterministic-from-seed in source (so the seed is
  the only "secret" and it's a constant) or loaded from a gitignored
  `fixtures/` directory.
- Bypass git hooks (`--no-verify`) or git signing flags
  (`--no-gpg-sign`, `-c commit.gpgsign=false`) without explicit user
  approval. If a hook fails, fix the cause.
- Silently use `RISC0_DEV_MODE=1` for any output reported as a
  measurement.
- Refactor or "clean up" code outside the PoC's narrow surface area.

## 7. Non-goals (deferred, explicit)

The plan's "Not doing" list is canonical; restating here so a reader of
the spec alone sees the boundary:

- **RedStone payload parsing.** Synthetic test vectors only.
- **Multi-sig threshold (M-of-N).** Exactly one ECDSA verify.
- **Push mode** (write to PDA). Instruction returns a value, doesn't
  mutate accounts.
- **Pull mode** (output for tail call). The output struct exists, no
  tail-call wiring.
- **SPeL framework extensions** for push/pull attributes.
- **Deployment** (`lgs deploy`). Local proving only.
- **`lgs build` / `bench_verify` in CI.** Lint and `cargo build` only;
  proving stays local.
- **Cost target / pass-fail threshold.** This PoC measures and reports;
  acceptability is decided in a follow-up after the numbers exist.

## 8. Acceptance criteria (testable "done")

The PoC is complete when **all** of these are demonstrably true:

- [ ] `lgs build` succeeds from a clean checkout.
- [ ] `cargo run --release --bin gen_test_vector` produces a vector
      whose recovered address matches an independent reference
      (`cast wallet address` or equivalent).
- [ ] `cargo run --release --bin bench_verify` with `RISC0_DEV_MODE=0`
      prints `matches = true` and emits three numbers: total cycles,
      wall-clock prove time, receipt size in bytes.
- [ ] Flipping one byte of the signature in the test vector flips the
      result to `matches = false` (or `InvalidInput`).
- [ ] `README.md` contains a results table with: cycles, prove time
      (mean of 5 runs, machine spec noted), receipt size, the
      `risc0_zkvm` version, and the pinned SPeL commit SHA.
- [ ] No private keys or secrets are committed; `fixtures/` is in
      `.gitignore`.
- [ ] The repo lives at `github.com/fryorcraken/lez-ecdsa` with at
      least the SPEC and a runnable scaffold pushed.
- [ ] `.github/workflows/ci.yml` exists and the fmt + clippy + build
      jobs are green on the default branch.

## 9. Reference URLs

See the plan file for the full link list. Key ones:

- logos-scaffold: https://github.com/logos-co/logos-scaffold
- spel: https://github.com/logos-co/spel (track `main` branch during PoC)
- LEZ: https://github.com/logos-blockchain/logos-execution-zone
- RISC Zero ECDSA example: https://github.com/risc0/risc0/tree/main/examples/ecdsa
- RISC Zero precompiles: https://dev.risczero.com/api/zkvm/precompiles
