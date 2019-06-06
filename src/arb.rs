//! Aspect ratio-bounded tiling
use num::{PrimInt, Unsigned};
use std::borrow::BorrowMut;

use crate::core::{HilbertScanCore, LevelState};

/// An iterator wrapping [`HilbertScanCore`] that produces better results
/// for rectangles having extreme proportions.
///
/// See `HilbertScanCore`'s documentation for the usage.
///
/// The algorithm used by `HilbertScanCore` accepts any rectangle size, but
/// produces a worse result as the proportions of the rectangle gets distant
/// from square. `ArbHilbertScanCore` improves the output quality by dividing
/// the rectangle into multiple rectangles whose proportions are closer to
/// square than the original rectangle is (thus *aspect-ratio bounded*).
///
#[derive(Debug)]
pub struct ArbHilbertScanCore<T, LevelSt> {
    inner: Option<HilbertScanCore<T, LevelSt>>,
    major_axis: u8,
    divider: Divider<T>,
    /// The current part's position.
    pos: T,
    /// The current part's size.
    len: T,
}

impl<T, LevelSt> ArbHilbertScanCore<T, LevelSt>
where
    LevelSt: BorrowMut<[LevelState<T>]>,
    T: PrimInt + Unsigned + std::fmt::Debug,
{
    /// Construct a `ArbHilbertScanCore` with a default-constructed `LevelSt` .
    ///
    /// See also: [`HilbertScanCore::new`].
    pub fn new(size: [T; 2]) -> Self
    where
        LevelSt: Default,
    {
        Self::with_level_state_storage(LevelSt::default(), size)
    }

    /// Construct a `ArbHilbertScanCore` with an explicit `LevelSt`.
    ///
    /// The slice borrowed by `level_states` must have a specific minimum
    /// number of elements. The required number of elements varies in regard
    /// to `size` and it can be calculated using `num_levels_for_size`.
    /// The elements do not have to be initialized as they are overwritten
    /// by this function.
    pub fn with_level_state_storage(level_states: LevelSt, size: [T; 2]) -> Self {
        if size[0] == T::zero() || size[1] == T::zero() {
            return Self {
                inner: Some(HilbertScanCore::with_level_state_storage(
                    level_states,
                    size,
                )),
                major_axis: 0,
                divider: Divider {
                    remaining: T::zero(),
                    minor: T::zero(),
                },
                pos: T::zero(),
                len: T::zero(),
            };
        }

        let major_axis = (size[1] > size[0]) as usize;
        let mut divider = Divider {
            remaining: size[major_axis],
            minor: size[major_axis ^ 1],
        };

        // The first part
        let len = divider.next().unwrap_or_else(|| T::zero());

        Self {
            inner: Some(HilbertScanCore::with_level_state_storage(
                level_states,
                [len, divider.minor],
            )),
            major_axis: major_axis as u8,
            divider,
            pos: T::zero(),
            len,
        }
    }

    fn to_global(&self, mut p: [T; 2]) -> [T; 2] {
        p[0] = p[0] + self.pos;
        if self.major_axis != 0 {
            [p[1], p[0]]
        } else {
            [p[0], p[1]]
        }
    }
}

impl<T, LevelSt> std::iter::FusedIterator for ArbHilbertScanCore<T, LevelSt>
where
    LevelSt: BorrowMut<[LevelState<T>]>,
    T: PrimInt + Unsigned + std::fmt::Debug,
{
}

impl<T, LevelSt> Iterator for ArbHilbertScanCore<T, LevelSt>
where
    LevelSt: BorrowMut<[LevelState<T>]>,
    T: PrimInt + Unsigned + std::fmt::Debug,
{
    type Item = [T; 2];

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(p) = self.inner.as_mut().unwrap().next() {
            return Some(self.to_global(p));
        }

        let next_len = if let Some(x) = self.divider.next() {
            x
        } else {
            return None;
        };

        let level_states = self.inner.take().unwrap().into_level_states();
        let minor = self.divider.minor;
        self.inner = Some(HilbertScanCore::with_level_state_storage(
            level_states,
            [next_len, minor],
        ));
        self.pos = self.pos + self.len;
        self.len = next_len;

        let p = self.inner.as_mut().unwrap().next().unwrap();
        Some(self.to_global(p))
    }
}

#[derive(Debug)]
struct Divider<T> {
    remaining: T,
    minor: T,
}

impl<T> Divider<T>
where
    T: PrimInt + Unsigned,
{
    fn next(&mut self) -> Option<T> {
        if self.remaining == T::zero() {
            return None;
        }

        let count = division_count(self.remaining, self.minor);
        let remaining = self.remaining;

        let width = if count == T::one() {
            remaining
        } else {
            let mut w = remaining / count;
            if (w & T::one()) != T::zero() {
                // Make `w` even. We need the last point's Y coordinate to be `0`
                // so that the curve connects seamlessly to the next one.
                w = w + T::one();
            }
            w
        };

        self.remaining = self.remaining - width;

        Some(width)
    }
}

/// Estimate the optimal subdivision count.
fn division_count<T: PrimInt + Unsigned>(major: T, minor: T) -> T {
    if major <= minor {
        T::one()
    } else {
        // I can't believe how many integer divisions I wrote here...
        // (They are really slow and not fully pipelined on any known
        // processors)
        let k = major / minor;

        let w1 = major / k;
        let w2 = major / (k + T::one());

        let d1 = w1 - minor;
        let d2 = minor - w2;

        // Choose the one of `k` and `k + 1` that makes the proportion closer to
        // square
        if d1 < d2 {
            k
        } else {
            k + T::one()
        }
    }
}
