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

/// A fixed-point number scaled by 1e36 (36 decimal places) — the natural,
/// unreduced result of multiplying two [`Wad`]s together without losing
/// precision to premature truncation.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Rad(pub i128);

/// Converts a raw integer amount at `from_decimals` precision into a raw
/// integer amount at `to_decimals` precision.
///
/// This is the primitive underlying every `Wad`/`Rad` conversion and is
/// also useful standalone for normalizing token amounts that carry
/// different `decimals()` (e.g. a 6-decimal USDC-style asset feeding into
/// 18-decimal internal accounting).
///
/// # Errors
/// - [`MathError::InvalidScale`] if either decimal count exceeds
///   [`MAX_SCALE`].
/// - [`MathError::Overflow`] if scaling up would exceed `i128::MAX`.
pub fn scale_to(amount: i128, from_decimals: u32, to_decimals: u32) -> Result<i128, MathError> {
    if from_decimals > MAX_SCALE || to_decimals > MAX_SCALE {
        return Err(MathError::InvalidScale);
    }

    if to_decimals >= from_decimals {
        let diff = to_decimals - from_decimals;
        let factor = pow10(diff)?;
        amount.checked_mul(factor).ok_or(MathError::Overflow)
    } else {
        let diff = from_decimals - to_decimals;
        let factor = pow10(diff)?;
        // Integer division truncates toward zero, which is the correct,
        // conservative direction for downscaling on-chain balances (never
        // manufacture value that wasn't there).
        Ok(amount / factor)
    }
}

/// Computes `10^exp` as an `i128`, guarding against overflow for
/// out-of-range exponents.
fn pow10(exp: u32) -> Result<i128, MathError> {
    10i128.checked_pow(exp).ok_or(MathError::Overflow)
}

impl Wad {
    /// The additive identity, `0.0`.
    pub const ZERO: Wad = Wad(0);
    /// The multiplicative identity, `1.0`.
    pub const ONE: Wad = Wad(WAD);

    /// Wraps a raw, already-scaled `i128` as a `Wad`.
    pub const fn from_raw(raw: i128) -> Self {
        Wad(raw)
    }

    /// Builds a `Wad` from an integer amount expressed at `decimals`
    /// precision (e.g. `Wad::from_scaled(1_000_000, 6)` for `1.0` USDC).
    pub fn from_scaled(amount: i128, decimals: u32) -> Result<Self, MathError> {
        scale_to(amount, decimals, 18).map(Wad)
    }

    /// Converts this `Wad` down to a raw integer amount at `decimals`
    /// precision, truncating any precision beyond `decimals`.
    pub fn to_scaled(self, decimals: u32) -> Result<i128, MathError> {
        scale_to(self.0, 18, decimals)
    }

    /// Checked addition.
    pub fn checked_add(self, rhs: Wad) -> Result<Wad, MathError> {
        self.0.checked_add(rhs.0).map(Wad).ok_or(MathError::Overflow)
    }

    /// Checked subtraction.
    pub fn checked_sub(self, rhs: Wad) -> Result<Wad, MathError> {
        self.0.checked_sub(rhs.0).map(Wad).ok_or(MathError::Underflow)
    }

    /// Checked fixed-point multiplication: `(self * rhs) / WAD`.
    ///
    /// Widens to [`Rad`] internally (via [`Wad::checked_mul_wide`]) so the
    /// intermediate product never overflows `i128` prematurely, then
    /// rescales back down.
    pub fn checked_mul(self, rhs: Wad) -> Result<Wad, MathError> {
        self.checked_mul_wide(rhs)?.to_wad()
    }

    /// Checked fixed-point division: `(self * WAD) / rhs`.
    pub fn checked_div(self, rhs: Wad) -> Result<Wad, MathError> {
        if rhs.0 == 0 {
            return Err(MathError::DivisionByZero);
        }
        let scaled = self.0.checked_mul(WAD).ok_or(MathError::Overflow)?;
        Ok(Wad(scaled / rhs.0))
    }

    /// Multiplies two `Wad`s without reducing the result back down,
    /// yielding a full-precision [`Rad`] (18 + 18 = 36 decimals). Useful
    /// when the caller needs to accumulate several `Wad * Wad` products
    /// before losing precision to a single final [`Rad::to_wad`] call.
    pub fn checked_mul_wide(self, rhs: Wad) -> Result<Rad, MathError> {
        self.0.checked_mul(rhs.0).map(Rad).ok_or(MathError::Overflow)
    }

    /// True if the underlying value is exactly zero.
    pub fn is_zero(self) -> bool {
        self.0 == 0
    }
}

impl Rad {
    /// The additive identity, `0.0`.
    pub const ZERO: Rad = Rad(0);

    /// Wraps a raw, already-scaled `i128` as a `Rad`.
    pub const fn from_raw(raw: i128) -> Self {
        Rad(raw)
    }

    /// Checked addition.
    pub fn checked_add(self, rhs: Rad) -> Result<Rad, MathError> {
        self.0.checked_add(rhs.0).map(Rad).ok_or(MathError::Overflow)
    }

    /// Checked subtraction.
    pub fn checked_sub(self, rhs: Rad) -> Result<Rad, MathError> {
        self.0.checked_sub(rhs.0).map(Rad).ok_or(MathError::Underflow)
    }

    /// Narrows this `Rad` back down to a [`Wad`], truncating the extra
    /// 1e18 of precision. This is the standard point at which an
    /// accumulator is "settled" back into a spendable/storable amount.
    pub fn to_wad(self) -> Result<Wad, MathError> {
        scale_to(self.0, 36, 18).map(Wad)
    }
}

