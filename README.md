# soroban-financial-math

[![CI](https://github.com/Ibawa12/soroban_financialmath_crates/actions/workflows/ci.yml/badge.svg)](https://github.com/Ibawa12/soroban_financialmath_crates/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

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

## The problem this solves in the Soroban ecosystem

Soroban is still early. There is no equivalent yet of what
[OpenZeppelin](https://www.openzeppelin.com/) or
[Solmate](https://github.com/transmissions11/solmate) became for EVM
chains — a small set of primitives that the ecosystem converges on instead
of every team writing (and auditing, and re-auditing) its own version. Fixed-
point math is one of the clearest places that gap shows up:

- **Every AMM, lending market, and yield vault needs it**, and right now
  each one is reinventing scale conversion, checked multiplication, and
  overflow handling independently — with independently-varying levels of
  rigor, and independently-paid-for audits of code that does the same
  thing every other protocol also had audited.
- **Inconsistent precision between protocols is a composability hazard.**
  If protocol A prices assets at 7-decimal precision and protocol B at
  18-decimal, any contract that reads a price from one and feeds it into
  the other has to get that conversion exactly right, every time, with no
  shared reference implementation to check against.
- **Silent overflow/rounding bugs in money math are the single most
  expensive category of smart-contract bug** industry-wide. A shared,
  independently-reviewed library that makes overflow a hard, typed error
  instead of a wrapped value or a rounding surprise reduces that risk
  ecosystem-wide, not just for whoever adopts it first.

`soroban-financial-math` aims to be that shared layer for the Soroban
ecosystem specifically: a `no_std`, dependency-light, `i128`-based
fixed-point library that any Soroban contract — an AMM, a lending
protocol, a payments/streaming app, a stablecoin — can build on instead of
rolling its own, with the algorithmic work (and its review) done once in a
place the whole ecosystem can point an audit at.

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

```rust
use soroban_financial_math::fixed_point::Wad;

let price = Wad::from_scaled(1_500_000, 6)?;  // 1.5, expressed at 6 decimals
let quantity = Wad::from_raw(2 * soroban_financial_math::fixed_point::WAD); // 2.0

let notional = price.checked_mul(quantity)?; // checked, no silent overflow
```

Or as a standalone deployed contract, invoked like any other Soroban
contract:

```rust
let client = FinancialMathContractClient::new(&env, &contract_id);
let scaled = client.scale(&1_000_000, &6, &18); // USDC-style 6dp -> 18dp
```

## Open Issues

These four functions are the primary place external contributors can add
value right now. Each has a full doc-comment in
[`src/algorithms.rs`](src/algorithms.rs) explaining the target algorithm,
required range-reduction steps, and exact error conditions — read that
before opening a PR.

| Function | File | Suggested difficulty | What's involved |
|---|---|---|---|
| `sqrt` | `src/algorithms.rs` | **Medium** | Newton-Raphson iteration over the `Wad` domain |
| `geometric_mean` | `src/algorithms.rs` | **Medium** | Mostly wiring: composes `ln`/`exp`, plus a small-`n` fast path |
| `ln` | `src/algorithms.rs` | **High** | Range reduction (`ln(x) = k·ln(2) + ln(m)`) + Taylor series + error bounding |
| `exp` | `src/algorithms.rs` | **High** | Range reduction (`exp(x) = 2^k · exp(r)`) + Taylor series + overflow guarding |

Each has a matching `#[ignore]`d integration test in
[`src/test.rs`](src/test.rs) sketching the expected input/output — remove the
`#[ignore]` and fill in the assertions once your implementation lands.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full workflow, coding
conventions, and PR checklist.

## Safety notes for reviewers

- `overflow-checks = true` is set explicitly in the `[profile.release]`
  section of `Cargo.toml` and must never be removed — this crate's entire
  safety argument rests on overflow being a hard error, not a silent wrap.
- No `unsafe` is used or expected to be needed anywhere in this crate.
- Floating-point types (`f32`/`f64`) must never appear in `fixed_point.rs`
  or `algorithms.rs` — see [Why this design](#why-this-design) above.

## Security

Found a suspected vulnerability? Please don't open a public issue — see
[SECURITY.md](SECURITY.md) for the private reporting process, response
targets, and what's in/out of scope.

## License

Apache-2.0 — see [LICENSE](LICENSE).
