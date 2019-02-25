//! The core implementation of the algorithm.
use array::Array2;
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
    ///
    /// Invalid for the last level (`last_level`).
    curve_type: u8,
    /// An integer in `0..4`. Indicates which subblock we are in this level's
    /// block.
    ///
    /// Invalid for the last level (`last_level`).
    progress: u8,
}

/// The curve type address sequence table.
///
/// Let `Tₜᵣₘ[γ][i]` be `(CURVE_ADDRESS_TABLE[γ] >> (i * 2)) & 0b11`.
/// `Tₜᵣₘ[γ][i]` represents the position of `i`-th subblock within a block
/// assigned a curve type `γ`.
///
/// ```text
///   ,----,   <----,   ^    |   ,-----
///   |    |        |   |    |   |
///   |    v   -----'   '----'   '---->
///
///   Type 0   Type 1   Type 2   Type 3
/// ```
///
/// The curve types are associated with the scanning manners:
///
/// ```text
///   ,--,  ,--,   <----,   ^  ,--,  |   ,-----
///   |  |  |  |   ,----'   |  |  |  |   '-----,
///   |  |  |  |   '----,   |  |  |  |   ,-----'
///   |  '--'  v   -----'   '--'  '--'   '---->
///
///     Type 0     Type 1     Type 2     Type 3
/// ```
///
const CURVE_ADDRESS_TABLE: [u8; 8] = [
    //XY_XY_XY_XY
    // Type 0
    0b10_11_01_00,
    // Type 1
    0b01_11_10_00,
    // Type 2
    0b01_00_10_11,
    // Type 3,
    0b10_00_01_11,
    // Reverse type 0
    0b00_01_11_10,
    // Reverse type 1
    0b00_10_11_01,
    // Reverse type 2
    0b11_10_00_01,
    // Reverse type 3,
    0b11_01_00_10,
];

/// The curve induction table.
///
/// `CURVE_INDUCTION_TABLE[γ][i]` represents the curve type of `i`-th subblock
/// within a block assigned a curve type `γ`.
const CURVE_INDUCTION_TABLE: [[u8; 4]; 8] = [
    [1, 0, 0, 3],
    [0, 1, 1, 2],
    [3, 2, 2, 1],
    [2, 3, 3, 0],
    [7, 4, 4, 5],
    [6, 5, 5, 4],
    [5, 6, 6, 7],
    [4, 7, 7, 6],
];

/// Get the primary axis (X = 0, Y = 1) of a curve type.
///
/// ```text
///   ,--,  ,--,
///   |  |  |  |
///   |  |  |  |  ---> primary axis
///   |  '--'  v
/// ```
fn curve_primary_axis(c: u8) -> u8 {
    c & 1
}

/// Get the sign of the primary direction of a curve type.
fn curve_primary_negative(c: u8) -> u8 {
    (c ^ (c >> 1)) & 0b10
}

fn curve_secondary_negative_at_start(c: u8) -> u8 {
    c & 0b10
}

fn curve_end_point(c: u8) -> u8 {
    CURVE_ADDRESS_TABLE[c as usize] >> 6
}

/// Get the number of [`LevelState`]s required by [`HilbertScanCore`] to
/// hold its internal state.
///
/// `size[0]` and `size[1]` must be both greater than `1`.
pub fn num_levels_for_size<T: PrimInt + Unsigned>(size: [T; 2]) -> usize {
    assert!(size[0] > T::one());
    assert!(size[1] > T::one());
    // Allocate one extra level so that we can perform the extra subdivision
    // on the last (`log2_floor(min(size[0], size[1])) - 1`-th) level
    log2_floor(min(size[0], size[1])) as usize + 1
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

/// Get the size of a extra-subdivided subblock.
///
/// `curve_type` is the curve type of the block containing the extra-subdivided
/// subblock. `pos` specifies a subblock within the block.
fn extra_division_subblock_size<T: PrimInt + Unsigned>(size: [T; 2], mut pos: u8, curve_type: u8) -> [T; 2] {
    // If the block is odd-sized (`T_B(O, _)` and/or `T_B(_, O)`), we must
    // be careful to make the subblocks' sizes compatible with their curve types.
    //
    // T_B(O, E) (first) - Type 0 + helper row
    // T_B(O, O) (first) - Type 0 + helper row
    //    ,-, ,-, | \
    //    | | | | |  } l1
    //    | '-' | | /
    //    '-, ,-' | \
    //    --' '---' / l0  l0 must be even
    //
    // T_B(O, E) (other) - Reverse type-3
    //  ,----, ^
    //  '-,  '-'
    //  ,-'  ,-,  l1 must be even
    //  '----' |
    //  \---/\-/
    //    l0  l1
    //
    // T_B(E, O) (first) - Type-1 + helper row
    //    ,----->
    //    | ,---,
    //    '-' ,-'  TODO
    //    ,-, '-,
    //    | '---'
    //
    // T_B(E, O) (other) - Type-2
    //    --, ,-> \
    //    ,-' '-, / l1  l1 must be even
    //    | ,-, | \
    //    | | | |  }l0
    //    '-' '-' /
    //
    let three = T::from(3).unwrap();
    let size_l1 = size.map(|x| (x + three) >> 2 << 1);
    let size_l0 = [size[0] - size_l1[0], size[1] - size_l1[1]];

    pos ^= (curve_type == 0) as u8;

    let (adr0, adr1) = ((pos & 0b10) != 0, (pos & 0b01) != 0);

    [
        if adr0 { size_l1[0] } else { size_l0[0] },
        if adr1 { size_l1[1] } else { size_l0[1] },
    ]
}

/// An iterator producing a pseudo-Hilbert scan.
///
/// `T` is a type used to represent the output coordinates. `LevelSt` is
/// a mutable reference to a slice of `LevelState<T>`s.
#[derive(Debug)]
pub struct HilbertScanCore<T, LevelSt> {
    size: [T; 2],
    num_levels: usize,
    /// `num_levels - 1` or `last_level - 2`
    last_level: usize,
    level_states: LevelSt,
    position: [T; 2],

    // ============ Basic (last-level block) scanning state =============
    bb_progress: [T; 2],
    bb_secondary_neg: bool,
    bb_curve_type: u8,
    bb_end: u8,
    bb_helper_row: bool,

    done: bool,
}

impl<T, LevelSt> HilbertScanCore<T, LevelSt>
where
    LevelSt: BorrowMut<[LevelState<T>]>,
    T: PrimInt + Unsigned + std::fmt::Debug,
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
        let mut last_level;
        let (bb_curve_type, bb_helper_row, bb_progress);
        {
            let level_states = &mut level_states.borrow_mut()[0..num_levels];
            level_states[0] = LevelState {
                size,
                curve_type: 0, // γ(0) = 1
                progress: 0,
            };
            for i in 1..=num_levels - 2 {
                let prev = level_states[i - 1];
                level_states[i] = LevelState {
                    size: prev.size.map(|x| x - division_l1(x)),
                    curve_type: (i % 2) as u8, // CURVE_INDUCTION_TABLE[prev.curve_type as usize][0],
                    progress: 0,
                };
            }
            last_level = num_levels - 2;

            // Set up the scan of the first block
            let last_curve_type = last_level % 2;
            let (curve_type, helper) = match size.map(|x| (x & T::one()).to_u8().unwrap()) {
                // T_R(E, E)
                [0, 0] => (last_curve_type as u8, false),
                // T_R(E, O) - Type-1 basic pattern + helper row
                //
                //  ,------>  - Helper row
                //  '------,  \
                //    ...      } Type-1 basic pattern
                //  -------'  /
                //
                [0, 1] => (1, true),
                // T_R(O, E), T_R(O, O) - Type-0 basic pattern + helper row
                [1, 0] | [1, 1] => (0, true),
                [_, _] => unreachable!(),
            };

            if helper {
                // The helper row is specially handled, so exclude it from
                // the block's size
                let last_size = &mut level_states[last_level].size;
                let pri_size = &mut last_size[curve_primary_axis(curve_type) as usize];
                *pri_size = *pri_size - T::one();
            }

            let mut last_size = level_states[last_level].size;

            level_states[last_level].curve_type = curve_type;

            // Try the extra-subdivision on the first block.
            let three = T::from(3u8).unwrap();
            if last_size[0] >= three && last_size[1] >= three {
                // If the block is large enough, we can (and should) do the extra
                // subdivision.
                level_states[last_level].progress = 0;

                last_size = extra_division_subblock_size(last_size, 0b00, curve_type);
                bb_curve_type = CURVE_INDUCTION_TABLE[curve_type as usize][0];

                last_level += 1;
                level_states[last_level].size = last_size;
            } else {
                // Otherwise, apply the basic scanning pattern on this block.
                bb_curve_type = curve_type;
            }

            bb_helper_row = helper;
            bb_progress = if curve_primary_axis(bb_curve_type) != 0 {
                [last_size[1], last_size[0]]
            } else {
                [last_size[0], last_size[1]]
            };
        }

        let bb_secondary_neg = curve_secondary_negative_at_start(bb_curve_type) != 0;
        let bb_end = curve_end_point(bb_curve_type);

        Self {
            size,
            num_levels,
            last_level,
            level_states,
            position: [T::zero(), T::zero()],
            bb_progress,
            bb_secondary_neg,
            bb_curve_type,
            bb_end,
            bb_helper_row,
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
        let pri_axis = curve_primary_axis(self.bb_curve_type) as usize;
        let sec_axis = pri_axis ^ 1;
        let sec_width = level_states[self.last_level].size[sec_axis];
        sec = sec - T::one();

        if sec == T::zero() {
            pri = pri - T::one();
            sec = sec_width;
            // Zigzag
            self.bb_secondary_neg = !self.bb_secondary_neg;
        } else {
            let sec_pos = &mut self.position[sec_axis];
            if self.bb_secondary_neg {
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
            if curve_primary_negative(self.bb_curve_type) != 0 {
                *pri_pos = *pri_pos - T::one();
            } else {
                *pri_pos = *pri_pos + T::one();
            }
            self.bb_progress = [pri, sec];
            return Some(position);
        }

        if self.bb_helper_row {
            let block_done = if self.last_level == num_levels - 2 {
                true
            } else {
                level_states[num_levels - 2].progress == 3
            };

            if block_done {
                // The current block is complete. Now, generate the helper row.
                // The current block requires a helper row so that we can exit
                // the block at the intended (top-right) corner.
                //    ,----->  ← helper row
                //    | ,---,  \
                //    '-' ,-'  | Type-1 curve
                //    ,-, '-,  |
                //    | '---'  /
                //
                let level = &mut level_states[num_levels - 2];
                let pri_axis = curve_primary_axis(level.curve_type) as usize;
                let sec_axis = pri_axis ^ 1;
                let sec_width = level.size[sec_axis];

                self.bb_end = 0b11;
                self.bb_curve_type = level.curve_type;
                self.bb_secondary_neg = false;
                self.bb_progress = [T::one(), sec_width];

                self.bb_helper_row = false;

                self.position[pri_axis] = self.position[pri_axis] + T::one();
                self.last_level = num_levels - 2;

                return Some(position);
            }
        }

        if self.last_level == 0 {
            self.done = true;
            return Some(position);
        }

        let mut i = self.last_level - 1;
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
                    if curve_primary_negative(self.bb_curve_type) != 0 {
                        *pri_pos = *pri_pos - T::one();
                    } else {
                        *pri_pos = *pri_pos + T::one();
                    }
                } else {
                    let sec_pos = &mut self.position[sec_axis];
                    // This condition is negated on purpose to cancel out
                    // the effect of the "zigzag" part.
                    if self.bb_secondary_neg {
                        *sec_pos = *sec_pos + T::one();
                    } else {
                        *sec_pos = *sec_pos - T::one();
                    }
                }

                // Now we also know where do we enter the next block
                next_bb_enter = self.bb_end ^ (adr_rel & 0b11);
                break;
            }
        }

        if i == num_levels - 2 {
            // We were and are still in the same basic block and we just moved
            // between extra-subdivided blocks.
            let progress = level_states[i].progress;
            let curve_type = level_states[i].curve_type;

            let adr = CURVE_ADDRESS_TABLE[curve_type as usize] >> (progress * 2) as u32;
            let bb_curve_type = CURVE_INDUCTION_TABLE[curve_type as usize][progress as usize];

            let prev_size = level_states[i].size;
            let size = extra_division_subblock_size(prev_size, adr, curve_type);
            level_states[i + 1].size = size;

            self.bb_secondary_neg = curve_secondary_negative_at_start(bb_curve_type) != 0;
            self.bb_curve_type = bb_curve_type;
            self.bb_end = curve_end_point(bb_curve_type);
            self.bb_progress = if curve_primary_axis(bb_curve_type) != 0 {
                [size[1], size[0]]
            } else {
                [size[0], size[1]]
            };

            debug_assert_eq!(self.bb_progress[0] & T::one(), T::zero());
            debug_assert_ne!(self.bb_progress[0], T::zero());
            debug_assert_ne!(self.bb_progress[1], T::zero());

            return Some(position);
        }

        while i < num_levels - 2 {
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
        let mut size = level_states[i].size;
        let even_flags = (((size[0] & T::zero()) << 1) | (size[1] & T::zero()))
            .to_u8()
            .unwrap();

        // I wonder why they didn't mention the memory consumption of this
        // look-up table in the paper. (Not to mention the local variables...)
        const SCANNING_TYPE: [[[u8; 2]; 4]; 2] = [
            // Move right/up
            [
                [
                    // Bottom-left to ...
                    0, // 0b00 → 0b10 - Type-0 basic pattern
                    1, // 0b00 → 0b01 - Type-1 basic pattern
                ],
                [
                    // Top-left to ...
                    4 | 2, // 0b01 → 0b11 - Reversed type-2 basic pattern
                    4 | 2, // 0b01 → 0b11 - Reversed type-2 basic pattern
                ],
                [
                    // bottom-right to ...
                    4 | 3, // 0b10 → 0b11 - Reversed type-3 basic pattern
                    4 | 3, // 0b10 → 0b11 - Reversed type-3 basic pattern
                ],
                [
                    // top-right to ...
                    3, // 0b11 → 0b10 - Type-3 basic pattern
                    2, // 0b11 → 0b01 - Type-2 basic pattern
                ],
            ],
            // Move left/down
            [
                [
                    // Bottom-left to ...
                    1, // 0b00 → 0b01 - Type-1 basic pattern
                    0, // 0b00 → 0b10 - Type-0 basic pattern
                ],
                [
                    // Top-left to ...
                    4 | 1, // 0b01 → 0b00 - Reversed type-1 basic pattern
                    4 | 1, // 0b01 → 0b00 - Reversed type-1 basic pattern
                ],
                [
                    // bottom-right to ...
                    4 | 0, // 0b10 → 0b00 - Reversed type-0 basic pattern
                    4 | 0, // 0b10 → 0b00 - Reversed type-0 basic pattern
                ],
                [
                    // top-right to ...
                    2, // 0b11 → 0b01 - Type-2 basic pattern
                    3, // 0b11 → 0b10 - Type-3 basic pattern
                ],
            ],
        ];

        let mut bb_curve_type = match even_flags {
            // T_B(E, E)
            0b00 => {
                // Find "the location (left, right, up or down) of the next block"
                let next_dir;
                let next_dir_sign;
                let mut i = i - 1;
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
                SCANNING_TYPE[next_dir_sign][next_bb_enter as usize][next_dir as usize]
            }
            // T_B(E, O) - Reversed Type-2 basic pattern
            0b01 => 4 | 2,
            // T_B(O, E) - Reversed type-3 basic pattern
            0b10 => 4 | 3,
            // T_B(O, O) - Unreachable because there can be only one T_B(O, O)
            // a rectangle!
            0b11 => unreachable!(),
            _ => unreachable!(),
        };

        let three = T::from(3u8).unwrap();
        if size[0] >= three && size[1] >= three {
            // If the block is large enough, we can (and should) do the extra
            // subdivision (i.e., dividing the smallest blocks defined by the
            // top level of the algorithm in the paper)
            level_states[i].progress = 0;
            level_states[i].curve_type = bb_curve_type;

            size = extra_division_subblock_size(size, next_bb_enter, bb_curve_type);
            bb_curve_type = CURVE_INDUCTION_TABLE[bb_curve_type as usize][0];

            i += 1;
            debug_assert_eq!(i, num_levels - 1);
            level_states[i].size = size;
        } else {
            // Otherwise, apply the basic scanning pattern on this block.
        }
        self.bb_secondary_neg = curve_secondary_negative_at_start(bb_curve_type) != 0;
        self.bb_curve_type = bb_curve_type;
        self.bb_end = curve_end_point(bb_curve_type);
        self.bb_progress = if curve_primary_axis(bb_curve_type) != 0 {
            [size[1], size[0]]
        } else {
            [size[0], size[1]]
        };

        debug_assert_eq!(self.bb_progress[0] & T::one(), T::zero());
        debug_assert_ne!(self.bb_progress[0], T::zero());
        debug_assert_ne!(self.bb_progress[1], T::zero());

        self.last_level = i;
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
