# Contributing to soroban-financial-math

Thanks for considering a contribution. This crate is deliberately structured
so that meaningful, self-contained pieces of work — implementing one
approximation algorithm, hardening one edge case — can be picked up without
first understanding the whole codebase. This document explains how.

## Before you start

1. **Check for an existing issue or claim.** If you're picking up one of the
   items in the [README's Known Limitations section](README.md#known-limitations)
   — e.g. tightening `ln`/`exp` precision, or a full `mulDiv` for
   `checked_div` — comment on (or open) the corresponding GitHub issue first
   so two people don't duplicate the same work.
2. **Read the doc-comment before writing code.** Every function in
   [`src/algorithms.rs`](src/algorithms.rs) has a doc-comment specifying the
   algorithm it implements, the required range-reduction steps, and the exact
   error conditions the function must return. That doc-comment is the spec —
   if you think it's wrong, ambiguous, or the implementation has drifted from
   it, open a discussion/issue before changing behavior, rather than silently
   diverging from what's documented.
3. **Small PRs over big ones.** One algorithm, one bug fix, or one test
   improvement per PR. A PR that touches `sqrt` and also refactors
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

CI (`.github/workflows/ci.yml`) enforces `cargo fmt --check`, `cargo clippy
-- -D warnings`, `cargo test`, and the `wasm32-unknown-unknown` release
build on every push and PR. The rest of these are enforced in review, so
please self-check before requesting it:

- **No floating point, anywhere.** Not `f32`, not `f64`, not as an
  intermediate step "just to check the answer" left in committed code.
  Soroban's determinism guarantee depends on every validator computing the
  identical result, and float rounding is not guaranteed identical across
  implementations. If you need floats to sanity-check your fixed-point
  implementation while developing, do that in a scratch script or a
  `#[cfg(test)]`-only helper that never runs in the Wasm build — not in the
  algorithm body itself.
- **Every fallible path returns `Result<T, MathError>`.** Never `panic!`,
  `unwrap()`, or `expect()` inside `fixed_point.rs` or `algorithms.rs`
  (the `#[contract]` wrapper in `lib.rs` is the one place `unwrap_or_else`
  + `panic_with_error!` is expected, since that's the Soroban-idiomatic way
  to surface an error at the host boundary). If your algorithm hits a new
  failure mode not already covered by [`MathError`](src/errors.rs), add a
  variant rather than overloading an existing one — see that file's
  doc-comments for the existing taxonomy and pick a discriminant one higher
  than the current maximum.
- **Checked arithmetic only.** Use `checked_add`/`checked_mul`/etc. (or the
  `Wad`/`Rad` checked methods already provided) — never bare `+`/`*` on the
  underlying `i128`, even when you're confident it can't overflow. The
  confidence is usually wrong at some input boundary, and the checked form
  costs nothing at this precision.
- **Match the existing doc-comment style.** Public functions document their
  algorithm/formula, not just their signature, and enumerate every
  `# Errors` variant they can return. Look at `sqrt`/`ln`/`exp` in
  `algorithms.rs` for the expected level of detail.
- **No unrelated formatting churn.** Run `cargo fmt` on files you actually
  touched; don't reformat files you didn't otherwise change.

## Testing requirements

- Any algorithm change needs unit tests in the same file
  (`#[cfg(test)] mod tests` at the bottom of `algorithms.rs`, following the
  existing pattern) covering: a known-good value, the domain boundary
  (e.g. `ln(x)` at `x` just above zero), and every documented error
  condition.
- Add or update the matching integration test in
  [`src/test.rs`](src/test.rs) exercising the change through the host
  contract boundary, not just the pure-Rust unit test.
- Run `cargo test` (not just `cargo check`) before opening a PR. All
  existing tests must still pass; don't loosen or delete a test to make
  your change pass unless you can explain why the old assertion was wrong.

## PR checklist

- [ ] Linked to (or created) the tracking issue
- [ ] Implementation matches the doc-comment's specified algorithm and
      error conditions, or the doc-comment was updated to match a
      deliberate, discussed deviation
- [ ] New/updated unit tests in `algorithms.rs`, covering domain boundaries
      and error paths
- [ ] Corresponding integration test added/updated in `src/test.rs`
- [ ] `cargo test` passes locally
- [ ] `cargo build --target wasm32-unknown-unknown --release` succeeds
- [ ] No floating-point types introduced
- [ ] No new `unwrap()`/`expect()`/`panic!()` outside the `#[contract]`
      wrapper in `lib.rs`

## Commit / PR style

- Commit messages: imperative mood, explain *why* over *what* where it's
  not obvious from the diff (e.g. "Use Newton-Raphson seed from
  leading_zeros to cut iteration count" rather than "update sqrt").
- Keep the PR description focused on: what algorithm/bug this addresses,
  what testing you did, and any deliberate deviation from the doc-comment
  spec.

## Getting help

If you're unsure about an approach before investing the time to implement
it, open a draft PR or a discussion issue describing your plan — that's
much cheaper for everyone than a full implementation that needs to be
redesigned in review.
