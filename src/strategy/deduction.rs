//! Results of strategy applications

use crate::board::{Candidate};
use super::Strategy;
use crate::board::*;
use crate::bitset::Set;

type DeductionRange = std::ops::Range<usize>;
type _Deduction = Deduction<DeductionRange>;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
/// Contains the sequence of deductions made to solve / partially solve the sudoku
pub struct Deductions {
	pub(crate) deductions: Vec<_Deduction>,
	pub(crate) deduced_entries: Vec<Candidate>,
	pub(crate) eliminated_entries: Vec<Candidate>,
}

/// Borrowing iterator over [`Deductions`]
pub struct Iter<'a> {
	deductions: std::slice::Iter<'a, _Deduction>,
	eliminated_entries: &'a [Candidate]
}

impl<'a> Iterator for Iter<'a> {
	type Item = Deduction<&'a [Candidate]>;

	fn next(&mut self) -> Option<Self::Item> {
		self.deductions.next()
			.map(|deduction| deduction.clone().with_slices(self.eliminated_entries))
	}
}

impl Deductions {
	/// Returns the number of deductions.
	pub fn len(&self) -> usize {
		self.deductions.len()
	}

	/// Return the `index`th Deduction, if it exists.
	pub fn get(&self, index: usize) -> Option<Deduction<&[Candidate]>> {
		self.deductions.get(index)
			.map(|deduction| deduction.clone().with_slices(&self.eliminated_entries))
	}

	/// Return an iterator over the deductions.
	pub fn iter(&self) -> Iter<'_> {
		Iter {
			deductions: self.deductions.iter(),
			eliminated_entries: &self.eliminated_entries,
		}
	}
}

/// Result of a single, successful strategy application
///
/// This enum contains the data necessary to explain why the step could be taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Deduction<T> {
	/// Result of [`NakedSingles`](super::Strategy::NakedSingles)
    NakedSingles(Candidate),
	/// Result of [`HiddenSingles`](super::Strategy::HiddenSingles)
    HiddenSingles(Candidate, HouseType),
	/// Result of [`LockedCandidates`](super::Strategy::LockedCandidates)
    LockedCandidates {
		digit: Digit,
		/// The miniline which is the only one in the block or line, that contains `digit`
		miniline: MiniLine,
		/// In the "Pointing" variant, only one miniline in a block can contain the digit and all candidates
		/// in other blocks in the same line are impossible. In the "Claiming" variant, it's the other way around.
		is_pointing: bool,
		conflicts: T,
	}, // which miniline is affected and what's unique

	/// Result of naked or hidden subsets, i.e. [`NakedPairs`](super::Strategy::NakedPairs), [`NakedTriples`](super::Strategy::NakedTriples), [`NakedQuads`](super::Strategy::NakedQuads),
	/// [`HiddenPairs`](super::Strategy::HiddenPairs), [`HiddenTriples`](super::Strategy::HiddenTriples) or [`HiddenQuads`](super::Strategy::HiddenQuads).
    Subsets {
		/// A house that contains all cells of the locked set.
		house: House,
		/// The cells that contain the locked set. Can be 2-4 positions.
		positions: Set<Position<House>>,
		/// The digits that are part of the locked set. The number of digits is always equal to the number of
		/// positions
		digits: Set<Digit>,
		conflicts: T,
	},
	/// Result of [`XWing`](super::Strategy::XWing), [`Swordfish`](super::Strategy::Swordfish) or [`Jellyfish`](super::Strategy::Jellyfish)
    BasicFish {
		digit: Digit,
		/// The lines that contain the fish. Can be 2-4 lines.
		lines: Set<Line>,
		/// The union of possible positions in the `lines`. The number of positions is always equal to the number
		/// of lines.
		positions: Set<Position<Line>>,
		conflicts: T,
	},

    //SinglesChain(T),
    #[doc(hidden)] __NonExhaustive
}

impl Deduction<&'_ [Candidate]> {
	/// Returns the type of strategy that was used to make this deduction.
	pub fn strategy(&self) -> Strategy {
		use self::Deduction::*;
		match self {
			NakedSingles { .. } => Strategy::NakedSingles,
			HiddenSingles { .. } => Strategy::HiddenSingles,
			LockedCandidates { .. } => Strategy::LockedCandidates,
			BasicFish { positions, .. } => {
				match positions.len() {
					2 => Strategy::XWing,
					3 => Strategy::Swordfish,
					4 => Strategy::Jellyfish,
					_ => unreachable!(),
				}
			}
			//SinglesChain { .. } => Strategy::SinglesChain,
			Subsets { house, positions, conflicts, .. } => {
				use crate::board::positions::HouseType::*;
				let conflict_cell = conflicts[0].cell;
				let conflict_pos = match house.categorize() {
					Row(_) => conflict_cell.row_pos(),
					Col(_) => conflict_cell.col_pos(),
					Block(_) => conflict_cell.block_pos(),
				};
				let is_hidden_subset = conflict_pos.as_set().overlaps(*positions);
				match (is_hidden_subset, positions.len()) {
					(false, 2) => Strategy::NakedPairs,
					(false, 3) => Strategy::NakedTriples,
					(false, 4) => Strategy::NakedQuads,
					(true, 2) => Strategy::HiddenPairs,
					(true, 3) => Strategy::HiddenTriples,
					(true, 4) => Strategy::HiddenQuads,
					_ => unreachable!(),
				}
			}
			/*HiddenSubsets { digits, .. } => {
				match digits.len() {
					2 => Strategy::HiddenPairs,
					3 => Strategy::HiddenTriples,
					4 => Strategy::HiddenQuads,
					_ => unreachable!(),
				}
			}*/
			__NonExhaustive => unreachable!(),
		}
	}
}

impl _Deduction {
	/// Replace the index ranges from the internal representation with slices
	/// for the external API
	fn with_slices(self, eliminated: &[Candidate]) -> Deduction<&[Candidate]> {
		use self::Deduction::*;
		match self {
			NakedSingles(c) => NakedSingles(c),
			HiddenSingles(c, h) => HiddenSingles(c, h),

			LockedCandidates {
				miniline, digit, is_pointing,
				conflicts
			} => LockedCandidates { miniline, digit, is_pointing, conflicts: &eliminated[conflicts] },

			Subsets {
				house, positions, digits,
				conflicts
			}
			=> Subsets { house, positions, digits, conflicts: &eliminated[conflicts]},

			BasicFish {
				lines, positions, digit,
				conflicts
			}
			=> BasicFish { lines, positions, digit, conflicts: &eliminated[conflicts]},

			//SinglesChain(x) => SinglesChain(&eliminated[x]),
			__NonExhaustive => __NonExhaustive
		}
	}
}