//! The sounds a Roland GO:KEYS plays over MIDI: its JUNO-DS patches and the
//! GM2 set, each with the bank select and program change that picks it.
//!
//! The list comes from the GO:Plus project's `GO-sounds.md`, converted by
//! `tools/import-sounds` into `data/patches.tsv`. The waves the sounds are
//! built from, with their names, are in `data/waves.tsv` ([`Waves`]).
//!
//! Builds without `std` (`default-features = false`).

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod category;
mod patch;
mod search;
mod wave;

pub use category::Category;
pub use patch::{Bank, Patch, PatchError};
pub use search::Query;
pub use wave::{Wave, Waves};

use alloc::vec::Vec;

/// The patches in `data/patches.tsv`, in the keyboard's order.
const PATCHES_TSV: &str = include_str!("../data/patches.tsv");

/// Every patch, in catalogue order. A patch's id is its index here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catalogue {
    patches: Vec<Patch>,
}

impl Catalogue {
    /// The built-in catalogue.
    ///
    /// # Panics
    ///
    /// If `data/patches.tsv` has a bad row, which its importer and tests
    /// rule out.
    #[must_use]
    pub fn builtin() -> Self {
        Self::parse(PATCHES_TSV).unwrap_or_else(|(line, e)| panic!("patches.tsv:{line}: {e}"))
    }

    /// Reads one patch per line: name, category code, MSB, LSB and program
    /// (1–128), separated by tabs.
    ///
    /// # Errors
    ///
    /// The first bad line's number (from 1) and what's wrong with it.
    pub fn parse(tsv: &str) -> Result<Self, (usize, PatchError)> {
        let patches = tsv
            .lines()
            .enumerate()
            .filter(|(_, line)| !line.is_empty())
            .map(|(i, line)| line.parse().map_err(|e| (i + 1, e)))
            .collect::<Result<_, _>>()?;
        Ok(Self { patches })
    }

    /// Every patch, in catalogue order.
    #[must_use]
    pub fn patches(&self) -> &[Patch] {
        &self.patches
    }

    /// The patch with this id.
    #[must_use]
    pub fn get(&self, id: usize) -> Option<&Patch> {
        self.patches.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_builtin_catalogue_has_every_sound() {
        let catalogue = Catalogue::builtin();
        assert_eq!(catalogue.patches().len(), 1383);
        let first = &catalogue.patches()[0];
        assert_eq!(first.name, "88StageGrand");
        assert_eq!((first.msb, first.lsb, first.pc), (87, 64, 1));
    }

    #[test]
    fn every_builtin_sound_has_its_own_address() {
        let mut seen = alloc::collections::BTreeMap::new();
        for patch in Catalogue::builtin().patches() {
            let address = (patch.msb, patch.lsb, patch.pc);
            if let Some(other) = seen.insert(address, &patch.name) {
                panic!("{} and {other} share {address:?}", patch.name);
            }
        }
    }

    #[test]
    fn the_builtin_catalogue_covers_every_bank() {
        let catalogue = Catalogue::builtin();
        for bank in [Bank::JunoDs, Bank::Gm2, Bank::Gm2Drums] {
            assert!(
                catalogue.patches().iter().any(|p| p.bank() == Some(bank)),
                "{bank:?}"
            );
        }
    }

    #[test]
    fn a_bad_line_is_reported_by_number() {
        let tsv = "Piano\tPNO\t87\t64\t1\nGO Grand\tPNO\t0\t68\t1\n";
        assert_eq!(Catalogue::parse(tsv), Err((2, PatchError::Bank(0))));
    }
}
