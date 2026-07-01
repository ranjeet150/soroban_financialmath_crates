# Contributing to soroban-financial-math

Thanks for considering a contribution. This crate is deliberately structured
so that meaningful, self-contained pieces of work — implementing one
approximation algorithm, hardening one edge case — can be picked up without
first understanding the whole codebase. This document explains how.

## Before you start

1. **Check for an existing issue or claim.** If you're picking up one of the
   algorithms listed in the [README's Open Issues table](README.md#open-issues),
   comment on (or open) the corresponding GitHub issue first so two people
   don't duplicate the same work.
2. **Read the doc-comment before writing code.** Every `todo!()` function in
   [`src/algorithms.rs`](src/algorithms.rs) has a doc-comment specifying the
   target algorithm, the required range-reduction steps, and the exact error
   conditions the function must return. That doc-comment is the spec — if
   you think it's wrong or ambiguous, open a discussion/issue before
   implementing something different, rather than silently diverging from it.
3. **Small PRs over big ones.** One algorithm, one bug fix, or one test
   improvement per PR. A PR that implements `sqrt` and also refactors
   `fixed_point.rs` and also fixes an unrelated typo is three PRs wearing a
   trenchcoat, and much harder to review and revert independently.

## Development setup

```bash
git clone <your-fork-url>
cd soroban-financial-math
rustup target add wasm32-unknown-unknown   # rust-toolchain.toml pins this
cargo test                                  # pure-Rust + Env-based tests
cargo build --target wasm32-unknown-unknown --release   # the deployable artifact
```

No other tooling is required to get started. If you also want to exercise
the contract via the Soroban CLI (`stellar contract deploy`, etc.), see the
[Soroban CLI docs](https://developers.stellar.org/docs/tools/developer-tools#cli)
— that's optional for algorithm-only contributions.

## Coding conventions

