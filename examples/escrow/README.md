# Escrow Example

A timelock escrow program with a deliberately seeded boundary bug, used as a
basics-level showcase for Crucible.

## Layout

```
escrow/
├── programs/escrow/   # The program under test (Anchor)
└── fuzz/escrow/       # The fuzz harness (fixture, actions, invariant)
```

## Program

A vault PDA (one per depositor) holds funds with two unlock paths:

- `withdraw` — depositor recovers funds *before* `unlock_slot`
- `claim`    — beneficiary takes funds *at or after* `unlock_slot`

The seeded bug is in `withdraw`'s slot check: it uses `<=` instead of `<`, so at
exactly `slot == unlock_slot` the depositor can still drain the vault even
though it's already claimable.

## Harness

Four actions: `action_deposit`, `action_withdraw`, `action_claim`,
`action_advance_slots`. One invariant: every successful withdraw must have
happened strictly before `unlock_slot`. The bug fires when the fuzzer hits the
sequence `deposit → advance to unlock_slot → withdraw`, which the typed-action
mutator finds in seconds thanks to range-boundary biasing.

## Prerequisites

- `crucible` CLI on your `PATH` (`cargo install --path crates/crucible-fuzz-cli`
  from the repo root).
- Solana platform-tools **v1.52 or later** — earlier versions ship rustc 1.84
  which can't build the dependency tree (edition2024). If `cargo-build-sbf`
  reports a `feature edition2024 is required` error, pass
  `--tools-version v1.52`.

## Build & run

From this directory:

```bash
# 1. Build the program → target/deploy/escrow.so
cargo build-sbf --tools-version v1.52 --manifest-path programs/escrow/Cargo.toml

# 2. Run the fuzzer for 60 seconds in release mode
crucible run escrow invariant_escrow --release --timeout 60
```

You should see crashes recorded almost immediately, with a trace like:

```
1. deposit(amount=270661) -> OK
2. advance_slots(slots=10) -> OK
3. withdraw(amount=257) -> OK [VIOLATION]
[FUZZ_FINDING] withdraw at slot 10 should have been rejected (unlock_slot = 10)
```

## Inspect & replay crashes

List crashes:

```bash
crucible list escrow invariant_escrow
```

Replay a specific one with full per-action output:

```bash
crucible show escrow invariant_escrow <crash-id>
```

Minimize a crash to its smallest reproducer:

```bash
crucible tmin escrow invariant_escrow <crash-id>
```

## Fix the bug (try it yourself)

In `programs/escrow/src/lib.rs`, change the `withdraw` slot check from `<=` to
`<`. Rebuild the program (`cargo build-sbf …`) and rerun the fuzzer — it should
now run for the full timeout without finding any crash.
