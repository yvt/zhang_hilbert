//! This crate provides a function that produces an arbitrary-sized
//! pseudo-Hilbert scan based on “A Pseudo-Hilbert Scan for Arbitrarily-Sized
//! Arrays” by Zhang, et al.
//!
//! # Differences from the original algorithm
//!
//! ## The `division` function
//!
//! TODO
//!
//! ## The last `E_B(E, O)` block
//!
//! This implementation uses a different curve-type selection rule for the
//! last `E_B(E, O)` block in a `E_R(E, O)` rectangle. This makes the leaving
//! point fixed at a known point in more cases, making the output suitable for
//! tiling.
//!
//! ```text
//! cargo run --example hilbertgen -- 6 7
//!   ,---, ,---,        ,---, ,---,
//!   '-, '-' ,-'        '-, '-' ,-'
//!   ,-' ,-, '-,        ,-' ,-, '-,
//!   '-, | '---'        '-, | '---'
//!   ,-' '-, ,--        ,-' '-----,
//!   '-, ,-' '-,        '-, ,-----'
//!   --' '-----'        --' '------
//!    Original      This implementation
//! ```
//!
//! ## Aspect-ratio bounded tiling
//!
//! TODO
//!
mod core;

pub use self::core::*;

pub type HilbertScan32 = HilbertScanCore<u32, [LevelState<u32>; 32]>;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
