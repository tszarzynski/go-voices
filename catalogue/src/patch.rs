//! One patch and how to select it.

use alloc::{format, string::String, vec, vec::Vec};
use core::{fmt, num::ParseIntError, str::FromStr};

use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::Category;

/// A sound the keyboard can play.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Patch {
    /// As the keyboard (or the JUNO-DS) names it.
    pub name: String,
    /// What kind of sound.
    pub category: Category,
    /// Bank select MSB (CC 0).
    pub msb: u8,
    /// Bank select LSB (CC 32).
    pub lsb: u8,
    /// Program number, 1–128 (sent as 0–127).
    pub pc: u8,
}

/// Which sound set a patch comes from, told by its bank select MSB.
#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub enum Bank {
    /// The JUNO-DS patches (MSB 87).
    JunoDs,
    /// General MIDI 2 (MSB 121).
    Gm2,
    /// General MIDI 2 drum kits (MSB 120).
    Gm2Drums,
}

impl Bank {
    /// Every bank, in display order.
    pub const ALL: [Self; 3] = [Self::JunoDs, Self::Gm2, Self::Gm2Drums];

    /// The bank with this bank select MSB, if the keyboard has one.
    #[must_use]
    pub const fn from_msb(msb: u8) -> Option<Self> {
        match msb {
            87 => Some(Self::JunoDs),
            121 => Some(Self::Gm2),
            120 => Some(Self::Gm2Drums),
            _ => None,
        }
    }

    /// What to call it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::JunoDs => "JUNO-DS",
            Self::Gm2 => "GM2",
            Self::Gm2Drums => "GM2 Drums",
        }
    }
}

impl Patch {
    /// Which sound set it's from: always one for a patch from
    /// [`Patch::from_str`], which checks the MSB.
    #[must_use]
    pub const fn bank(&self) -> Option<Bank> {
        Bank::from_msb(self.msb)
    }

    /// The MIDI that selects it on `channel` (1–16): bank select MSB and
    /// LSB, then program change.
    ///
    /// # Panics
    ///
    /// If `channel` isn't 1–16.
    #[must_use]
    pub fn select_bytes(&self, channel: u8) -> Vec<u8> {
        assert!((1..=16).contains(&channel), "MIDI channel {channel}");
        let n = channel - 1;
        vec![
            0xB0 | n,
            0,
            self.msb,
            0xB0 | n,
            32,
            self.lsb,
            0xC0 | n,
            self.pc - 1,
        ]
    }

    /// The address, as `87·64·004`.
    #[must_use]
    pub fn address(&self) -> String {
        format!("{}·{}·{:03}", self.msb, self.lsb, self.pc)
    }
}

/// Why a line isn't a patch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatchError {
    /// Not five tab-separated fields.
    Fields(usize),
    /// The name is empty.
    Name,
    /// Not a category code the keyboard uses.
    Category(String),
    /// A number didn't parse.
    Number(ParseIntError),
    /// Not a bank this keyboard has.
    Bank(u8),
    /// The LSB is over 127.
    Lsb(u8),
    /// The program isn't 1–128.
    Program(u8),
}

impl fmt::Display for PatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fields(n) => write!(f, "{n} fields, not 5"),
            Self::Name => f.write_str("no name"),
            Self::Category(code) => write!(f, "unknown category {code:?}"),
            Self::Number(e) => write!(f, "not a number: {e}"),
            Self::Bank(msb) => write!(f, "no bank with MSB {msb}"),
            Self::Lsb(lsb) => write!(f, "LSB {lsb} over 127"),
            Self::Program(pc) => write!(f, "program {pc} not 1–128"),
        }
    }
}

impl core::error::Error for PatchError {}

impl From<ParseIntError> for PatchError {
    fn from(e: ParseIntError) -> Self {
        Self::Number(e)
    }
}

/// A line of `patches.tsv`: name, category code, MSB, LSB and program,
/// separated by tabs.
impl FromStr for Patch {
    type Err = PatchError;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
        let fields: Vec<&str> = line.split('\t').map(str::trim).collect();
        let &[name, category, msb, lsb, pc] = fields.as_slice() else {
            return Err(PatchError::Fields(fields.len()));
        };
        if name.is_empty() {
            return Err(PatchError::Name);
        }
        let category =
            Category::from_code(category).ok_or_else(|| PatchError::Category(category.into()))?;
        let (msb, lsb, pc) = (msb.parse()?, lsb.parse()?, pc.parse()?);
        if Bank::from_msb(msb).is_none() {
            return Err(PatchError::Bank(msb));
        }
        if lsb > 127 {
            return Err(PatchError::Lsb(lsb));
        }
        if !(1..=128).contains(&pc) {
            return Err(PatchError::Program(pc));
        }
        Ok(Self {
            name: name.into(),
            category,
            msb,
            lsb,
            pc,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patch(line: &str) -> Patch {
        line.parse().unwrap()
    }

    #[test]
    fn selects_with_bank_select_and_program_change() {
        let piano = patch("JUNO Piano 1\tPNO\t087\t064\t004");
        assert_eq!(piano.select_bytes(1), [0xB0, 0, 87, 0xB0, 32, 64, 0xC0, 3]);
        assert_eq!(piano.select_bytes(16), [0xBF, 0, 87, 0xBF, 32, 64, 0xCF, 3]);
    }

    #[test]
    fn program_128_is_sent_as_127() {
        assert_eq!(patch("X\tSYN\t87\t69\t128").select_bytes(1)[7], 127);
    }

    #[test]
    fn rejects_what_the_keyboard_has_not() {
        let error = |line: &str| line.parse::<Patch>().unwrap_err();
        assert_eq!(error("A\tPNO\t87\t64"), PatchError::Fields(4));
        assert_eq!(error("\tPNO\t87\t64\t1"), PatchError::Name);
        assert_eq!(
            error("A\tXYZ\t87\t64\t1"),
            PatchError::Category("XYZ".into())
        );
        assert_eq!(error("GO Grand\tPNO\t0\t68\t1"), PatchError::Bank(0));
        assert_eq!(error("A\tPNO\t87\t128\t1"), PatchError::Lsb(128));
        assert_eq!(error("A\tPNO\t87\t64\t0"), PatchError::Program(0));
        assert!(matches!(error("A\tPNO\t87\t64\tx"), PatchError::Number(_)));
    }

    #[test]
    fn shows_its_address() {
        assert_eq!(patch("A\tPNO\t87\t64\t4").address(), "87·64·004");
    }
}
