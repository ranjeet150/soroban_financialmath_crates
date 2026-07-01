# soroban-financial-math

Fixed-point arithmetic and high-precision approximation algorithms for
[Soroban](https://developers.stellar.org/docs/build/smart-contracts/overview)
DeFi contracts, written in `no_std` Rust with explicit overflow handling
instead of floating-point math.

Soroban's Wasm host has no native float type — and even if it did, floating
point is not deterministic across validator implementations, so it can't be
used in consensus-critical code. Every DeFi contract that needs prices,
interest accrual, or share pricing ends up re-inventing scaled-integer math
from scratch, usually with less rigor than it deserves. This crate exists so
that work happens once, in one well-tested place, instead of once per
contract.

## Status

**Early-stage / actively looking for contributors.** The fixed-point scaling
layer ([`Wad`]/[`Rad`], see below) is complete, tested, and safe to build on
today. The four approximation algorithms (`sqrt`, `ln`, `exp`,
`geometric_mean`) are fully specified — signatures, domains, error
conditions, and the target algorithm are all documented in
[`src/algorithms.rs`](src/algorithms.rs) — but their bodies are open
`todo!()`s. See [Open Issues](#open-issues) below if you want to pick one up.

## Why this design

- **`i128`, never `u128`.** Interest, PnL, and rate calculations legitimately
  go negative. Modeling that with unsigned integers and manual sign-flag
  bookkeeping is exactly the kind of thing that produces subtle accounting
  bugs; a signed representation avoids the whole class.
- **Checked arithmetic everywhere.** Every operation returns
  `Result<T, MathError>` instead of panicking or silently wrapping. Callers
  decide how to surface a failure — a contract entrypoint might
  `panic_with_error!`, but a pure Rust caller in a test or another library
  can choose to retry, log, or degrade gracefully instead.
- **`Wad` (1e18) / `Rad` (1e36) scaling**, adapted from MakerDAO's
  Wad/Ray/Rad convention. We stop at a 36-decimal `Rad` rather than
  MakerDAO's 45-decimal one deliberately: a true 45-decimal value (an
  18-decimal `Wad` times a 27-decimal `Ray`) does not fit in `i128`
  (max ≈ 1.7e38). `Rad` here is exactly what an unreduced `Wad * Wad`
  product needs — nothing more — so multiplication never truncates
  mid-expression before the caller explicitly asks for it to.
- **Host-independent core.** [`fixed_point`](src/fixed_point.rs) and
  [`algorithms`](src/algorithms.rs) don't touch `soroban_sdk::Env` at all,
  so they're unit-testable with plain `#[test]` and reusable as an ordinary
  Rust dependency by any downstream contract. [`lib.rs`](src/lib.rs) adds a
  thin `#[contract]` wrapper on top so the crate can also be deployed and
  invoked directly as its own standalone contract.

## Module layout

```
src/
├── lib.rs          # #[contract] host wrapper — thin, delegates everything
├── errors.rs       # MathError: the crate's complete #[contracterror] taxonomy
├── fixed_point.rs  # Wad / Rad types, scale conversion — fully implemented
├── algorithms.rs   # sqrt / ln / exp / geometric_mean — spec'd, bodies open
└── test.rs         # Env-based contract-level integration test harness
```

## Quick start

```bash
# Run the full test suite (pure-Rust unit tests + Env-based integration tests)
cargo test

# Build the deployable contract Wasm
cargo build --target wasm32-unknown-unknown --release
```

## Usage

As a library dependency (the common case — most contracts will want the
Rust types directly rather than going through the host contract wrapper):

