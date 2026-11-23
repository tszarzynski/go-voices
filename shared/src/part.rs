//! The settings the Edit panel changes.
//!
//! Each is described once (where it lives on the keyboard, how it's
//! encoded, its range, its default and its words), so the view, the SysEx
//! and the settings all follow from here.
//!
//! The addresses and encodings are GO:Plus's (`goplus.py`: `PART_MODEL`,
//! `SYS_CTRL_MODEL`).

use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};

use facet::Facet;
use serde::{Deserialize, Serialize};

/// One setting the app can change.
#[derive(
    Facet, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[repr(C)]
pub enum Control {
    /// The part's volume, 0–127.
    Level,
    /// Left to right, 0–127 (64 is the middle).
    Pan,
    /// Mono, poly, or the sound's own.
    Mono,
    /// Legato off, on, or the sound's own.
    Legato,
    /// Glide (portamento) off, on, or the sound's own.
    Glide,
    /// How long a glide takes, 0–127, or 128 for the sound's own.
    GlideTime,
    /// How much the keys' velocity changes the sound, −63 to +63 from the
    /// sound's own.
    VelocitySens,
    /// The filter's cutoff, −64 to +63 from the sound's own.
    Cutoff,
    /// The filter's resonance, −64 to +63 from the sound's own.
    Resonance,
    /// The envelope's attack time, −64 to +63 from the sound's own.
    Attack,
    /// The envelope's decay time, −64 to +63 from the sound's own.
    Decay,
    /// The envelope's release time, −64 to +63 from the sound's own.
    Release,
    /// The vibrato's rate, −64 to +63 from the sound's own.
    VibratoRate,
    /// The vibrato's depth, −64 to +63 from the sound's own.
    VibratoDepth,
    /// How long before the vibrato starts, −64 to +63 from the sound's own.
    VibratoDelay,
    /// How much goes to the reverb, 0–127.
    ReverbSend,
    /// How much goes to the chorus, 0–127.
    ChorusSend,
    /// The shared reverb on or off, for every part.
    ReverbSwitch,
    /// The shared chorus on or off, for every part.
    ChorusSwitch,
    /// The shared reverb's level, 0–127.
    ReverbLevel,
    /// The shared chorus's level, 0–127.
    ChorusLevel,
    /// The part's keys shifted by octaves, −3 to +3.
    Octave,
    /// The part's pitch in semitones, −48 to +48.
    CoarseTune,
    /// The part's pitch in cents, −50 to +50.
    FineTune,
    /// How far the pitch bend wheel bends, in semitones, 0–24.
    BendRange,
    /// How the 12 notes are tuned: equal, or one of [`TEMPERAMENTS`].
    Temperament,
    /// The key a temperament is in: 0 is C, 11 is B.
    TuningKey,
}

/// The temperaments: name, and each note's offset in cents, C first.
///
/// Offsets are from equal temperament. Just and Pythagorean come from their
/// ratios (the tests check them); Werckmeister III from its published cents.
pub const TEMPERAMENTS: [(&str, [i16; 12]); 4] = [
    ("Equal", [0; 12]),
    ("Just", [0, 12, 4, 16, -14, -2, -10, 2, 14, -16, -4, -12]),
    ("Pythagorean", [0, -10, 4, -6, 8, -2, 12, 2, -8, 6, -4, 10]),
    (
        "Werckmeister",
        [0, -10, -8, -6, -10, -2, -12, -4, -8, -12, -4, -8],
    ),
];

/// Each note's offset in cents, C first, for temperament `temperament` (an
/// index into [`TEMPERAMENTS`]) in key `key` (0 is C).
#[must_use]
pub fn scale(temperament: i16, key: i16) -> [i16; 12] {
    let (_, offsets) = usize::try_from(temperament)
        .ok()
        .and_then(|index| TEMPERAMENTS.get(index))
        .copied()
        .unwrap_or(TEMPERAMENTS[0]);
    let key = usize::try_from(key.rem_euclid(12)).unwrap_or_default();
    core::array::from_fn(|note| offsets[(note + 12 - key) % 12])
}

/// A heading in the Edit panel and the settings under it.
#[derive(Clone, Copy, Debug)]
pub struct Group {
    /// `Mix`, `Effect sends`.
    pub title: &'static str,
    /// What the settings do, where the title alone doesn't say. May be
    /// empty.
    pub note: &'static str,
    /// The settings, in order.
    pub controls: &'static [Control],
}

const fn group(title: &'static str, controls: &'static [Control]) -> Group {
    Group {
        title,
        note: "",
        controls,
    }
}

/// Whose settings: one part's (1–16), or the whole keyboard's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// This part's.
    Part(u8),
    /// Every part's.
    Keyboard,
}

/// Where a setting lives on the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// In each part's block, at this offset.
    Part(u8),
    /// In each part's zone, at this offset.
    Zone(u8),
    /// The part's 12 per-note offsets, at `+25`, from the temperament and
    /// its key together.
    Scale,
    /// At this address: the whole keyboard.
    Keyboard([u8; 4]),
}

/// How a value is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    /// One byte, as it is.
    Byte,
    /// Two bytes of four bits each, high then low (values up to 255).
    Nibbles,
    /// One byte, offset by 64: an offset from the sound's own, 0 at 64.
    Signed,
}

impl Control {
    /// The Edit panel's groups for the part being edited, in order.
    pub const GROUPS: [Group; 7] = [
        group("Mix", &[Self::Level, Self::Pan]),
        group(
            "Play",
            &[
                Self::Octave,
                Self::Mono,
                Self::Legato,
                Self::Glide,
                Self::GlideTime,
                Self::VelocitySens,
            ],
        ),
        group(
            "Tune",
            &[
                Self::Temperament,
                Self::TuningKey,
                Self::CoarseTune,
                Self::FineTune,
                Self::BendRange,
            ],
        ),
        group("Tone", &[Self::Cutoff, Self::Resonance]),
        group("Envelope", &[Self::Attack, Self::Decay, Self::Release]),
        group(
            "Vibrato",
            &[Self::VibratoRate, Self::VibratoDepth, Self::VibratoDelay],
        ),
        Group {
            title: "Effect sends",
            note: "",
            controls: &[Self::ReverbSend, Self::ChorusSend],
        },
    ];

    /// The Edit panel's groups for the whole keyboard: every part.
    pub const KEYBOARD_GROUPS: [Group; 1] = [Group {
        title: "Effects",
        note: "One reverb and one chorus, shared by every part.",
        controls: &[
            Self::ReverbSwitch,
            Self::ReverbLevel,
            Self::ChorusSwitch,
            Self::ChorusLevel,
        ],
    }];

    /// The part settings, in the Edit panel's order.
    pub fn part_controls() -> impl Iterator<Item = Self> {
        Self::GROUPS
            .into_iter()
            .flat_map(|group| group.controls.iter().copied())
    }

    /// Whose setting it is: the whole keyboard's, or part `part`'s.
    #[must_use]
    pub const fn scope(self, part: u8) -> Scope {
        match self.target() {
            Target::Keyboard(_) => Scope::Keyboard,
            Target::Part(_) | Target::Zone(_) | Target::Scale => Scope::Part(part),
        }
    }

    /// Where it lives.
    #[must_use]
    pub const fn target(self) -> Target {
        match self {
            Self::Level => Target::Part(0x07),
            Self::Pan => Target::Part(0x08),
            Self::Mono => Target::Part(0x0B),
            Self::Legato => Target::Part(0x0C),
            Self::Glide => Target::Part(0x0E),
            Self::GlideTime => Target::Part(0x0F),
            Self::Cutoff => Target::Part(0x11),
            Self::Resonance => Target::Part(0x12),
            Self::Attack => Target::Part(0x13),
            Self::Release => Target::Part(0x14),
            Self::VelocitySens => Target::Part(0x16),
            Self::ChorusSend => Target::Part(0x1D),
            Self::ReverbSend => Target::Part(0x1E),
            Self::Decay => Target::Part(0x21),
            Self::VibratoRate => Target::Part(0x22),
            Self::VibratoDepth => Target::Part(0x23),
            Self::VibratoDelay => Target::Part(0x24),
            Self::Octave => Target::Zone(0x00),
            Self::CoarseTune => Target::Part(0x09),
            Self::FineTune => Target::Part(0x0A),
            Self::BendRange => Target::Part(0x0D),
            Self::Temperament | Self::TuningKey => Target::Scale,
            // The setup.
            Self::ChorusSwitch => Target::Keyboard([0x01, 0x00, 0x00, 0x0D]),
            Self::ReverbSwitch => Target::Keyboard([0x01, 0x00, 0x00, 0x0E]),
            // The performance's chorus and reverb, as on the JUNO-DS: not in
            // GO:Plus, read and written on a GO:KEYS.
            Self::ChorusLevel => Target::Keyboard([0x10, 0x00, 0x04, 0x01]),
            Self::ReverbLevel => Target::Keyboard([0x10, 0x00, 0x06, 0x01]),
        }
    }

    /// How it's written.
    #[must_use]
    pub const fn encoding(self) -> Encoding {
        match self {
            Self::GlideTime => Encoding::Nibbles,
            Self::VelocitySens
            | Self::Cutoff
            | Self::Resonance
            | Self::Attack
            | Self::Decay
            | Self::Release
            | Self::VibratoRate
            | Self::VibratoDepth
            | Self::VibratoDelay
            | Self::Octave
            | Self::CoarseTune
            | Self::FineTune => Encoding::Signed,
            _ => Encoding::Byte,
        }
    }

    /// The lowest and highest values.
    #[must_use]
    pub const fn range(self) -> (i16, i16) {
        match self {
            Self::Level
            | Self::Pan
            | Self::ReverbSend
            | Self::ChorusSend
            | Self::ReverbLevel
            | Self::ChorusLevel => (0, 127),
            Self::ReverbSwitch | Self::ChorusSwitch => (0, 1),
            Self::Octave => (-3, 3),
            Self::CoarseTune => (-48, 48),
            Self::FineTune => (-50, 50),
            Self::BendRange => (0, 24),
            Self::Temperament => (0, 3), // TEMPERAMENTS' indices

            Self::TuningKey => (0, 11),
            Self::VelocitySens => (-63, 63),
            Self::Cutoff
            | Self::Resonance
            | Self::Attack
            | Self::Decay
            | Self::Release
            | Self::VibratoRate
            | Self::VibratoDepth
            | Self::VibratoDelay => (-64, 63),
            Self::Mono | Self::Legato | Self::Glide => (0, 2),
            Self::GlideTime => (0, 128),
        }
    }

    /// The keyboard's own value, for `scope`: what a reset goes back to.
    ///
    /// Read from a GO:KEYS just after power-on. Where they differ from
    /// GO:Plus's `init`: glide is off, and each part has its own reverb send.
    #[must_use]
    pub const fn default(self, scope: Scope) -> i16 {
        match self {
            Self::Level => 100,
            Self::Pan => 64,
            // The sound's own setting; a bend of 2 semitones.
            Self::Mono | Self::Legato | Self::BendRange => 2,
            Self::ReverbSwitch | Self::ChorusSwitch => 1,
            Self::ReverbLevel => 24,
            Self::ChorusLevel => 127,
            Self::ReverbSend => match scope {
                Scope::Part(1) => 50,
                Scope::Part(2) => 60,
                _ => 75,
            },
            // Glide off; no offset from the sound's own; no chorus.
            Self::Glide
            | Self::GlideTime
            | Self::VelocitySens
            | Self::Cutoff
            | Self::Resonance
            | Self::Attack
            | Self::Decay
            | Self::Release
            | Self::VibratoRate
            | Self::VibratoDepth
            | Self::VibratoDelay
            | Self::ChorusSend
            | Self::Octave
            | Self::CoarseTune
            | Self::FineTune
            | Self::Temperament
            | Self::TuningKey => 0,
        }
    }

    /// What the panel calls it.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            // The effects' levels sit beside their own switches.
            Self::Level | Self::ReverbLevel | Self::ChorusLevel => "Level",
            Self::Pan => "Pan",
            Self::Mono => "Voice",
            Self::Legato => "Legato",
            Self::Glide => "Glide",
            Self::GlideTime => "Glide time",
            Self::VelocitySens => "Velocity",
            Self::Cutoff => "Cutoff",
            Self::Resonance => "Resonance",
            Self::Attack => "Attack",
            Self::Decay => "Decay",
            Self::Release => "Release",
            Self::VibratoRate => "Rate",
            Self::VibratoDepth => "Depth",
            Self::VibratoDelay => "Delay",
            Self::ReverbSend | Self::ReverbSwitch => "Reverb",
            Self::ChorusSend | Self::ChorusSwitch => "Chorus",
            Self::Octave => "Octave",
            Self::CoarseTune => "Coarse",
            Self::FineTune => "Fine",
            Self::BendRange => "Bend range",
            Self::Temperament => "Temperament",
            Self::TuningKey => "Key",
        }
    }

    /// The choices, for a setting that's a choice rather than a knob, in
    /// the panel's order.
    #[must_use]
    pub fn choices(self) -> Vec<(i16, &'static str)> {
        match self {
            Self::Mono => Vec::from([(2, "Sound"), (1, "Poly"), (0, "Mono")]),
            Self::Legato | Self::Glide => Vec::from([(2, "Sound"), (0, "Off"), (1, "On")]),
            Self::ReverbSwitch | Self::ChorusSwitch => Vec::from([(0, "Off"), (1, "On")]),
            Self::Octave => Vec::from([
                (-3, "−3"),
                (-2, "−2"),
                (-1, "−1"),
                (0, "0"),
                (1, "+1"),
                (2, "+2"),
                (3, "+3"),
            ]),
            Self::Temperament => (0..).zip(TEMPERAMENTS.map(|(name, _)| name)).collect(),
            Self::TuningKey => (0..).zip(crate::go_keys::NOTE_NAMES).collect(),
            _ => Vec::new(),
        }
    }

    /// A value in words: `C`, `L20`, `Sound`, `Off`.
    #[must_use]
    pub fn text(self, value: i16) -> String {
        if let Some((_, label)) = self.choices().into_iter().find(|&(v, _)| v == value) {
            return label.into();
        }
        match (self, value) {
            (Self::Pan, 64) => "C".into(),
            (Self::Pan, v) if v < 64 => format!("L{}", 64 - v),
            (Self::Pan, v) => format!("R{}", v - 64),
            (Self::GlideTime, 128) => "Sound".into(),
            (_, v) if self.encoding() == Encoding::Signed && v > 0 => format!("+{v}"),
            (_, v) if v < 0 => format!("−{}", -v),
            (_, v) => v.to_string(),
        }
    }

    /// `value`, kept within the range.
    #[must_use]
    pub fn clamp(self, value: i16) -> i16 {
        let (low, high) = self.range();
        value.clamp(low, high)
    }

    /// The bytes that write `value`.
    #[must_use]
    pub fn data(self, value: i16) -> Vec<u8> {
        let value = self.clamp(value);
        let byte = |value: i16| u8::try_from(value).unwrap_or_default();
        match self.encoding() {
            Encoding::Byte => Vec::from([byte(value)]),
            Encoding::Nibbles => Vec::from([byte(value >> 4), byte(value & 0x0F)]),
            Encoding::Signed => Vec::from([byte(value + 64)]),
        }
    }
}

/// Settings changed from the keyboard's own: only these are sent.
pub type Edits = BTreeMap<Control, i16>;

/// One changed setting, as the settings keep it.
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    /// Which.
    pub control: Control,
    /// Its value.
    pub value: i16,
}

/// Edits as a list, for the settings.
#[must_use]
pub fn to_list(edits: &Edits) -> Vec<Edit> {
    edits
        .iter()
        .map(|(&control, &value)| Edit { control, value })
        .collect()
}

/// A list from the settings as `scope`'s edits: unknown or default values
/// dropped.
#[must_use]
pub fn from_list(list: &[Edit], scope: Scope) -> Edits {
    let keyboard = |scope| scope == Scope::Keyboard;
    list.iter()
        // A part's setting among the keyboard's, or the other way round,
        // isn't one: it couldn't be written there.
        .filter(|edit| keyboard(edit.control.scope(1)) == keyboard(scope))
        .map(|edit| (edit.control, edit.control.clamp(edit.value)))
        .filter(|&(control, value)| value != control.default(scope))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_for_values() {
        assert_eq!(Control::Pan.text(64), "C");
        assert_eq!(Control::Pan.text(44), "L20");
        assert_eq!(Control::Pan.text(84), "R20");
        assert_eq!(Control::Mono.text(0), "Mono");
        assert_eq!(Control::GlideTime.text(128), "Sound");
        assert_eq!(Control::GlideTime.text(40), "40");
        assert_eq!(Control::Cutoff.text(12), "+12");
        assert_eq!(Control::Cutoff.text(-12), "−12");
        assert_eq!(Control::Cutoff.text(0), "0");
        assert_eq!(Control::ReverbSend.text(40), "40");
    }

    #[test]
    fn encodes_and_clamps() {
        assert_eq!(Control::GlideTime.data(100), [0x06, 0x04]);
        assert_eq!(Control::GlideTime.data(128), [0x08, 0x00]);
        assert_eq!(Control::Level.data(200), [127]);
        assert_eq!(Control::ReverbSwitch.clamp(5), 1);
        assert_eq!(Control::Cutoff.data(-64), [0]);
        assert_eq!(Control::Cutoff.data(20), [84]);
        assert_eq!(Control::VelocitySens.clamp(-64), -63);
    }

    #[test]
    fn each_setting_is_in_one_group_for_its_scope() {
        let keyboard = |control: Control| control.scope(1) == Scope::Keyboard;
        let mut seen: Vec<Control> = Control::part_controls().collect();
        assert!(seen.iter().all(|&c| !keyboard(c)));
        let shared: Vec<Control> = Control::KEYBOARD_GROUPS
            .iter()
            .flat_map(|group| group.controls.iter().copied())
            .collect();
        assert!(shared.iter().all(|&c| keyboard(c)));
        seen.extend(shared);
        let count = seen.len();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), count, "once each");
    }

    #[test]
    fn temperaments_are_their_ratios_in_cents() {
        extern crate std;
        let cents = |ratio: f64| 1200.0 * ratio.log2();
        let offsets = |ratios: [f64; 12]| -> [i16; 12] {
            core::array::from_fn(|note| {
                let equal = 100.0 * f64::from(u8::try_from(note).unwrap());
                #[expect(clippy::cast_possible_truncation, reason = "a few cents")]
                let offset = (cents(ratios[note]) - equal).round() as i16;
                offset
            })
        };
        let just = [
            1.0,
            16.0 / 15.0,
            9.0 / 8.0,
            6.0 / 5.0,
            5.0 / 4.0,
            4.0 / 3.0,
            45.0 / 32.0,
            3.0 / 2.0,
            8.0 / 5.0,
            5.0 / 3.0,
            16.0 / 9.0,
            15.0 / 8.0,
        ];
        let pythagorean = [
            1.0,
            256.0 / 243.0,
            9.0 / 8.0,
            32.0 / 27.0,
            81.0 / 64.0,
            4.0 / 3.0,
            729.0 / 512.0,
            3.0 / 2.0,
            128.0 / 81.0,
            27.0 / 16.0,
            16.0 / 9.0,
            243.0 / 128.0,
        ];
        assert_eq!(scale(0, 0), [0; 12]);
        assert_eq!(scale(1, 0), offsets(just));
        assert_eq!(scale(2, 0), offsets(pythagorean));
        // Werckmeister III, from its published cents.
        assert_eq!(
            scale(3, 0),
            [0, -10, -8, -6, -10, -2, -12, -4, -8, -12, -4, -8]
        );
        // In D: D gets C's offset, E gets D's, and so on.
        assert_eq!(scale(1, 2)[2], 0);
        assert_eq!(scale(1, 2)[4], offsets(just)[2]);
        assert_eq!(scale(1, 2)[0], offsets(just)[10]);
    }

    #[test]
    fn the_settings_keep_only_changes() {
        let list = [
            Edit {
                control: Control::Level,
                value: 80,
            },
            Edit {
                control: Control::Pan,
                value: 64,
            },
        ];
        assert_eq!(
            from_list(&list, Scope::Part(1)),
            Edits::from([(Control::Level, 80)])
        );
    }
}
