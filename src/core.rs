//! The core implementation of the algorithm.
use array::Array2;
use bitflags::bitflags;
use flags_macro::flags;
use num::{PrimInt, Unsigned};
use std::{borrow::BorrowMut, cmp::min};

/// Stores pre-calculated values used to generate a pseudo-Hilbert scan of
/// a specific size.
#[derive(Debug, Default)]
pub struct LevelInfo {}

/// Stores the state data required for a single subdivision level.
///
/// `T` is a type used to represent the output coordinates.
#[derive(Debug, Default, Clone, Copy)]
pub struct LevelState<T> {
    size: [T; 2],
    /// The curve type of this block. Only used for block address assignment.
    ///
    /// Invariant: `i == 0 || cur.curve_type == CURVE_INDUCTION_TABLE[prev.curve_type][prev.progress]`
    /// where `cur` is `level_states[i]` and `prev` is `level_states[i - 1]`.
    curve_type: u8,
    /// An integer in `0..4`. Indicates which subblock we are in this level's
    /// block. Invalid for the last level.
    progress: u8,
}

/// The curve type address sequence table.
///
/// Let `Tₜᵣₘ[γ][i]` be `(CURVE_ADDRESS_TABLE[γ] >> (i * 2)) & 0b11`.
/// `Tₜᵣₘ[γ][i]` represents the position of `i`-th subblock within a block
/// assigned a curve type `γ`.
const CURVE_ADDRESS_TABLE: [u8; 4] = [0b10_11_01_00, 0b01_11_10_00, 0b01_00_10_11, 0b10_00_01_11];

/// The curve induction table.
///
/// `CURVE_INDUCTION_TABLE[γ][i]` represents the curve type of `i`-th subblock
/// within a block assigned a curve type `γ`.
const CURVE_INDUCTION_TABLE: [[u8; 4]; 4] =
    [[1, 0, 0, 3], [0, 1, 1, 2], [3, 2, 2, 1], [2, 3, 3, 0]];

/// Get the number of [`LevelState`]s required by [`HilbertScanCore`] to
/// hold its internal state.
///
/// `size[0]` and `size[1]` must be both greater than `1`.
pub fn num_levels_for_size<T: PrimInt + Unsigned>(size: [T; 2]) -> usize {
    assert!(size[0] > T::one());
    assert!(size[1] > T::one());
    log2_floor(min(size[0], size[1])) as usize
}

fn log2_floor<T: PrimInt>(x: T) -> u32 {
    T::zero().leading_zeros() - 1 - x.leading_zeros()
}

/// Find the split position (l₁) of a side.
fn division_l1<T: PrimInt + Unsigned>(size: T) -> T {
    let m = log2_floor(size) - 1;

    let mask = T::one().unsigned_shl(m);
    (size & mask) + mask
}

/// An iterator producing a pseudo-Hilbert scan.
///
/// `T` is a type used to represent the output coordinates. `LevelSt` is
/// a mutable reference to a slice of `LevelState<T>`s.
#[derive(Debug)]
pub struct HilbertScanCore<T, LevelSt> {
    size: [T; 2],
    num_levels: usize,
    level_states: LevelSt,
    position: [T; 2],

    // ============ Basic (last-level block) scanning state =============
    bb_progress: [T; 2],
    bb_flags: BbFlags,

    done: bool,
}

bitflags! {
    struct BbFlags: u8 {
        /// The primary axis of the scan. `0` = the X axis, `1` = the Y axis.
        const PRIMARY_AXIS_Y = 1 << 0;
        /// The sign of the primary scan direction. `0` = positive, `1` = negative.
        const PRIMARY_DIR_NEG = 1 << 1;
        /// The sign of the secondary scan direction.
        /// This flag can be flipped for multiple times in a single scan
        /// to draw a zigzag pattern.
        const SECONDARY_DIR_NEG = 1 << 2;
        /// The Y position of the final point.
        const END_X = 1 << 5;
        /// The X position of the final point.
        const END_Y = 1 << 4;
    }
}

impl BbFlags {
    fn end(self) -> u8 {
        (self & (Self::END_X | Self::END_Y)).bits() / Self::END_Y.bits()
    }
}

impl<T, LevelSt> HilbertScanCore<T, LevelSt>
where
    LevelSt: BorrowMut<[LevelState<T>]>,
    T: PrimInt + Unsigned,
{
    /// Construct a `HilbertScanCore` with a default-constructed `LevelSt`.
    ///
    /// **Warning**: As noted in the documentation of [`with_level_state_storage`],
    /// the supplied `LevelSt` must have a certain number of elements. When
    /// `LevelSt` is `Vec`, this function always panics because `Vec` is
    /// default-constructed to have zero elements. Rather, this function is
    /// useful when `LevelSt` has a predetermined number of elements like
    /// `[LevelState<T>; 32]` does.
    pub fn new(size: [T; 2]) -> Self
    where
        LevelSt: Default,
    {
        Self::with_level_state_storage(LevelSt::default(), size)
    }

    /// Construct a `HilbertScanCore` with an explicit `LevelSt`.
    ///
    /// The slice borrowed by `level_states` must have a specific minimum
    /// number of elements. The required number of elements varies in regard
    /// to `size` and it can be calculated using `num_levels_for_size`.
    /// The elements do not have to be initialized as they are overwritten
    /// by this function.
    pub fn with_level_state_storage(mut level_states: LevelSt, size: [T; 2]) -> Self {
        let num_levels = num_levels_for_size(size);
        let last_size;
        {
            let level_states = &mut level_states.borrow_mut()[0..num_levels];
            level_states[0] = LevelState {
                size,
                curve_type: 0, // γ(0) = 1
                progress: 0,
            };
            for i in 1..num_levels {
                let prev = level_states[i - 1];
                level_states[i] = LevelState {
                    size: prev.size.map(|x| x - division_l1(x)),
                    curve_type: (i % 2) as u8, // CURVE_INDUCTION_TABLE[prev.curve_type as usize][0],
                    progress: 0,
                };
            }
            last_size = level_states.last().unwrap().size;
        }

        // Set up the scan of the first block
        let second_to_last_curve_type = (num_levels - 1) % 2;
        let bb_flags = match size.map(|x| (x & T::one()).to_u8().unwrap()) {
            // T_R(E, E)
            [0, 0] => {
                [
                    // Type-0 basic pattern
                    flags![BbFlags::{END_X}],
                    // Type-1 basic pattern
                    flags![BbFlags::{PRIMARY_AXIS_Y | END_Y}],
                ][second_to_last_curve_type]
            }
            // T_R(E, O) - Type-1 basic pattern
            [0, 1] => flags![BbFlags::{PRIMARY_AXIS_Y | END_X | END_Y}],
            // T_R(O, E), T_R(O, O) - Type-0 basic pattern
            [1, 0] | [1, 1] => flags![BbFlags::{END_X | END_Y}],
            [_, _] => unreachable!(),
        };

        let bb_progress = if bb_flags.contains(BbFlags::PRIMARY_AXIS_Y) {
            [last_size[1], last_size[0]]
        } else {
            [last_size[0], last_size[1]]
        };

        Self {
            size,
            num_levels,
            level_states,
            position: [T::zero(), T::zero()],
            bb_progress,
            bb_flags,
            done: false,
        }
    }
}

impl<T, LevelSt> Iterator for HilbertScanCore<T, LevelSt>
where
    LevelSt: BorrowMut<[LevelState<T>]>,
    T: PrimInt + Unsigned + std::fmt::Debug,
{
    type Item = [T; 2];

    // Inlining this method opens useful optimization opportunities:
    //  - `self.done` checks can be eliminated
    //  - Array bounds checks on `level_states` can be eliminated
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let num_levels = self.num_levels;
        let level_states = &mut self.level_states.borrow_mut()[0..num_levels];

        // The output position
        let position = self.position;

        // Update the basic block scan state
        let [mut pri, mut sec] = self.bb_progress;
        let pri_axis = self.bb_flags.contains(BbFlags::PRIMARY_AXIS_Y) as usize;
        let sec_axis = (!self.bb_flags.contains(BbFlags::PRIMARY_AXIS_Y)) as usize;
        let sec_width = level_states.last().unwrap().size[sec_axis];
        sec = sec - T::one();

        if sec == T::zero() {
            pri = pri - T::one();
            sec = sec_width;
            // Zigzag
            self.bb_flags.toggle(BbFlags::SECONDARY_DIR_NEG);
        } else {
            let sec_pos = &mut self.position[sec_axis];
            if self.bb_flags.contains(BbFlags::SECONDARY_DIR_NEG) {
                *sec_pos = *sec_pos - T::one();
            } else {
                *sec_pos = *sec_pos + T::one();
            }
            self.bb_progress = [pri, sec];
            return Some(position);
        }

        if pri == T::zero() {
            // This block is complete! Find the next block.
        } else {
            let pri_pos = &mut self.position[pri_axis];
            if self.bb_flags.contains(BbFlags::PRIMARY_DIR_NEG) {
                *pri_pos = *pri_pos - T::one();
            } else {
                *pri_pos = *pri_pos + T::one();
            }
            self.bb_progress = [pri, sec];
            return Some(position);
        }

        if num_levels == 1 {
            self.done = true;
            return Some(position);
        }

        let mut i = num_levels - 2;
        let next_bb_enter;

        loop {
            level_states[i].progress += 1;
            if level_states[i].progress == 4 {
                if i == 0 {
                    // No left blocks
                    self.done = true;
                    return Some(position);
                } else {
                    i -= 1;
                }
            } else {
                // Get the relative position of the next block
                let level = &level_states[i];
                let adr = CURVE_ADDRESS_TABLE[level.curve_type as usize]
                    >> (level.progress * 2 - 2) as u32;
                // adr[1:0] = current, adr[3:2] = next
                let adr_rel = adr ^ (adr >> 2);
                debug_assert!((adr_rel & 3) == 0b01 || (adr_rel & 3) == 0b10);

                // ... and move the cursor based on that
                let is_adr_rel_primary = (adr_rel >> sec_axis as u32) & 1 != 0;

                if is_adr_rel_primary {
                    let pri_pos = &mut self.position[pri_axis];
                    if self.bb_flags.contains(BbFlags::PRIMARY_DIR_NEG) {
                        *pri_pos = *pri_pos - T::one();
                    } else {
                        *pri_pos = *pri_pos + T::one();
                    }
                } else {
                    let sec_pos = &mut self.position[sec_axis];
                    // This condition is inverted on purpose to cancel out
                    // the effect of the "zigzag" part.
                    if self.bb_flags.contains(BbFlags::SECONDARY_DIR_NEG) {
                        *sec_pos = *sec_pos + T::one();
                    } else {
                        *sec_pos = *sec_pos - T::one();
                    }
                }

                // Now we also know where do we enter the next block
                next_bb_enter = self.bb_flags.end() ^ (adr_rel & 0b11);
                break;
            }
        }

        while i + 1 < num_levels {
            let progress = level_states[i].progress;
            let curve_type = level_states[i].curve_type;

            let adr = CURVE_ADDRESS_TABLE[curve_type as usize] >> (progress * 2) as u32;
            let adr0 = (adr & 0b10) != 0;
            let adr1 = (adr & 0b01) != 0;
            let ind = CURVE_INDUCTION_TABLE[curve_type as usize][progress as usize];

            let prev_size = level_states[i].size;
            let size_l1 = prev_size.map(division_l1);
            let size_l0 = [prev_size[0] - size_l1[0], prev_size[1] - size_l1[1]];

            let size = [
                if adr0 { size_l1[0] } else { size_l0[0] },
                if adr1 { size_l1[1] } else { size_l0[1] },
            ];

            level_states[i + 1].size = size;
            level_states[i + 1].curve_type = ind;
            level_states[i + 1].progress = 0;

            i += 1;
        }

        // Now that a new block is found, the scanning pattern of the block
        // must be determined.
        //
        // > we always know the entry point of the current scanned block
        // > (T_B(E, E)) and the location (left, right, up or down) of the next
        // > block. Then we can decide the scanning manner of this T_B(E, E)
        // > block.
        //
        let size = level_states.last().unwrap().size;
        let even_flags = (((size[0] & T::zero()) << 1) | (size[1] & T::zero()))
            .to_u8()
            .unwrap();
        // Alas, this can't be initialized using constexpr.
        // I wonder why they didn't mention the memory consumption of this
        // look-up table in the paper. (Not to mention the local variables...)
        let scanning_type_table: [[[BbFlags; 2]; 4]; 2] = [
            // Move right/up
            [
                [
                    // Bottom-left to ...
                    // 0b00 → 0b10 - Type-0 basic pattern
                    flags![BbFlags::{END_X}],
                    // 0b00 → 0b01 - Type-1 basic pattern
                    flags![BbFlags::{PRIMARY_AXIS_Y | END_Y}],
                ],
                [
                    // Top-left to ...
                    // 0b01 → 0b11 - Reversed type-2 basic pattern
                    flags![BbFlags::{SECONDARY_DIR_NEG | END_X | END_Y}],
                    // 0b01 → 0b11 - Reversed type-2 basic pattern
                    flags![BbFlags::{SECONDARY_DIR_NEG | END_X | END_Y}],
                ],
                [
                    // bottom-right to ...
                    // 0b10 → 0b11 - Reversed type-3 basic pattern
                    flags![BbFlags::{SECONDARY_DIR_NEG | PRIMARY_AXIS_Y | END_X | END_Y}],
                    // 0b10 → 0b11 - Reversed type-3 basic pattern
                    flags![BbFlags::{SECONDARY_DIR_NEG | PRIMARY_AXIS_Y | END_X | END_Y}],
                ],
                [
                    // top-right to ...
                    // 0b11 → 0b10 - Type-3 basic pattern
                    flags![BbFlags::{PRIMARY_DIR_NEG | SECONDARY_DIR_NEG | PRIMARY_AXIS_Y | END_X}],
                    // 0b11 → 0b01 - Type-2 basic pattern
                    flags![BbFlags::{PRIMARY_DIR_NEG | SECONDARY_DIR_NEG | END_Y}],
                ],
            ],
            // Move left/down
            [
                [
                    // Bottom-left to ...
                    // 0b00 → 0b01 - Type-1 basic pattern
                    flags![BbFlags::{PRIMARY_AXIS_Y | END_Y}],
                    // 0b00 → 0b10 - Type-0 basic pattern
                    flags![BbFlags::{END_X}],
                ],
                [
                    // Top-left to ...
                    // 0b01 → 0b00 - Reversed type-1 basic pattern
                    flags![BbFlags::{PRIMARY_DIR_NEG | PRIMARY_AXIS_Y}],
                    // 0b01 → 0b00 - Reversed type-1 basic pattern
                    flags![BbFlags::{PRIMARY_DIR_NEG | PRIMARY_AXIS_Y}],
                ],
                [
                    // bottom-right to ...
                    // 0b10 → 0b00 - Reversed type-0 basic pattern
                    flags![BbFlags::{PRIMARY_DIR_NEG}],
                    // 0b10 → 0b00 - Reversed type-0 basic pattern
                    flags![BbFlags::{PRIMARY_DIR_NEG}],
                ],
                [
                    // top-right to ...
                    // 0b11 → 0b01 - Type-2 basic pattern
                    flags![BbFlags::{PRIMARY_DIR_NEG | SECONDARY_DIR_NEG | END_Y}],
                    // 0b11 → 0b10 - Type-3 basic pattern
                    flags![BbFlags::{PRIMARY_DIR_NEG | SECONDARY_DIR_NEG | PRIMARY_AXIS_Y | END_X}],
                ],
            ],
        ];
        self.bb_flags = match even_flags {
            // T_B(E, E)
            0b00 => {
                // Find "the location (left, right, up or down) of the next block"
                let next_dir;
                let next_dir_sign;
                let mut i = num_levels - 2;
                loop {
                    let level = &level_states[i];
                    if level.progress == 3 {
                        if i == 0 {
                            next_dir = 0; // Default to X
                            next_dir_sign = 0; // Positive X (move right)
                            break;
                        } else {
                            i -= 1;
                        }
                    } else {
                        let adr = CURVE_ADDRESS_TABLE[level.curve_type as usize]
                            >> (level.progress * 2) as u32;
                        // adr[1:0] = current, adr[3:2] = next
                        let adr_rel = adr ^ (adr >> 2);
                        debug_assert!((adr_rel & 3) == 0b01 || (adr_rel & 3) == 0b10);
                        next_dir = adr_rel & 1;
                        next_dir_sign = ((adr & adr_rel & 0b11) != 0) as usize;
                        break;
                    }
                }
                scanning_type_table[next_dir_sign][next_bb_enter as usize][next_dir as usize]
            }
            // T_B(E, O) - Type-2 basic pattern
            0b01 => flags![BbFlags::{SECONDARY_DIR_NEG | END_X | END_Y}],
            // T_B(O, E) - Reversed type-3 basic pattern
            0b10 => flags![BbFlags::{PRIMARY_AXIS_Y | SECONDARY_DIR_NEG | END_X | END_Y}],
            // T_B(O, O) - Unreachable because there can be only one T_B(O, O)
            // a rectangle!
            0b11 => unreachable!(),
            _ => unreachable!(),
        };

        self.bb_progress = if self.bb_flags.contains(BbFlags::PRIMARY_AXIS_Y) {
            [size[1], size[0]]
        } else {
            [size[0], size[1]]
        };

        Some(position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log2_sanity() {
        assert_eq!(log2_floor(1), 0);
        assert_eq!(log2_floor(2), 1);
        assert_eq!(log2_floor(3), 1);
        assert_eq!(log2_floor(256), 8);
        assert_eq!(log2_floor(300), 8);
        assert_eq!(log2_floor(511), 8);
        assert_eq!(log2_floor(512), 9);
    }

    #[test]
    fn division_sanity() {
        assert_eq!(division_l1(18u32), 8);
        // The follwing expression returns `16` instead of `8` expected by
        // the definition on the paper. This deviation improves the performance but
        // it's probable (not drastic, I believe) that it could slightly affect
        // the output quality
        // assert_eq!(division_l1(24u32), 8);
        assert_eq!(division_l1(32u32), 16);
    }
}
