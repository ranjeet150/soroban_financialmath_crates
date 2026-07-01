//! Fixed-point scale primitives.
//!
//! Soroban has no native floating-point type (and using one would break
//! deterministic consensus across validators), so all DeFi math in this
//! workspace is expressed as scaled integers using a `Wad`/`Rad`
//! convention adapted from MakerDAO's, but bounded to fit `i128`:
//!
//! | Type  | Scale | Typical use                                          |
//! |-------|-------|-------------------------------------------------------|
//! | `Wad` | 1e18  | Token amounts, prices, general 18-decimal math        |
//! | `Rad` | 1e36  | An unreduced `Wad * Wad` product (18 + 18 decimals)    |
//!
//! `Rad` intentionally stops at 1e36 rather than MakerDAO's 1e45: a true
//! 45-decimal `Rad` (the product of an 18-decimal `Wad` and a 27-decimal
//! `Ray`) does not fit in `i128` (max ≈ 1.7e38), and this workspace treats
//! "never silently truncate/wrap on overflow" as a harder constraint than
//! bit-for-bit compatibility with MakerDAO's original scale choices. `Rad`
//! here exists purely so that a `Wad * Wad` multiplication has somewhere
//! to land *before* being explicitly reduced back down via [`Rad::to_wad`],
//! rather than truncating mid-expression.
//!
//! All arithmetic uses `i128` (never `u128`) so that intermediate values in
//! interest/PnL calculations can safely go negative, and every operation is
//! implemented with checked (`checked_*`) primitives that return
//! [`MathError`] instead of panicking or silently wrapping.

use crate::errors::MathError;

/// Scale of a [`Wad`]: 10^18.
pub const WAD: i128 = 1_000_000_000_000_000_000;

/// Scale of a [`Rad`]: 10^36 — see the module docs for why this differs
/// from MakerDAO's 1e45.
pub const RAD: i128 = 1_000_000_000_000_000_000_000_000_000_000_000_000;

/// Upper bound accepted by [`scale_to`] / [`scale_from`] — decimal places
/// beyond this would overflow `i128` for any realistic token amount.
pub const MAX_SCALE: u32 = 36;

/// A fixed-point number scaled by 1e18 (18 decimal places).
///
/// This is the default working precision for token amounts, prices, and
/// spending-limit thresholds throughout the workspace.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Wad(pub i128);

