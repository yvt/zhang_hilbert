//! This crate provides iterator types that produce an arbitrary-sized
//! pseudo-Hilbert scan based on “A Pseudo-Hilbert Scan for Arbitrarily-Sized
//! Arrays” by Zhang, et al.
//!
//! ![](https://ipfs.io/ipfs/QmUbNnFkcyHQrg3CpNf3ykVq6dm7vG7CGzU8tryzWvXrEf/thecurve.svg)
//!
//! ```
//! use zhang_hilbert::ArbHilbertScan32;
//! for [x, y] in ArbHilbertScan32::new([11, 42]) {
//!     assert!(x >= 0 && y >= 0 && x < 11 && y < 42);
//!     println!("{:?}", [x, y]);
//! }
//! ```
//!
//! # Differences from the original algorithm
//!
//! ## The last `E_B(E, O)` block
//!
//! This implementation uses a different curve-type selection rule for the
//! last `E_B(E, O)` block in a `E_R(E, O)` rectangle. This makes the leaving
//! point fixed at a known point in more cases, making the output suitable for
//! tiling.
//!
//! ```text
//! cargo run --example hilbertgen -- -a zhang 6 7
//!   ,---, ,---,        ,---, ,---,
//!   '-, '-' ,-'        '-, '-' ,-'
//!   ,-' ,-, '-,        ,-' ,-, '-,
//!   '-, | '---'        '-, | '---'
//!   ,-' '-, ,--        ,-' '-----,
//!   '-, ,-' '-,        '-, ,-----'
//!   --' '-----'        --' '------
//!    Original      This implementation
//!
//! cargo run --example hilbertgen -- -a zhang 4 3
//!     ,------           ,-----,
//!     '-----,           '-, ,-'
//!     ------'           --' '--
//!    Original      This implementation
//! ```
//!
//! ## Aspect-ratio bounded tiling
//!
//! The algorithm accepts any rectangle size, but the output quality
//! deteriorates as the proportions of the rectangle gets distant from square.
//! `ArbHilbertScanCore` improves it by dividing the rectangle into multiple
//! rectangles whose proportions are closer to square than the original
//! rectangle is (thus their aspect ratios are bounded).
//!
//! ```text
//! $ cargo run --example hilbertgen -- 40 7
//! ,---, ,---, ,---, ,---, ,---, ,-, ,---, ,---, ,---, ,---, ,-, ,---, ,---, ,---,
//! '-, '-' ,-' '-, '-' ,-' '-, '-' '-' ,-' '-, '-' ,-' '-, '-' '-' ,-' '-, '-' ,-'
//! ,-' ,-, '-, ,-' ,-, '-, ,-' ,-, ,-, '-, ,-' ,-, '-, ,-' ,-, ,-, '-, ,-' ,-, '-,
//! '-, | '---' '-, | '---' '---' | | '---' '-, | '---' '---' | | '---' '-, | '---'
//! ,-' '-----, ,-' '-----, ,-----' '-----, ,-' '-----, ,-----' '-----, ,-' '-----,
//! '-, ,-----' '-, ,-----' '-----, ,-----' '-, ,-----' '-----, ,-----' '-, ,-----'
//! --' '---------' '-------------' '---------' '-------------' '---------' '------
//!
//! $ cargo run --example hilbertgen -- -a zhang 40 7
//! ,-----------------------, ,-, ,-, ,-, ,-, ,-, ,-, ,-, ,-, ,-, ,---------------,
//! '---------------------, '-' '-' '-' '-' '-' '-' '-' '-' '-' '-' ,-------------'
//! ,---------------------' ,-, ,-, ,-, ,-, ,-, ,-, ,-, ,-, ,-, ,-, '-------------,
//! '-----------------------' '-' '-' '-' '-' '-' | | '-' '-' '-' '---------------'
//! ,---------------------------------------------' '-----------------------------,
//! '---------------------------------------------, ,-----------------------------'
//! ----------------------------------------------' '------------------------------
//! ```
//!
//! ## The `division` function
//!
//! The `division` internal function was modified for efficient implementation.
//! As a result, the function produces an different output for the input `3⋅2ⁿ`.
//!
mod arb;
mod core;

pub use self::{arb::*, core::*};

/// `HilbertScanCore` with an array-based working area.
pub type HilbertScan32 = HilbertScanCore<u32, [LevelState<u32>; 32]>;

/// `ArbHilbertScan32` with an array-based working area.
pub type ArbHilbertScan32 = ArbHilbertScanCore<u32, [LevelState<u32>; 32]>;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
