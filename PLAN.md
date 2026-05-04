# PLAN — lez-signature-bench

Companion to `SPEC.md`. Five phases, sequenced. Each phase has a
verification gate; do not advance to the next phase until the gate
passes.

## Phase Overview

```
Phase 1 (spike) ──→ Phase 2 (rename + scaffold) ──→ Phase 3 (verifiers)
                                                           │
                                                           ▼
                                              Phase 4 (bench harness)
                                                           │
                                                           ▼
                                              Phase 5 (run + report)
```

| Phase | Scope | Gate |
|---|---|---|
| 1 | Patch-availability spike | Decision: which schemes survive into the matrix |
| 2 | Repo rename, workspace rename, scaffold | `cargo build --workspace` green |
| 3 | Per-scheme verifier modules + guest binaries | Host tests + executor sanity green per scheme |
| 4 | Bench harness (single + matrix mode) | Single-point bench succeeds end-to-end for one scheme |
| 5 | Run the matrix, write README + decision note | All success criteria from SPEC §8 ticked |

## Phase 1 — Patch-availability spike

**Why first:** the matrix shape is unknown until we confirm whether
RISC0 patches exist for Ed25519 and P-256, and whether k256's Schnorr
submodule actually benefits from the secp256k1 precompile. Building
guest code before this is wasted work if a scheme drops out.

**Tasks**

- [ ] **1.1** Check RISC Zero's RustCrypto-elliptic-curves and related
      forks for current stable tags covering `p256` and `curve25519-dalek`.
      - Acceptance: a tag (or commit SHA) is recorded for each, OR the
        scheme is marked "no patch available, drop from matrix."
      - Verify: a 5-line `Cargo.toml` snippet using each patched crate
        compiles for the `riscv32im-risc0-zkvm-elf` target in a
        throwaway `cargo new` workspace.
      - Files: scratch workspace; do not modify the bench repo yet.

- [ ] **1.2** Confirm whether k256's `schnorr` module uses the same
      curve arithmetic that RISC0 patches.
      - Acceptance: documented evidence (link to source, or a small
        cycle-count diff between ECDSA and Schnorr in a smoke guest)
        that Schnorr verify is accelerated.
      - Verify: brief writeup pasted into SPEC §10 Open Questions
        (resolving the Schnorr question), or matrix updated to drop
        Schnorr if it falls back to soft.
      - Files: SPEC.md (§10 only).

- [ ] **1.3** Update SPEC §1 (schemes in scope) and §10 (open
      questions) based on 1.1 and 1.2 findings.
      - Acceptance: SPEC reflects the actual matrix that will be built.
      - Verify: SPEC.md diff reviewed by user before Phase 2.

**Phase 1 gate:** the spec lists exactly the schemes Phase 3 will
implement, with patch sources pinned.

## Phase 2 — Repo rename, workspace rename, scaffold

**Why second:** infrastructure churn is cheaper before code grows.
Doing it after Phase 3 means renaming dozens of imports.

**Tasks**

- [ ] **2.1** GitHub repo rename `lez-ecdsa` → `lez-signature-bench`.
      - Acceptance: `gh repo view fryorcraken/lez-signature-bench`
        succeeds; old URL redirects.
      - Verify: `git remote -v` updated locally; one push lands.
      - **Ask first** — visible action; do not run without confirmation.
      - Files: none in repo (GitHub-side + local `.git/config`).

- [ ] **2.2** Workspace rename in Cargo: `lez-ecdsa` →
      `lez-signature-bench`, `lez_ecdsa` → `lez_signature_bench`,
      `LEZ_ECDSA_ELF` references removed.
      - Acceptance: `cargo build --workspace` succeeds with zero
        references to the old name.
      - Verify: `rg -i 'lez.?ecdsa'` returns nothing except SPEC §0
        historical note.
      - Files: `Cargo.toml`, `methods/Cargo.toml`,
        `methods/guest/Cargo.toml`, `src/lib.rs`,
        `methods/guest/src/bin/*.rs`, `README.md`.

- [ ] **2.3** Generalize `VerifyInput` with a `Scheme` enum and
      per-scheme fixtures builder in `src/lib.rs`. Old `make_test_vector`
      becomes one branch.
      - Acceptance: `make_test_vector(Scheme::EcdsaSecp256k1, n=1)`
        returns the same bytes the old function produced (regression).
      - Verify: a `cargo test` in `src/lib.rs` asserts byte-for-byte
        equality against the previous output for the (ECDSA, n=1) case.
      - Files: `src/lib.rs`.

- [ ] **2.4** Replace existing `methods/guest/src/bin/lez_ecdsa.rs`
      with `ecdsa_secp256k1.rs` matching the SPEC §5 thin-shell pattern.
      Add `noop.rs`. Other scheme binaries deferred to Phase 3.
      - Acceptance: workspace builds; both binaries embed via
        `risc0-build`.
      - Verify: `cargo build --workspace --release` green; ELF symbols
        list both binaries.
      - Files: `methods/guest/src/bin/`, `methods/build.rs` if needed.

- [ ] **2.5** Add the AI-generated / not-for-mainnet warning to
      `README.md` (top of the file, before any other content).
      - Acceptance: README opens with a clearly visible warning that
        (a) this codebase is AI-generated as an indication of signature
        verification cost on RISC0 / LEZ, and (b) it MUST NOT be used
        in any mainnet program.
      - Verify: the warning is the first thing a reader sees on the
        repo's GitHub landing page.
      - Files: `README.md`.

**Phase 2 gate:** `cargo build --workspace --release` is green; the
old `run_private` still runs against the new `ecdsa_secp256k1` binary
(temporary; replaced in Phase 4); the README warning is in place
before any code beyond the existing baseline is added.

## Phase 3 — Per-scheme verifier modules + guest binaries

One scheme at a time. Each scheme is a complete vertical slice:
fixture builder, verifier module, guest binary, host tests.

**Tasks** (one per scheme; order = simplest first to validate the
pattern)

- [ ] **3.1 ECDSA secp256k1** (mostly a port of existing code)
      - Acceptance: `verify_all` accepts fixtures from
        `gen_test_vectors --scheme ecdsa-secp256k1 --n {1,3}`; one-byte
        flip → `Err`; guest binary runs in executor mode.
      - Verify: `cargo test --workspace` (positive + negative) +
        executor sanity test green.
      - Files: `methods/guest/src/verifier/ecdsa_k256.rs`,
        `methods/guest/src/bin/ecdsa_secp256k1.rs`, `src/lib.rs`
        (fixtures), tests.

- [ ] **3.2 Schnorr secp256k1 (BIP-340)** (only if Phase 1.2 cleared it)
      - Acceptance: same pattern as 3.1.
      - Verify: same as 3.1.
      - Files: `verifier/schnorr_k256.rs`, `bin/schnorr_secp256k1.rs`,
        `src/lib.rs`, tests.

- [ ] **3.3 Ed25519** (only if Phase 1.1 cleared it)
      - Acceptance: same pattern; uses RISC0 `curve25519-dalek` fork.
      - Verify: same as 3.1.
      - Files: `verifier/ed25519.rs`, `bin/ed25519.rs`, `src/lib.rs`,
        tests.

- [ ] **3.4 P-256 ECDSA** (only if Phase 1.1 cleared it)
      - Acceptance: same pattern; uses RISC0 `p256` fork.
      - Verify: same as 3.1.
      - Files: `verifier/ecdsa_p256.rs`, `bin/ecdsa_p256.rs`,
        `src/lib.rs`, tests.

**Phase 3 gate:** every in-scope scheme has positive + negative host
tests passing and an executor-mode guest sanity test passing. No
prove-time runs yet.

## Phase 4 — Bench harness

Generalize `run_private.rs` into `bench.rs`. Single-point mode first;
matrix mode second.

**Tasks**

- [ ] **4.1** `gen_test_vectors` binary: `--scheme <s> --n <n>` writes
      a deterministic JSON fixture to `fixtures/<scheme>_n<n>.json`.
      - Acceptance: a fresh run produces a byte-identical file twice.
      - Verify: `diff <(run twice)` is empty for every (scheme, N).
      - Files: `src/bin/gen_test_vectors.rs`.

- [ ] **4.2** `bench` binary, single-point mode: `--scheme <s> --n <n>
      --account-id <id>` selects the right ELF, builds the fixture,
      submits a real private TX, prints `cycles | prove_time |
      receipt_size | tx_e2e_time`.
      - Acceptance: succeeds end-to-end for `--scheme ecdsa-secp256k1
        --n 1` against the LEZ devnet.
      - Verify: stdout shows all four numbers; `RISC0_DEV_MODE=0`
        confirmed in output.
      - Files: `src/bin/bench.rs`, `src/lib.rs` (ELF lookup helper).

- [ ] **4.3** `bench --all` matrix mode: enumerates the full
      (scheme × N) matrix + noop baseline, runs each once, writes
      `results/results.json` and `results/README-snippet.md`.
      - Acceptance: a single invocation produces both files; row count
        equals matrix size; one run per row.
      - Verify: `bench --all` succeeds; output files validate as JSON
        and Markdown.
      - Files: `src/bin/bench.rs`, `results/` (gitignored).

- [ ] **4.4** Delete `src/bin/run_private.rs` (superseded by `bench`).
      - Acceptance: no references remain.
      - Verify: `rg run_private` returns nothing; build still green.
      - Files: deletion only.

**Phase 4 gate:** `bench --scheme ecdsa-secp256k1 --n 1` produces
real numbers for one scheme end-to-end; the matrix runner has been
exercised on at least two schemes successfully.

## Phase 5 — Run the matrix, write the report

**Tasks**

- [ ] **5.1** Run `bench --all` from a quiescent machine state (no
      heavy background processes). Capture the output.
      - Acceptance: every (scheme, N) point in the in-scope matrix
        produces a row; no skipped or errored rows.
      - Verify: `results/results.json` row count = matrix size; no
        `null`/`error` values in measurement columns.
      - Files: `results/results.json` (gitignored),
        `results/README-snippet.md`.

- [ ] **5.2** Reproducibility check: re-run `bench --all`. Confirm
      every measurement is within ±10% of the first run.
      - Acceptance: a small script (or eyeball) confirms ±10% bound on
        cycles, prove_time, receipt_size, tx_e2e.
      - Verify: numbers from run 1 vs run 2 documented in commit message
        for the README update.
      - Files: none committed; just verification.

- [ ] **5.3** Update `README.md`: results table (with machine spec,
      RISC0 version, LEZ version), decision note ("given a 5s / 30s /
      5min end-to-end TX budget, which (scheme, N) fits?"), pointers to
      SPEC + PLAN. Confirm the AI-generated / not-for-mainnet warning
      from 2.5 is still at the top of the file.
      - Acceptance: a reader unfamiliar with the repo can answer
        "which scheme should I use for N=3 under 30s?" from the README
        alone, AND sees the warning before anything else.
      - Verify: user reads the README cold and confirms.
      - Files: `README.md`.

- [ ] **5.4** Tick all SPEC §8 success criteria.
      - Acceptance: SPEC §8 checklist is fully ticked.
      - Verify: SPEC.md diff in the closing commit shows all `[x]`.
      - Files: `SPEC.md`.

**Phase 5 gate:** SPEC §8 fully ticked; README has the warning + table
+ decision note; CI green.

## Risks & mitigations

| Risk | Likelihood | Mitigation |
|---|---|---|
| RISC0 patches missing for Ed25519 or P-256 | Medium | Phase 1 catches early; matrix shrinks rather than blocks |
| k256 Schnorr falls back to soft path | Medium | Phase 1.2 catches early; either drop Schnorr or accept "soft" row |
| LEZ devnet flakes mid-matrix run | Low–Medium | Single-point mode allows resuming individual rows; matrix runner should tolerate per-row retry |
| 9 guest ELFs blow build time | Low | `risc0-build` caches; only changed binaries rebuild. Acceptable. |
| Prove-time numbers vary >10% run-to-run | Medium | Phase 5.2 explicitly checks; if fails, document variance and add a "median of N" run mode |
| Repo rename breaks CI references | Low | Update `.github/workflows/ci.yml` paths in Phase 2.2 |
| README warning gets buried by later edits | Low | Phase 5.3 explicitly re-confirms warning is at top of file before sign-off |

## Parallelizable work

Within Phase 3, scheme implementations 3.2 / 3.3 / 3.4 are independent
of each other (each has its own verifier module, guest binary, and
fixture). They can be done in any order or interleaved. They all
depend on 3.1 (which establishes the pattern).

Phase 1 spike tasks (1.1, 1.2) can run in parallel.

Everything else is sequential.

## Out of scope for this plan

Per SPEC §9: RedStone payload parsing, threshold crypto, batch verify,
N-sweep, BLS/RSA/post-quantum, multi-machine sweeps, deployment, cost
targets. Note as future work in commit messages if relevant; do not
implement.
