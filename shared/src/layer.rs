//! A sound's layers (Roland's tones): what the Layers editor changes.
//!
//! A part's sound on the keyboard is a JUNO-DS patch: a tone mix table that
//! switches its four tones on and off, and the four tones themselves, each
//! with a wave, a filter, envelopes and LFOs. The app reads them from the
//! keyboard ([`Sound`]) and writes changes back, byte by byte. Each setting
//! the editor offers is described here once: where it is, how it's encoded,
//! its range and its words. The layout is the JUNO-DS MIDI Implementation's
//! "Patch TMT" and "Patch Tone".

use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};

use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::go_keys;

/// How many layers a sound has.
pub const LAYERS: u8 = 4;

/// The size of the tone mix table, in bytes.
pub const MIX_SIZE: usize = 0x29;

/// The size of one tone, in bytes (`01 1A` in Roland's 7-bit addresses).
pub const TONE_SIZE: usize = 128 + 0x1A;

/// Where a byte of a sound is: the tone mix table (block 0) or a tone
/// (blocks 1–4), and its index in the block.
pub type Place = (u8, u8);

/// Bytes of a sound, by place.
pub type Bytes = BTreeMap<Place, u8>;

/// A part's sound, as read from the keyboard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sound {
    /// The tone mix table.
    pub mix: [u8; MIX_SIZE],
    /// The four tones.
    pub tones: [[u8; TONE_SIZE]; LAYERS as usize],
}

impl Default for Sound {
    fn default() -> Self {
        Self {
            mix: [0; MIX_SIZE],
            tones: [[0; TONE_SIZE]; LAYERS as usize],
        }
    }
}

impl Sound {
    /// The byte at `place`, if there's one.
    #[must_use]
    pub fn get(&self, (block, index): Place) -> Option<u8> {
        let index = usize::from(index);
        match block {
            0 => self.mix.get(index).copied(),
            tone => self.tones.get(usize::from(tone) - 1)?.get(index).copied(),
        }
    }

    /// Puts `data` into `block` from `index` on (what a read brings back).
    pub fn fill(&mut self, block: u8, index: usize, data: &[u8]) {
        let target: &mut [u8] = match block {
            0 => &mut self.mix,
            tone => {
                let Some(tone) = self.tones.get_mut(usize::from(tone) - 1) else {
                    return;
                };
                tone
            }
        };
        for (slot, &byte) in target.iter_mut().skip(index).zip(data) {
            *slot = byte;
        }
    }
}

/// One layer setting.
#[derive(
    Facet, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[repr(C)]
pub enum LayerControl {
    /// The layer plays or not.
    Switch,
    /// Its volume.
    Level,
    /// Left to right.
    Pan,
    /// Its pitch in semitones.
    Coarse,
    /// Its pitch in cents.
    Fine,
    /// The filter: off, low pass, band pass …
    FilterType,
    /// Where the filter cuts.
    Cutoff,
    /// The peak at the cutoff.
    Resonance,
    /// How far the filter envelope moves the cutoff.
    FilterDepth,
    /// The filter envelope's attack time.
    FilterAttack,
    /// Its decay time.
    FilterDecay,
    /// Its sustain level.
    FilterSustain,
    /// Its release time.
    FilterRelease,
    /// The amp envelope's attack time.
    AmpAttack,
    /// Its decay time.
    AmpDecay,
    /// Its sustain level.
    AmpSustain,
    /// Its release time.
    AmpRelease,
    /// How far the pitch envelope bends, in semitones.
    PitchDepth,
    /// The pitch envelope's attack time.
    PitchAttack,
    /// Its decay time.
    PitchDecay,
    /// LFO 1's waveform.
    Lfo1Wave,
    /// LFO 1's rate.
    Lfo1Rate,
    /// How long before LFO 1 starts.
    Lfo1Delay,
    /// LFO 1 on the pitch: vibrato.
    Lfo1Pitch,
    /// LFO 1 on the filter: wah.
    Lfo1Filter,
    /// LFO 1 on the level: tremolo.
    Lfo1Amp,
    /// LFO 1 on the pan: auto-pan.
    Lfo1Pan,
    /// LFO 2's waveform.
    Lfo2Wave,
    /// LFO 2's rate.
    Lfo2Rate,
    /// How long before LFO 2 starts.
    Lfo2Delay,
    /// LFO 2 on the pitch.
    Lfo2Pitch,
    /// LFO 2 on the filter.
    Lfo2Filter,
    /// LFO 2 on the level.
    Lfo2Amp,
    /// LFO 2 on the pan.
    Lfo2Pan,
    /// The lowest key it plays.
    KeyLow,
    /// The highest key it plays.
    KeyHigh,
    /// The softest it plays at.
    VelocityLow,
    /// The hardest it plays at.
    VelocityHigh,
    /// Keys below the lowest over which it fades in.
    KeyFadeLow,
    /// Keys above the highest over which it fades out.
    KeyFadeHigh,
    /// Velocities below the softest over which it fades in.
    VelocityFadeLow,
    /// Velocities above the hardest over which it fades out.
    VelocityFadeHigh,
}

/// How a setting is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Encoding {
    /// One byte, as it is.
    Byte,
    /// One byte, offset by 64.
    Signed,
    /// Two bytes of four bits each, high then low.
    Nibbles,
}

/// The editor's groups, in order: a heading and its settings.
pub const GROUPS: [(&str, &[LayerControl]); 9] = {
    use LayerControl as C;
    [
        ("Layer", &[C::Level, C::Pan, C::Coarse, C::Fine]),
        (
            "Keys",
            &[C::KeyLow, C::KeyHigh, C::KeyFadeLow, C::KeyFadeHigh],
        ),
        (
            "Velocity",
            &[
                C::VelocityLow,
                C::VelocityHigh,
                C::VelocityFadeLow,
                C::VelocityFadeHigh,
            ],
        ),
        (
            "Pitch envelope",
            &[C::PitchDepth, C::PitchAttack, C::PitchDecay],
        ),
        (
            "Filter",
            &[C::FilterType, C::Cutoff, C::Resonance, C::FilterDepth],
        ),
        (
            "Filter envelope",
            &[
                C::FilterAttack,
                C::FilterDecay,
                C::FilterSustain,
                C::FilterRelease,
            ],
        ),
        (
            "Amp envelope",
            &[C::AmpAttack, C::AmpDecay, C::AmpSustain, C::AmpRelease],
        ),
        (
            "LFO 1",
            &[
                C::Lfo1Wave,
                C::Lfo1Rate,
                C::Lfo1Delay,
                C::Lfo1Pitch,
                C::Lfo1Filter,
                C::Lfo1Amp,
                C::Lfo1Pan,
            ],
        ),
        (
            "LFO 2",
            &[
                C::Lfo2Wave,
                C::Lfo2Rate,
                C::Lfo2Delay,
                C::Lfo2Pitch,
                C::Lfo2Filter,
                C::Lfo2Amp,
                C::Lfo2Pan,
            ],
        ),
    ]
};

const LFO_WAVES: [&str; 13] = [
    "Sine",
    "Triangle",
    "Saw up",
    "Saw down",
    "Square",
    "Random",
    "Bend up",
    "Bend down",
    "Trapezoid",
    "Sample & hold",
    "Chaos",
    "Varied sine",
    "Step",
];

impl LayerControl {
    /// Where it is for layer `layer` (0–3), and how it's written.
    fn place(self, layer: u8) -> (Place, Encoding) {
        use Encoding::{Byte, Nibbles, Signed};
        let tone = |index: u8, encoding| ((layer + 1, index), encoding);
        match self {
            Self::Switch => ((0, 5 + 9 * layer), Byte),
            Self::KeyLow => ((0, 6 + 9 * layer), Byte),
            Self::KeyHigh => ((0, 7 + 9 * layer), Byte),
            Self::VelocityLow => ((0, 10 + 9 * layer), Byte),
            Self::VelocityHigh => ((0, 11 + 9 * layer), Byte),
            Self::KeyFadeLow => ((0, 8 + 9 * layer), Byte),
            Self::KeyFadeHigh => ((0, 9 + 9 * layer), Byte),
            Self::VelocityFadeLow => ((0, 12 + 9 * layer), Byte),
            Self::VelocityFadeHigh => ((0, 13 + 9 * layer), Byte),
            Self::Level => tone(0x00, Byte),
            Self::Coarse => tone(0x01, Signed),
            Self::Fine => tone(0x02, Signed),
            Self::Pan => tone(0x04, Byte),
            Self::PitchDepth => tone(0x3A, Signed),
            Self::PitchAttack => tone(0x3F, Byte),
            Self::PitchDecay => tone(0x40, Byte),
            Self::FilterType => tone(0x48, Byte),
            Self::Cutoff => tone(0x49, Byte),
            Self::Resonance => tone(0x4D, Byte),
            Self::FilterDepth => tone(0x4F, Signed),
            Self::FilterAttack => tone(0x55, Byte),
            Self::FilterDecay => tone(0x56, Byte),
            Self::FilterRelease => tone(0x58, Byte),
            Self::FilterSustain => tone(0x5C, Byte),
            Self::AmpAttack => tone(0x66, Byte),
            Self::AmpDecay => tone(0x67, Byte),
            Self::AmpRelease => tone(0x69, Byte),
            Self::AmpSustain => tone(0x6C, Byte),
            Self::Lfo1Wave => tone(0x6D, Byte),
            Self::Lfo1Rate => tone(0x6E, Nibbles),
            Self::Lfo1Delay => tone(0x72, Byte),
            Self::Lfo1Pitch => tone(0x77, Signed),
            Self::Lfo1Filter => tone(0x78, Signed),
            Self::Lfo1Amp => tone(0x79, Signed),
            Self::Lfo1Pan => tone(0x7A, Signed),
            Self::Lfo2Wave => tone(0x7B, Byte),
            Self::Lfo2Rate => tone(0x7C, Nibbles),
            // From 01 00: 128 on.
            Self::Lfo2Delay => tone(128, Byte),
            Self::Lfo2Pitch => tone(128 + 5, Signed),
            Self::Lfo2Filter => tone(128 + 6, Signed),
            Self::Lfo2Amp => tone(128 + 7, Signed),
            Self::Lfo2Pan => tone(128 + 8, Signed),
        }
    }

    /// The lowest and highest values.
    #[must_use]
    pub const fn range(self) -> (i16, i16) {
        match self {
            Self::Switch => (0, 1),
            Self::Coarse => (-48, 48),
            Self::Fine => (-50, 50),
            Self::PitchDepth => (-12, 12),
            Self::FilterType => (0, 6),
            Self::FilterDepth
            | Self::Lfo1Pitch
            | Self::Lfo1Filter
            | Self::Lfo1Amp
            | Self::Lfo1Pan
            | Self::Lfo2Pitch
            | Self::Lfo2Filter
            | Self::Lfo2Amp
            | Self::Lfo2Pan => (-63, 63),
            Self::Lfo1Wave | Self::Lfo2Wave => (0, 12),
            Self::Lfo1Rate | Self::Lfo2Rate => (0, 149),
            Self::VelocityLow | Self::VelocityHigh => (1, 127),
            _ => (0, 127),
        }
    }

    /// What the editor calls it.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Switch => "On",
            Self::Level => "Level",
            Self::Pan => "Pan",
            Self::Coarse => "Coarse",
            Self::Fine => "Fine",
            Self::FilterType => "Type",
            Self::Cutoff => "Cutoff",
            Self::Resonance => "Resonance",
            Self::FilterDepth => "Envelope",
            Self::FilterAttack | Self::AmpAttack | Self::PitchAttack => "Attack",
            Self::FilterDecay | Self::AmpDecay | Self::PitchDecay => "Decay",
            Self::FilterSustain | Self::AmpSustain => "Sustain",
            Self::FilterRelease | Self::AmpRelease => "Release",
            Self::PitchDepth => "Depth",
            Self::Lfo1Wave | Self::Lfo2Wave => "Wave",
            Self::Lfo1Rate | Self::Lfo2Rate => "Rate",
            Self::Lfo1Delay | Self::Lfo2Delay => "Delay",
            Self::Lfo1Pitch | Self::Lfo2Pitch => "Vibrato",
            Self::Lfo1Filter | Self::Lfo2Filter => "Wah",
            Self::Lfo1Amp | Self::Lfo2Amp => "Tremolo",
            Self::Lfo1Pan | Self::Lfo2Pan => "Auto-pan",
            Self::KeyLow => "Lowest",
            Self::KeyHigh => "Highest",
            Self::VelocityLow => "Softest",
            Self::VelocityHigh => "Hardest",
            Self::KeyFadeLow | Self::VelocityFadeLow => "Fade below",
            Self::KeyFadeHigh | Self::VelocityFadeHigh => "Fade above",
        }
    }

    /// The choices, for a setting that's a choice rather than a knob.
    #[must_use]
    pub fn choices(self) -> Vec<(i16, &'static str)> {
        let named = |names: &[&'static str]| (0..).zip(names.iter().copied()).collect();
        match self {
            Self::Switch => named(&["Off", "On"]),
            Self::FilterType => named(&[
                "Off",
                "Low pass",
                "Band pass",
                "High pass",
                "Peaking",
                "Low pass 2",
                "Low pass 3",
            ]),
            Self::Lfo1Wave | Self::Lfo2Wave => named(&LFO_WAVES),
            _ => Vec::new(),
        }
    }

    /// A value in words: `C`, `L20`, `+7`, `Low pass`, `1/8 note`.
    #[must_use]
    pub fn text(self, value: i16) -> String {
        if let Some((_, label)) = self.choices().into_iter().find(|&(v, _)| v == value) {
            return label.into();
        }
        match (self, value) {
            (Self::KeyLow | Self::KeyHigh, v) => {
                go_keys::note_name(u8::try_from(v).unwrap_or_default())
            }
            (
                Self::KeyFadeLow
                | Self::KeyFadeHigh
                | Self::VelocityFadeLow
                | Self::VelocityFadeHigh,
                0,
            ) => "Off".into(),
            (Self::Pan, 64) => "C".into(),
            (Self::Pan, v) if v < 64 => format!("L{}", 64 - v),
            (Self::Pan, v) => format!("R{}", v - 64),
            // Above 127, the rate follows the tempo.
            (Self::Lfo1Rate | Self::Lfo2Rate, v) if v > 127 => format!("Tempo {}", v - 127),
            (_, v) if self.place(0).1 == Encoding::Signed && v > 0 => format!("+{v}"),
            (_, v) if v < 0 => format!("−{}", -v),
            (_, v) => v.to_string(),
        }
    }

    /// Its value for layer `layer`, from the sound's bytes.
    #[must_use]
    pub fn value(self, layer: u8, byte: impl Fn(Place) -> u8) -> i16 {
        let ((block, index), encoding) = self.place(layer);
        let first = i16::from(byte((block, index)));
        match encoding {
            Encoding::Byte => first,
            Encoding::Signed => first - 64,
            Encoding::Nibbles => (first & 0x0F) << 4 | i16::from(byte((block, index + 1)) & 0x0F),
        }
    }

    /// The bytes that write `value` for layer `layer`, by place.
    #[must_use]
    pub fn bytes(self, layer: u8, value: i16) -> Vec<(Place, u8)> {
        let (low, high) = self.range();
        let value = value.clamp(low, high);
        let ((block, index), encoding) = self.place(layer);
        let byte = |value: i16| u8::try_from(value).unwrap_or_default();
        match encoding {
            Encoding::Byte => Vec::from([((block, index), byte(value))]),
            Encoding::Signed => Vec::from([((block, index), byte(value + 64))]),
            Encoding::Nibbles => Vec::from([
                ((block, index), byte(value >> 4)),
                ((block, index + 1), byte(value & 0x0F)),
            ]),
        }
    }
}

/// How hard a layer must be played, in words; empty for any velocity.
#[must_use]
pub fn velocity_text(low: u8, high: u8) -> String {
    if low <= 1 && high >= 127 {
        String::new()
    } else {
        format!("Velocity {low}–{high}")
    }
}

/// Whether `place` belongs to layer `layer`: its tone, or its row of the
/// mix table (switch and ranges).
#[must_use]
pub fn owns(layer: u8, (block, index): Place) -> bool {
    let row = 5 + 9 * layer;
    block == layer + 1 || (block == 0 && (row..row + 9).contains(&index))
}

/// Where a layer's wave is: the group type (internal), the group and the
/// left and right wave numbers, each four 4-bit bytes.
const WAVE_GROUP_TYPE: u8 = 0x27;
const WAVE_GROUP: u8 = 0x28;
const WAVE_LEFT: u8 = 0x2C;
const WAVE_RIGHT: u8 = 0x30;

fn nibbles4(value: u16) -> [u8; 4] {
    [12, 8, 4, 0].map(|shift| u8::try_from(value >> shift & 0x0F).unwrap_or_default())
}

/// A layer's wave: its group and its (left) number, from the sound's bytes.
#[must_use]
pub fn wave(layer: u8, byte: impl Fn(Place) -> u8) -> (u16, u16) {
    let read = |index: u8| {
        (0..4).fold(0u16, |value, i| {
            value << 4 | u16::from(byte((layer + 1, index + i)) & 0x0F)
        })
    };
    (read(WAVE_GROUP), read(WAVE_LEFT))
}

/// The bytes that give layer `layer` this wave: the group, the left number,
/// and the right one for a stereo pair (0 for none).
#[must_use]
pub fn wave_bytes(layer: u8, group: u16, left: u16, right: u16) -> Vec<(Place, u8)> {
    let tone = layer + 1;
    let at = |start: u8, value: u16| {
        nibbles4(value)
            .into_iter()
            .zip(start..)
            .map(move |(byte, index)| ((tone, index), byte))
    };
    core::iter::once(((tone, WAVE_GROUP_TYPE), 0))
        .chain(at(WAVE_GROUP, group))
        .chain(at(WAVE_LEFT, left))
        .chain(at(WAVE_RIGHT, right))
        .collect()
}

/// An envelope's shape, as points from 0 to 1000 across and up, for the
/// filter (`amp` false) or amp envelope of layer `layer`. Times take a
/// share of the width each; the sustain holds for a fixed share.
#[must_use]
pub fn envelope(layer: u8, amp: bool, byte: impl Fn(Place) -> u8) -> Vec<(u16, u16)> {
    let at = |index: u8| u32::from(byte((layer + 1, index)));
    let scale = |value: u32, to: u32| u16::try_from(value * to / 127).unwrap_or_default();
    // Times T1–T4, then levels L0–L4.
    let (times, levels) = if amp {
        (
            [0x66, 0x67, 0x68, 0x69],
            [None, Some(0x6A), Some(0x6B), Some(0x6C), None],
        )
    } else {
        (
            [0x55, 0x56, 0x57, 0x58],
            [Some(0x59), Some(0x5A), Some(0x5B), Some(0x5C), Some(0x5D)],
        )
    };
    let level = |i: usize| levels[i].map_or(0, |index| scale(at(index), 1000));
    // Four times of up to 200 each; the sustain takes the rest.
    let width = |i: usize| scale(at(times[i]), 200);
    let mut x = 0;
    let mut points = Vec::from([(0, level(0))]);
    for i in 0..3 {
        x += width(i);
        points.push((x, level(i + 1)));
    }
    points.push((1000 - width(3), level(3)));
    points.push((1000, level(4)));
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes_settings() {
        let mut tone = [0u8; TONE_SIZE];
        tone[0x49] = 40; // cutoff
        tone[0x04] = 44; // pan
        tone[0x01] = 71; // coarse +7
        tone[0x6E] = 0x08; // LFO 1 rate 0x8C = 140
        tone[0x6F] = 0x0C;
        let byte = |(block, index): Place| {
            assert_eq!(block, 2, "layer 1 is tone 2");
            tone[usize::from(index)]
        };
        assert_eq!(LayerControl::Cutoff.value(1, byte), 40);
        assert_eq!(
            LayerControl::Pan.text(LayerControl::Pan.value(1, byte)),
            "L20"
        );
        assert_eq!(LayerControl::Coarse.value(1, byte), 7);
        assert_eq!(LayerControl::Lfo1Rate.value(1, byte), 140);
        assert_eq!(LayerControl::Lfo1Rate.text(140), "Tempo 13");

        assert_eq!(LayerControl::Cutoff.bytes(0, 110), [((1, 0x49), 110)]);
        assert_eq!(LayerControl::FilterDepth.bytes(3, -10), [((4, 0x4F), 54)]);
        assert_eq!(LayerControl::Switch.bytes(2, 0), [((0, 23), 0)]);
        assert_eq!(
            LayerControl::Lfo2Rate.bytes(0, 100),
            [((1, 0x7C), 6), ((1, 0x7D), 4)]
        );
        assert_eq!(LayerControl::Lfo2Pan.bytes(0, 0), [((1, 133 + 3), 64)]);
    }

    #[test]
    fn reads_and_writes_the_wave() {
        // "Cmp'd Fng Bs", tone 1: group 1, wave 291 (0x123).
        let mut tone = [0u8; TONE_SIZE];
        tone[0x28..0x34].copy_from_slice(&[0, 0, 0, 1, 0, 1, 2, 3, 0, 0, 0, 0]);
        assert_eq!(wave(0, |(_, i)| tone[usize::from(i)]), (1, 291));
        let bytes = wave_bytes(0, 1, 1, 2);
        assert_eq!(bytes[0], ((1, 0x27), 0));
        assert_eq!(
            bytes[1..].iter().map(|&(_, b)| b).collect::<Vec<_>>(),
            [0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 2]
        );
    }

    #[test]
    fn draws_an_envelope() {
        let mut tone = [0u8; TONE_SIZE];
        // Amp: attack 0, decay 127, then down to sustain 0, release 127.
        tone[0x67] = 127;
        tone[0x69] = 127;
        tone[0x6A] = 127;
        let points = envelope(0, true, |(_, i)| tone[usize::from(i)]);
        assert_eq!(
            points,
            [(0, 0), (0, 1000), (200, 0), (200, 0), (800, 0), (1000, 0)]
        );
    }

    #[test]
    fn every_setting_is_in_one_group() {
        let mut all: Vec<LayerControl> = GROUPS
            .iter()
            .flat_map(|(_, controls)| controls.iter().copied())
            .collect();
        let count = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), count);
        assert_eq!(count, 42 - 1, "every setting but the switch");
    }
}
