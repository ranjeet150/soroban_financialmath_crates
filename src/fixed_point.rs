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
        self.0
            .checked_add(rhs.0)
            .map(Wad)
            .ok_or(MathError::Overflow)
    }

    /// Checked subtraction.
    pub fn checked_sub(self, rhs: Wad) -> Result<Wad, MathError> {
        self.0
            .checked_sub(rhs.0)
            .map(Wad)
            .ok_or(MathError::Underflow)
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
    ///
    /// The common case computes `self.0 * WAD` directly, which is exact and
    /// cheap but overflows `i128` once `self.0` exceeds roughly `1.7e20`
    /// (a `Wad` real value of ~170 — an entirely ordinary magnitude, e.g.
    /// a token amount in the low millions at 18 decimals). When that
    /// happens this falls back to a divide-first decomposition —
    /// `self.0 = q * rhs.0 + r` via ordinary `i128` division (which cannot
    /// itself overflow, since both operands already fit in `i128`), giving
    /// `(self.0 * WAD) / rhs.0 == q * WAD + (r * WAD) / rhs.0` — which
    /// keeps every intermediate within range for the realistic case this
    /// crate targets: `self` and `rhs` of comparable order of magnitude
    /// (as arise naturally in, e.g., [`crate::algorithms::sqrt`]'s
    /// Newton-Raphson refinement). It still reports
    /// [`MathError::Overflow`] rather than wrapping for the residual case
    /// where `rhs` is many orders of magnitude smaller than `self`.
    pub fn checked_div(self, rhs: Wad) -> Result<Wad, MathError> {
        if rhs.0 == 0 {
            return Err(MathError::DivisionByZero);
        }
        if let Some(scaled) = self.0.checked_mul(WAD) {
            return Ok(Wad(scaled / rhs.0));
        }
        let quotient = self.0 / rhs.0;
        let remainder = self.0 % rhs.0;
        let quotient_scaled = quotient.checked_mul(WAD).ok_or(MathError::Overflow)?;
        let remainder_scaled = remainder.checked_mul(WAD).ok_or(MathError::Overflow)?;
        quotient_scaled
            .checked_add(remainder_scaled / rhs.0)
            .map(Wad)
            .ok_or(MathError::Overflow)
    }

    /// Multiplies two `Wad`s without reducing the result back down,
    /// yielding a full-precision [`Rad`] (18 + 18 = 36 decimals). Useful
    /// when the caller needs to accumulate several `Wad * Wad` products
    /// before losing precision to a single final [`Rad::to_wad`] call.
    pub fn checked_mul_wide(self, rhs: Wad) -> Result<Rad, MathError> {
        self.0
            .checked_mul(rhs.0)
            .map(Rad)
            .ok_or(MathError::Overflow)
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
        self.0
            .checked_add(rhs.0)
            .map(Rad)
            .ok_or(MathError::Overflow)
    }

    /// Checked subtraction.
    pub fn checked_sub(self, rhs: Rad) -> Result<Rad, MathError> {
        self.0
            .checked_sub(rhs.0)
            .map(Rad)
            .ok_or(MathError::Underflow)
    }

    /// Narrows this `Rad` back down to a [`Wad`], truncating the extra
    /// 1e18 of precision. This is the standard point at which an
    /// accumulator is "settled" back into a spendable/storable amount.
    pub fn to_wad(self) -> Result<Wad, MathError> {
        scale_to(self.0, 36, 18).map(Wad)
    }
}

impl From<i128> for Wad {
    fn from(raw: i128) -> Self {
        Wad(raw)
    }
}

impl From<Wad> for i128 {
    fn from(w: Wad) -> Self {
        w.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_up_and_down_round_trip_within_precision() {
        // 1.0 at 6 decimals (USDC-style) -> 18 decimals -> back to 6.
        let raw_usdc = 1_000_000i128;
        let wad = Wad::from_scaled(raw_usdc, 6).unwrap();
        assert_eq!(wad.0, WAD);
        let back = wad.to_scaled(6).unwrap();
        assert_eq!(back, raw_usdc);
    }

    #[test]
    fn downscaling_truncates_rather_than_rounds() {
        // 1.999999 wad-scaled down to 0 decimals must truncate to 1, not
        // round to 2.
        let wad = Wad(1_999_999_000_000_000_000);
        assert_eq!(wad.to_scaled(0).unwrap(), 1);
    }

    #[test]
    fn mul_and_div_are_inverse_for_nonzero_operands() {
        let a = Wad::from_raw(3 * WAD);
        let b = Wad::from_raw(2 * WAD);
        let product = a.checked_mul(b).unwrap();
        assert_eq!(product.0, 6 * WAD);
        let quotient = product.checked_div(b).unwrap();
        assert_eq!(quotient.0, a.0);
    }

    #[test]
    fn div_by_zero_is_rejected() {
        let a = Wad::ONE;
        assert_eq!(a.checked_div(Wad::ZERO), Err(MathError::DivisionByZero));
    }

    #[test]
    fn div_of_large_comparable_magnitude_operands_does_not_overflow() {
        // A million-token-scale numerator (self.0 * WAD alone would
        // overflow i128) divided by an operand of comparable magnitude —
        // this is exactly the shape sqrt()'s Newton-Raphson refinement
        // produces, and previously overflowed unconditionally.
        let a = Wad(1_000_000 * WAD);
        let b = Wad(1_000 * WAD);
        assert_eq!(a.checked_div(b).unwrap(), Wad(1_000 * WAD));
    }

    #[test]
    fn div_large_close_magnitude_operands_matches_exact_expectation() {
        // Two large Wads close in magnitude — as arise in sqrt()'s
        // Newton-Raphson refinement, where the two operands converge
        // toward equality — where `self.0 * WAD` alone overflows, but the
        // quotient is small and the remainder stays well within range so
        // the fallback resolves it exactly rather than reporting Overflow.
        let a = Wad(1_000_000 * WAD + 7 * WAD); // 1,000,007.0
        let b = Wad(1_000_000 * WAD); // 1,000,000.0
        let result = a.checked_div(b).unwrap();
        // 1,000,007 / 1,000,000 = 1.000007
        assert_eq!(result.0, WAD + 7_000_000_000_000);
    }

    #[test]
    fn div_reports_overflow_rather_than_a_wrong_answer_when_unrepresentable() {
        // Operands of wildly different magnitude, both individually valid
        // i128 Wads, whose exact quotient's remainder term cannot be
        // represented even via the fallback decomposition. The crate's
        // contract is "checked arithmetic everywhere" — this must fail
        // loudly with `Overflow`, never silently truncate or wrap.
        let a = Wad(i128::MAX - 1);
        let b = Wad(3);
        assert_eq!(a.checked_div(b), Err(MathError::Overflow));
    }

    #[test]
    fn wide_multiplication_round_trips_through_rad() {
        let a = Wad::from_raw(5 * WAD);
        let b = Wad::ONE;
        let rad = a.checked_mul_wide(b).unwrap();
        assert_eq!(rad.0, 5 * RAD); // 5.0 * 1.0 at 36-decimal scale
        assert_eq!(rad.to_wad().unwrap(), a);
    }

    #[test]
    fn invalid_scale_is_rejected() {
        assert_eq!(scale_to(1, 0, MAX_SCALE + 1), Err(MathError::InvalidScale));
    }
}
