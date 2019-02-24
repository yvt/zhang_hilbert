//! This crate provides a function that produces an arbitrary-sized
//! pseudo-Hilbert scan based on “A Pseudo-Hilbert Scan for Arbitrarily-Sized
//! Arrays” by Zhang, et al.
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
