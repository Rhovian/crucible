# Staking Example

A reward-bearing staking pool with seeded accounting bugs, used as the
mid-complexity Crucible showcase. Demonstrates multi-user fixtures, time
control, range constraints, both `#[crucible_fuzz]` (stateless single-action)
and `#[invariant_test]` (stateful action-sequence) modes, and a batched
single-transaction variant.

## Layout

```
staking/
├── programs/staking/   # The staking-pool program (Anchor)
└── fuzz/staking/       # The fuzz harness (fixtures, actions, tests)
```

## Program

A pool tracks `total_staked` and an accumulator-style `accumulated_rewards_per_share`.
Users stake/unstake/claim against the pool. The program contains classic
reward-debt accounting bugs (e.g. claimable rewards updating before share-price
update, missing pending-payout calculation on stake) that the fuzzer surfaces
as invariant violations.

## Harness

Three test functions are exposed as Cargo features (one feature == one test):

| Feature                  | Kind             | What it does                                                                       |
|--------------------------|------------------|------------------------------------------------------------------------------------|
| `fuzz_single_stake`      | `#[crucible_fuzz]`   | Stateless: one random `stake(amount)` per iteration, checks the user's share-of-pool invariant. |
| `invariant_fuzz`         | `#[invariant_test]`  | Stateful: random action sequences over `stake / unstake / claim / advance_slots`; per-user stake-time accounting invariant runs after every action. |
| `invariant_fuzz_batched` | `#[invariant_test]`  | Same actions but committed in batched single-transaction mode for higher throughput. |

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
# 1. Build the program → target/deploy/staking.so
cargo build-sbf --tools-version v1.52 --manifest-path programs/staking/Cargo.toml

# 2. Run a test (pick one of the features above)
crucible run staking invariant_fuzz --release --timeout 60
crucible run staking fuzz_single_stake --release --timeout 60
crucible run staking invariant_fuzz_batched --release --timeout 60 --stateful
```

A 20-second run of `invariant_fuzz` typically lands hundreds of crashes
matching the seeded reward-accounting bugs, e.g.:

```
[FUZZ_FINDING] User stake-time 0 but earned 1423304 vs expected 0
```

## Inspect & replay crashes

```bash
crucible list  staking <test_name>
crucible show  staking <test_name> <crash-id>
crucible tmin  staking <test_name> <crash-id>
```

Crashes land under `crashes/<test_name>/`.

## Modes

- `--release` — optimized build (recommended for any non-trivial run).
- `--stateful` — keep a coverage-indexed pool of live program states and apply
  one mutated action at a time. Roughly an order-of-magnitude throughput gain
  over the default stateless mode.
- `--timeout <seconds>` — stop the fuzzer after the given wall-clock time.
- `--coverage` — emit an LCOV report on exit. Combine with `genhtml` for an
  HTML view.

See the top-level [`docs/`](../../docs/) for full CLI and harness reference.
