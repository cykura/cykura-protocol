///! Math library for liquidity

use anchor_lang::require;
use crate::error::ErrorCode;

/// Add a signed liquidity delta to liquidity and revert if it overflows or underflows
///
/// # Arguments
///
/// * `x` - The liquidity (L) before change
/// * `y` - The delta (ΔL) by which liquidity should be changed
///
pub fn add_delta(x: u64, y: i64) -> Result<u64, ErrorCode> {
    let z: u64;
    if y < 0 {
        z = x - (y.abs() as u64);
        require!(z < x, ErrorCode::LS);
    } else {
        z = x + (y as u64);
        require!(z >= x, ErrorCode::LA);
    }

    Ok(z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_liquidity_delta() {
        let x: u64 = 1;
        let y: i64 = 2;
        assert_eq!(add_delta(x, y).unwrap(), 3);
    }

    #[test]
    fn negative_liquidity_delta() {
        let x: u64 = 2;
        let y: i64 = -1;
        assert_eq!(add_delta(x, y).unwrap(), 1);
    }

    #[test]
    #[should_panic]
    fn positive_liquidity_delta_overflow() {
        add_delta(u64::MAX, 1).unwrap();
    }

    #[test]
    #[should_panic]
    fn negative_liquidity_delta_underflow() {
        add_delta(u64::MIN, -1).unwrap();
    }
}
