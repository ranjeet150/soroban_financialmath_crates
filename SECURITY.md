# Security Policy

`soroban-financial-math` is math that other Soroban contracts build
financial logic on top of. A bug here — a domain check that can be
bypassed, an overflow that isn't actually caught, a range-reduction step
that silently loses precision at a boundary — doesn't stay contained to
this crate; it propagates into every contract that depends on it. Please
report suspected vulnerabilities privately rather than as a public issue.

## Supported versions

This project has not yet had a tagged `1.0` release. Until then, only the
latest commit on the `master` branch is supported — there are no older
version branches receiving security fixes. Once tagged releases begin, this
section will be updated with a version support table.

| Version | Supported |
|---|---|
| `master` (latest commit) | ✅ |
| Anything else | ❌ |

## Project status: pre-audit

This crate has **not** undergone an external security audit. The
fixed-point scaling layer (`Wad`/`Rad`) is implemented and tested; the
approximation algorithms (`sqrt`, `ln`, `exp`, `geometric_mean`) are
currently unimplemented stubs (see the [README](README.md#status)). Do not
use this crate to secure real, non-trivial value until it has both a
complete implementation and an independent audit. If you're evaluating it
for that purpose, please open a discussion issue first — we'd genuinely
like to know before it happens, not after.

## Reporting a vulnerability

**Please do not open a public GitHub issue for a suspected vulnerability.**

Use GitHub's private vulnerability reporting instead:

1. Go to the [Security tab](../../security) of this repository.
2. Click **"Report a vulnerability"** to open a private advisory draft.
3. Include as much of the following as you can:
   - The affected function(s) and file(s)
   - A minimal reproduction (concrete input values, expected vs. actual
     behavior)
   - Your assessment of impact — e.g. "silent overflow producing an
     incorrect `Wad` value" vs. "panic/DoS" vs. "incorrect rounding
     direction"
   - Whether you believe it's exploitable by an untrusted caller of a
     contract that depends on this crate, or only relevant to a
     misusing/careless integrator

If private vulnerability reporting isn't available or accessible to you,
open an issue titled only `Security contact needed` with no technical
details, and a maintainer will follow up with a private channel.

## What counts as in-scope

- Any input for which a documented function returns an incorrect result
  without returning the corresponding `MathError` (i.e. a silent
  correctness bug, not just a missing feature)
- Any input for which overflow, underflow, or division-by-zero is not
  caught and surfaced as a `MathError`
- Any panic reachable from a documented public API with a well-formed
  input (as opposed to a documented `todo!()` in an intentionally
  unimplemented function — those are tracked as open issues, not
  vulnerabilities)

## What's out of scope

- The known-unimplemented `todo!()` bodies in `src/algorithms.rs` — these
  are tracked as [open issues](../../issues), not vulnerabilities, until
  an implementation lands
- Findings that require the caller to already control values that would
  make the operation meaningless regardless of this crate (e.g. a
  contract that doesn't validate its own inputs before calling into this
  crate)

## Response process

We aim to acknowledge a new report within **5 business days** and to
provide an initial assessment (confirmed / not a vulnerability / needs more
information) within **10 business days**. Given the project's current
volunteer-maintained, pre-1.0 stage, please treat these as targets rather
than guarantees — if you haven't heard back in that window, following up
on the same advisory thread is welcome and expected.

We follow coordinated disclosure: please give us a reasonable window to
land and release a fix before any public disclosure. We'll credit
reporters (by name or handle, or anonymously if you prefer) in the fix's
release notes unless you ask us not to.
