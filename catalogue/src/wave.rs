//! The waves a tone is built from, by group and number, with their names.
//!
//! The names come from GO Tone Bridge's `go-wave-names.json`
//! (<https://github.com/madbadware/go-tone-bridge>, MIT), converted into
//! `data/waves.tsv`: group, number (from 1), name.

use alloc::vec::Vec;

/// The waves in `data/waves.tsv`.
pub const WAVES_TSV: &str = include_str!("../data/waves.tsv");

/// One wave.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wave {
    /// Its group: 1 or 2.
    pub group: u16,
    /// Its number in the group, from 1.
    pub number: u16,
    /// e.g. `Fng.EB1 mf A`. A stereo pair is two waves, `… L` and `… R`.
    pub name: &'static str,
}

impl Wave {
    /// The left wave of a stereo pair.
    #[must_use]
    pub fn is_left(&self) -> bool {
        self.name.ends_with(" L")
    }

    /// The right wave of a stereo pair, which goes with its left one.
    #[must_use]
    pub fn is_right(&self) -> bool {
        self.name.ends_with(" R")
    }
}

/// Every wave, in group and number order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Waves(Vec<Wave>);

impl Waves {
    /// The built-in list.
    ///
    /// # Panics
    ///
    /// If `data/waves.tsv` has a bad row, which its tests rule out.
    #[must_use]
    pub fn builtin() -> Self {
        Self(
            WAVES_TSV
                .lines()
                .filter(|line| !line.is_empty())
                .map(|line| {
                    let mut fields = line.splitn(3, '\t');
                    let mut number = || {
                        fields
                            .next()
                            .and_then(|f| f.parse().ok())
                            .expect("a number")
                    };
                    let (group, number) = (number(), number());
                    Wave {
                        group,
                        number,
                        name: fields.next().expect("a name"),
                    }
                })
                .collect(),
        )
    }

    /// Every wave.
    #[must_use]
    pub fn all(&self) -> &[Wave] {
        &self.0
    }

    /// The wave with this group and number.
    #[must_use]
    pub fn get(&self, group: u16, number: u16) -> Option<&Wave> {
        self.0
            .iter()
            .find(|wave| wave.group == group && wave.number == number)
    }

    /// For a left wave, the right one that goes with it.
    #[must_use]
    pub fn right_of(&self, wave: &Wave) -> Option<&Wave> {
        let stem = wave.name.strip_suffix(" L")?;
        let right = self.get(wave.group, wave.number + 1)?;
        (right.name.strip_suffix(" R") == Some(stem)).then_some(right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_waves_and_pairs_them() {
        let waves = Waves::builtin();
        assert_eq!(waves.all().len(), 1045);
        // The finger bass the keyboard's "Cmp'd Fng Bs" starts from.
        assert_eq!(waves.get(1, 291).unwrap().name, "Fng.EB1 mf A");
        let left = waves.get(1, 1).unwrap();
        assert_eq!(left.name, "Ac.Pno*p A L");
        assert_eq!(waves.right_of(left).unwrap().number, 2);
        assert_eq!(waves.right_of(waves.get(1, 291).unwrap()), None);
        assert_eq!(waves.get(2, 1).unwrap().name, "GrandP* 1 mp");
    }
}
