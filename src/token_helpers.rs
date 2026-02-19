//! Helper methods for [`NearToken`] and [`NearGas`] types.
//!
//! Provides convenient constructors and conversions for working with
//! yoctoNEAR amounts (as decimal strings) and gas units.

use crate::types::{NearGas, NearToken};

/// 1 NEAR = 10^24 yoctoNEAR.
const YOCTO_PER_NEAR: u128 = 1_000_000_000_000_000_000_000_000;

/// 1 TGas = 10^12 gas units.
const GAS_PER_TGAS: u64 = 1_000_000_000_000;

// ---------------------------------------------------------------------------
// NearToken helpers
// ---------------------------------------------------------------------------

impl NearToken {
    /// Parse the inner yoctoNEAR decimal string to `u128`.
    ///
    /// # Panics
    ///
    /// Panics if the inner string is not a valid `u128`.
    pub fn as_yoctonear(&self) -> u128 {
        self.0
            .parse::<u128>()
            .expect("NearToken inner string is not a valid u128")
    }

    /// Create a [`NearToken`] from a yoctoNEAR amount.
    pub fn from_yoctonear(amount: u128) -> Self {
        Self(amount.to_string())
    }

    /// Create a [`NearToken`] from whole NEAR (multiplied by 10^24).
    pub fn from_near(amount: u64) -> Self {
        Self::from_yoctonear(u128::from(amount) * YOCTO_PER_NEAR)
    }

    /// Approximate value in NEAR as `f64` (useful for display).
    pub fn as_near_f64(&self) -> f64 {
        self.as_yoctonear() as f64 / YOCTO_PER_NEAR as f64
    }
}

// ---------------------------------------------------------------------------
// NearGas helpers
// ---------------------------------------------------------------------------

impl NearGas {
    /// Returns the inner gas value.
    pub fn as_gas(&self) -> u64 {
        self.0
    }

    /// Create a [`NearGas`] from raw gas units.
    pub fn from_gas(gas: u64) -> Self {
        Self(gas)
    }

    /// Approximate value in TGas as `f64` (useful for display).
    pub fn as_tgas(&self) -> f64 {
        self.0 as f64 / GAS_PER_TGAS as f64
    }

    /// Create a [`NearGas`] from TGas (multiplied by 10^12).
    pub fn from_tgas(tgas: u64) -> Self {
        Self(tgas * GAS_PER_TGAS)
    }
}

// ---------------------------------------------------------------------------
// Optional interop with near-token / near-gas crates
// ---------------------------------------------------------------------------

#[cfg(feature = "near-primitives")]
mod interop {
    use super::*;

    impl From<NearToken> for near_token::NearToken {
        fn from(t: NearToken) -> Self {
            near_token::NearToken::from_yoctonear(t.as_yoctonear())
        }
    }

    impl From<near_token::NearToken> for NearToken {
        fn from(t: near_token::NearToken) -> Self {
            NearToken::from_yoctonear(t.as_yoctonear())
        }
    }

    impl From<NearGas> for near_gas::NearGas {
        fn from(g: NearGas) -> Self {
            near_gas::NearGas::from_gas(g.as_gas())
        }
    }

    impl From<near_gas::NearGas> for NearGas {
        fn from(g: near_gas::NearGas) -> Self {
            NearGas::from_gas(g.as_gas())
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn near_token_yoctonear_round_trip() {
        let amount: u128 = 999_999_999_999_999_999_999_999;
        let token = NearToken::from_yoctonear(amount);
        assert_eq!(token.as_yoctonear(), amount);
    }

    #[test]
    fn near_token_from_near() {
        let token = NearToken::from_near(1);
        assert_eq!(
            token.0, "1000000000000000000000000",
            "1 NEAR should be 10^24 yoctoNEAR"
        );
        assert_eq!(token.as_yoctonear(), YOCTO_PER_NEAR);
    }

    #[test]
    fn near_token_from_near_multiple() {
        let token = NearToken::from_near(5);
        assert_eq!(token.as_yoctonear(), 5 * YOCTO_PER_NEAR);
    }

    #[test]
    fn near_token_as_near_f64() {
        let token = NearToken::from_near(3);
        let approx = token.as_near_f64();
        assert!((approx - 3.0).abs() < 1e-10, "Expected ~3.0, got {approx}");
    }

    #[test]
    fn near_token_as_near_f64_fractional() {
        // 0.5 NEAR
        let half = YOCTO_PER_NEAR / 2;
        let token = NearToken::from_yoctonear(half);
        let approx = token.as_near_f64();
        assert!((approx - 0.5).abs() < 1e-10, "Expected ~0.5, got {approx}");
    }

    #[test]
    fn near_gas_round_trip() {
        let gas = 300_000_000_000_000_u64;
        let g = NearGas::from_gas(gas);
        assert_eq!(g.as_gas(), gas);
    }

    #[test]
    fn near_gas_tgas_conversion() {
        let g = NearGas::from_tgas(300);
        assert_eq!(g.as_gas(), 300 * GAS_PER_TGAS);
        assert!((g.as_tgas() - 300.0).abs() < 1e-10);
    }

    #[test]
    fn near_gas_from_tgas_round_trip() {
        let g = NearGas::from_tgas(1);
        assert_eq!(g.as_gas(), GAS_PER_TGAS);
        assert!((g.as_tgas() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn near_token_serde_round_trip() {
        let token = NearToken::from_near(2);
        let json = serde_json::to_string(&token).expect("serialize");
        let deserialized: NearToken = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(token.as_yoctonear(), deserialized.as_yoctonear());
    }

    #[test]
    fn near_gas_serde_round_trip() {
        let gas = NearGas::from_tgas(100);
        let json = serde_json::to_string(&gas).expect("serialize");
        let deserialized: NearGas = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(gas.as_gas(), deserialized.as_gas());
    }

    #[cfg(feature = "near-primitives")]
    mod interop_tests {
        use super::*;

        #[test]
        fn near_token_to_near_token_crate() {
            let ours = NearToken::from_near(10);
            let theirs: near_token::NearToken = ours.into();
            assert_eq!(theirs.as_yoctonear(), 10 * YOCTO_PER_NEAR);
        }

        #[test]
        fn near_token_from_near_token_crate() {
            let theirs = near_token::NearToken::from_yoctonear(YOCTO_PER_NEAR);
            let ours: NearToken = theirs.into();
            assert_eq!(ours.as_yoctonear(), YOCTO_PER_NEAR);
        }

        #[test]
        fn near_gas_to_near_gas_crate() {
            let ours = NearGas::from_tgas(5);
            let theirs: near_gas::NearGas = ours.into();
            assert_eq!(theirs.as_gas(), 5 * GAS_PER_TGAS);
        }

        #[test]
        fn near_gas_from_near_gas_crate() {
            let theirs = near_gas::NearGas::from_gas(GAS_PER_TGAS);
            let ours: NearGas = theirs.into();
            assert_eq!(ours.as_gas(), GAS_PER_TGAS);
        }
    }
}
