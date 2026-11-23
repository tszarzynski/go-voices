//! The GO:KEYS' Roland SysEx: writes (DT1) to its temporary performance,
//! which is how a split or a layer is set up.
//!
//! The keyboard runs a JUNO-DS-style performance with 16 parts. Each part
//! has a patch, and each part's zone says whether the keyboard plays it, on
//! which keys and with what octave shift. The addresses and layout come
//! from GO:Plus (<https://github.com/waldt/goplus>, `goplus.py`:
//! `PART_MODEL`, `ZONE_MODEL`, `construct_dt1_command`). The tests check
//! the bytes against that code's output.

use alloc::{format, string::String, vec::Vec};
use core::ops::RangeInclusive;

use catalogue::Patch;

use crate::{
    layer::{self, Bytes, MIX_SIZE, TONE_SIZE},
    part::{self, Control, Edits, Scope, Target},
};

/// The GO:KEYS model ID.
const MODEL_ID: [u8; 4] = [0x00, 0x00, 0x00, 0x3C];
/// Roland, device ID 17.
const HEADER: [u8; 3] = [0xF0, 0x41, 0x10];
/// Data set 1: write.
const DT1: u8 = 0x12;
/// Request data 1: read.
const RQ1: u8 = 0x11;

/// A DT1 message writing `data` from `address` on.
#[must_use]
pub fn dt1(address: [u8; 4], data: &[u8]) -> Vec<u8> {
    let checksum = checksum(address.into_iter().chain(data.iter().copied()));
    HEADER
        .into_iter()
        .chain(MODEL_ID)
        .chain([DT1])
        .chain(address)
        .chain(data.iter().copied())
        .chain([checksum, 0xF7])
        .collect()
}

/// The GO:KEYS' lowest key, C2, as a MIDI note (60 is middle C, C4).
pub const FIRST: u8 = 36;

/// Its highest key, C7.
pub const LAST: u8 = 96;

/// Its 61 keys.
pub const KEYS: RangeInclusive<u8> = FIRST..=LAST;

/// The notes' names, C first.
pub const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// A MIDI note's name: 60 is `C4`.
#[must_use]
pub fn note_name(note: u8) -> String {
    format!(
        "{}{}",
        NOTE_NAMES[usize::from(note % 12)],
        i16::from(note / 12) - 1
    )
}

/// A range of keys (a part's, a layer's) with an end at the keyboard's
/// taking in every key past it, as the sounds' own ranges do: 0 below, 127
/// above. It keeps playing with the octave shifted.
#[must_use]
pub const fn to_ends(low: u8, high: u8) -> (u8, u8) {
    (
        if low <= FIRST { 0 } else { low },
        if high >= LAST { 127 } else { high },
    )
}

/// Which keys a range plays, in words: `All keys`, `Up to B3`, `From C4`,
/// `C3–B3`. Past the keyboard's ends counts as every key that way.
#[must_use]
pub fn keys_text(low: u8, high: u8) -> String {
    match (low <= FIRST, high >= LAST) {
        (true, true) => "All keys".into(),
        (true, false) => format!("Up to {}", note_name(high)),
        (false, true) => format!("From {}", note_name(low)),
        _ if low == high => note_name(low),
        _ => format!("{}–{}", note_name(low), note_name(high)),
    }
}

/// The Roland checksum of an address and its data or size.
fn checksum(bytes: impl IntoIterator<Item = u8>) -> u8 {
    let sum = bytes.into_iter().fold(0u8, |sum, byte| (sum + byte) & 0x7F);
    (128 - sum) & 0x7F
}

/// An RQ1 message asking for `size` bytes from `address` on.
#[must_use]
pub fn rq1(address: [u8; 4], size: usize) -> Vec<u8> {
    let size = [21, 14, 7, 0].map(|shift| u8::try_from(size >> shift & 0x7F).unwrap_or_default());
    HEADER
        .into_iter()
        .chain(MODEL_ID)
        .chain([RQ1])
        .chain(address)
        .chain(size)
        .chain([checksum(address.into_iter().chain(size)), 0xF7])
        .collect()
}

/// The DT1 messages in `bytes` from a GO:KEYS, with good checksums: each
/// one's address and data.
#[must_use]
pub fn writes(bytes: &[u8]) -> Vec<([u8; 4], Vec<u8>)> {
    bytes
        .split(|&byte| byte == 0xF7)
        .filter_map(|message| {
            let start = message.iter().position(|&byte| byte == 0xF0)?;
            let message = &message[start..];
            let rest = message
                .strip_prefix(&HEADER)?
                .strip_prefix(&MODEL_ID)?
                .strip_prefix(&[DT1])?;
            let (body, sum) = rest.split_at(rest.len().checked_sub(1)?);
            (body.len() > 4 && checksum(body.iter().copied()) == sum[0]).then(|| {
                let (address, data) = body.split_at(4);
                (address.try_into().unwrap_or_default(), data.to_vec())
            })
        })
        .collect()
}

/// Where part `part`'s sound (its temporary patch) starts: `11 00 00 00`,
/// then `00 20 00 00` on for each part.
const fn sound_address(part: u8) -> [u8; 4] {
    [0x11 + (part - 1) / 4, ((part - 1) % 4) * 0x20, 0, 0]
}

/// Where a block of a sound starts: the tone mix table at `00 10 00`, tone
/// n at `00 20 00` + 2(n − 1).
const fn block_offset(block: u8) -> u8 {
    if block == 0 {
        0x10
    } else {
        0x20 + 2 * (block - 1)
    }
}

/// The address of `index` in `block` of part `part`'s sound.
fn sound_byte(part: u8, block: u8, index: usize) -> [u8; 4] {
    let [a, b, _, _] = sound_address(part);
    let high = u8::try_from(index / 128).unwrap_or_default();
    let low = u8::try_from(index % 128).unwrap_or_default();
    [a, b, block_offset(block) + high, low]
}

/// Asks for part `part`'s sound: its tone mix table and four tones.
///
/// # Panics
///
/// If `part` isn't 1–16.
#[must_use]
pub fn read_sound(part: u8) -> Vec<u8> {
    assert!((1..=16).contains(&part), "part {part}");
    (0..=layer::LAYERS)
        .flat_map(|block| {
            let size = if block == 0 { MIX_SIZE } else { TONE_SIZE };
            rq1(sound_byte(part, block, 0), size)
        })
        .collect()
}

/// For an address in part `part`'s sound: its block and index there.
#[must_use]
pub fn in_sound(part: u8, address: [u8; 4]) -> Option<(u8, usize)> {
    let [a, b, offset, low] = address;
    if [a, b] != sound_address(part)[..2] {
        return None;
    }
    (0..=layer::LAYERS).find_map(|block| {
        let index = usize::from(offset.checked_sub(block_offset(block))?) * 128 + usize::from(low);
        let size = if block == 0 { MIX_SIZE } else { TONE_SIZE };
        (index < size && offset < block_offset(block) + 2).then_some((block, index))
    })
}

/// Writes these bytes of part `part`'s sound, one DT1 for each run of
/// bytes next to each other.
///
/// # Panics
///
/// If `part` isn't 1–16.
#[must_use]
pub fn sound_bytes(part: u8, bytes: &Bytes) -> Vec<u8> {
    assert!((1..=16).contains(&part), "part {part}");
    let mut runs: Vec<((u8, u8), Vec<u8>)> = Vec::new();
    for (&(block, index), &byte) in bytes {
        match runs.last_mut() {
            Some(((b, start), data))
                if *b == block && usize::from(*start) + data.len() == usize::from(index) =>
            {
                data.push(byte);
            }
            _ => runs.push(((block, index), Vec::from([byte]))),
        }
    }
    runs.into_iter()
        .flat_map(|((block, index), data)| dt1(sound_byte(part, block, usize::from(index)), &data))
        .collect()
}

/// The start of part `part`'s block (1–16): `10 00 2p 00`.
const fn part_address(part: u8, offset: u8) -> [u8; 4] {
    [0x10, 0x00, 0x20 + part - 1, offset]
}

/// The start of zone `part`'s block (1–16): `10 00 5p 00`.
const fn zone_address(part: u8, offset: u8) -> [u8; 4] {
    [0x10, 0x00, 0x50 + part - 1, offset]
}

/// Sets part `part`'s patch: bank MSB, bank LSB and program.
///
/// # Panics
///
/// If `part` isn't 1–16.
#[must_use]
pub fn part_patch(part: u8, patch: &Patch) -> Vec<u8> {
    assert!((1..=16).contains(&part), "part {part}");
    dt1(
        part_address(part, 0x04),
        &[patch.msb, patch.lsb, patch.pc - 1],
    )
}

/// Writes a setting for `scope`: a part (1–16), or the whole keyboard.
/// The temperament and its key go together, with [`settings`].
///
/// # Panics
///
/// If the part isn't 1–16, a part's setting is written for the whole
/// keyboard, or the setting is the temperament or its key.
#[must_use]
pub fn setting(scope: Scope, control: Control, value: i16) -> Vec<u8> {
    let data = control.data(value);
    match (control.target(), scope) {
        (Target::Keyboard(address), _) => dt1(address, &data),
        (Target::Part(offset), Scope::Part(part)) => {
            assert!((1..=16).contains(&part), "part {part}");
            dt1(part_address(part, offset), &data)
        }
        (Target::Zone(offset), Scope::Part(part)) => {
            assert!((1..=16).contains(&part), "part {part}");
            dt1(zone_address(part, offset), &data)
        }
        (Target::Part(_) | Target::Zone(_) | Target::Scale, Scope::Keyboard) => {
            panic!("{control:?} is a part's setting")
        }
        (Target::Scale, Scope::Part(_)) => panic!("{control:?} goes with `settings`"),
    }
}

/// Writes every one of `edits` for `scope`. With the temperament or its
/// key among them, the part's 12 notes are tuned from both (the other at
/// its default if it isn't).
///
/// # Panics
///
/// As [`setting`], or if the temperament is written for the whole keyboard.
#[must_use]
pub fn settings(scope: Scope, edits: &Edits) -> Vec<u8> {
    let mut bytes: Vec<u8> = edits
        .iter()
        .filter(|&(control, _)| control.target() != Target::Scale)
        .flat_map(|(&control, &value)| setting(scope, control, value))
        .collect();
    if edits
        .keys()
        .any(|control| control.target() == Target::Scale)
    {
        let Scope::Part(part) = scope else {
            panic!("the temperament is a part's setting");
        };
        let value = |control: Control| {
            edits
                .get(&control)
                .copied()
                .unwrap_or_else(|| control.default(scope))
        };
        let offsets = part::scale(value(Control::Temperament), value(Control::TuningKey));
        bytes.extend(scale(part, offsets));
    }
    bytes
}

/// Tunes part `part`'s 12 notes by these offsets in cents, C first.
fn scale(part: u8, offsets: [i16; 12]) -> Vec<u8> {
    assert!((1..=16).contains(&part), "part {part}");
    let data = offsets.map(|cents| u8::try_from(cents.clamp(-64, 63) + 64).unwrap_or(64));
    dt1(part_address(part, 0x25), &data)
}

/// A zone: whether the keyboard plays the part, on which keys and with
/// what octave shift.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Zone {
    /// The keyboard plays the part.
    pub on: bool,
    /// The lowest key, as a MIDI note (60 is middle C, C4).
    pub low: u8,
    /// The highest key.
    pub high: u8,
    /// −3 to +3.
    pub octave: i8,
}

impl Zone {
    /// The whole keyboard, no shift.
    pub const FULL: Self = Self {
        on: true,
        low: 0,
        high: 127,
        octave: 0,
    };

    /// Not played.
    pub const OFF: Self = Self {
        on: false,
        ..Self::FULL
    };
}

/// Sets zone `part`: its octave shift and switch (`+00`, `+01`), then its
/// key range (`+0C`, `+0D`). Two messages, as the bytes between them are
/// reserved.
///
/// # Panics
///
/// If `part` isn't 1–16, or the zone is out of range.
#[must_use]
pub fn zone(part: u8, zone: Zone) -> Vec<u8> {
    assert!((1..=16).contains(&part), "part {part}");
    let octave = i16::from(zone.octave);
    assert_eq!(Control::Octave.clamp(octave), octave, "octave {octave}");
    assert!(zone.low <= zone.high && zone.high <= 127, "keys {zone:?}");
    // The octave (`Control::Octave`, at +00) and the switch after it.
    let mut data = Control::Octave.data(octave);
    data.push(u8::from(zone.on));
    let mut bytes = dt1(zone_address(part, 0x00), &data);
    bytes.extend(dt1(zone_address(part, 0x0C), &[zone.low, zone.high]));
    bytes
}

#[cfg(test)]
mod tests {
    use catalogue::Category;

    use super::*;

    // Fixtures from goplus.py's construct_dt1_command, model "GK".

    #[test]
    fn writes_a_parts_patch() {
        let piano = Patch {
            name: "Rich Grand 1".into(),
            category: Category::Piano,
            msb: 87,
            lsb: 64,
            pc: 9,
        };
        assert_eq!(
            part_patch(2, &piano),
            [
                0xF0, 0x41, 0x10, 0x00, 0x00, 0x00, 0x3C, 0x12, 0x10, 0x00, 0x21, 0x04, 0x57, 0x40,
                0x08, 0x2C, 0xF7
            ]
        );
    }

    #[test]
    fn writes_a_zone() {
        let lower = Zone {
            on: true,
            low: 0,
            high: 59,
            octave: -1,
        };
        let bytes = zone(1, lower);
        assert_eq!(
            bytes[..15],
            [
                0xF0, 0x41, 0x10, 0x00, 0x00, 0x00, 0x3C, 0x12, 0x10, 0x00, 0x50, 0x00, 0x3F, 0x01,
                0x60
            ],
            "octave −1, on"
        );
        assert_eq!(bytes[15], 0xF7);
        // Zone 2's range 0–59, from goplus; zone 1's differs only in the
        // address byte and so the checksum.
        assert_eq!(
            dt1(zone_address(2, 0x0C), &[0, 59]),
            [
                0xF0, 0x41, 0x10, 0x00, 0x00, 0x00, 0x3C, 0x12, 0x10, 0x00, 0x51, 0x0C, 0x00, 0x3B,
                0x58, 0xF7
            ]
        );
    }

    #[test]
    fn writes_settings() {
        let part1 = |control, value| setting(Scope::Part(1), control, value);
        assert_eq!(
            part1(Control::Level, 80),
            [
                0xF0, 0x41, 0x10, 0x00, 0x00, 0x00, 0x3C, 0x12, 0x10, 0x00, 0x20, 0x07, 0x50, 0x79,
                0xF7
            ]
        );
        assert_eq!(
            setting(Scope::Part(2), Control::Pan, 84),
            [
                0xF0, 0x41, 0x10, 0x00, 0x00, 0x00, 0x3C, 0x12, 0x10, 0x00, 0x21, 0x08, 0x54, 0x73,
                0xF7
            ]
        );
        assert_eq!(
            part1(Control::Mono, 0),
            [
                0xF0, 0x41, 0x10, 0x00, 0x00, 0x00, 0x3C, 0x12, 0x10, 0x00, 0x20, 0x0B, 0x00, 0x45,
                0xF7
            ]
        );
        assert_eq!(
            part1(Control::GlideTime, 100),
            [
                0xF0, 0x41, 0x10, 0x00, 0x00, 0x00, 0x3C, 0x12, 0x10, 0x00, 0x20, 0x0F, 0x06, 0x04,
                0x37, 0xF7
            ]
        );
        assert_eq!(
            part1(Control::GlideTime, 128),
            [
                0xF0, 0x41, 0x10, 0x00, 0x00, 0x00, 0x3C, 0x12, 0x10, 0x00, 0x20, 0x0F, 0x08, 0x00,
                0x39, 0xF7
            ]
        );
    }

    #[test]
    fn writes_offsets_and_sends() {
        let cases = [
            (1, Control::Cutoff, -20, [0x20, 0x11, 0x2C, 0x13]),
            (2, Control::Resonance, 30, [0x21, 0x12, 0x5E, 0x5F]),
            (1, Control::VelocitySens, -63, [0x20, 0x16, 0x01, 0x39]),
            (1, Control::ReverbSend, 40, [0x20, 0x1E, 0x28, 0x0A]),
            (2, Control::VibratoDelay, 63, [0x21, 0x24, 0x7F, 0x2C]),
            (1, Control::Decay, -64, [0x20, 0x21, 0x00, 0x2F]),
        ];
        for (part, control, value, [block, offset, data, checksum]) in cases {
            assert_eq!(
                setting(Scope::Part(part), control, value),
                [
                    0xF0, 0x41, 0x10, 0x00, 0x00, 0x00, 0x3C, 0x12, 0x10, 0x00, block, offset,
                    data, checksum, 0xF7
                ],
                "{control:?} {value}"
            );
        }
    }

    #[test]
    fn writes_the_whole_keyboards_settings() {
        // Addresses from GO:Plus's SETUP_MODEL and SYS_COMMON_MODEL; the
        // levels at the JUNO-DS's performance chorus and reverb, which a
        // GO:KEYS answers to.
        let cases = [
            (
                Control::ReverbSwitch,
                0,
                [0x01, 0x00, 0x00, 0x0E, 0x00, 0x71],
            ),
            (
                Control::ChorusSwitch,
                1,
                [0x01, 0x00, 0x00, 0x0D, 0x01, 0x71],
            ),
            (
                Control::ReverbLevel,
                100,
                [0x10, 0x00, 0x06, 0x01, 0x64, 0x05],
            ),
            (
                Control::ChorusLevel,
                40,
                [0x10, 0x00, 0x04, 0x01, 0x28, 0x43],
            ),
        ];
        for (control, value, [a, b, c, d, data, checksum]) in cases {
            assert_eq!(
                setting(Scope::Keyboard, control, value),
                [
                    0xF0, 0x41, 0x10, 0x00, 0x00, 0x00, 0x3C, 0x12, a, b, c, d, data, checksum,
                    0xF7
                ],
                "{control:?} {value}"
            );
        }
    }

    #[test]
    fn shifts_a_zones_octave() {
        // goplus: zone 2, octave −1 (64 − 1).
        assert_eq!(
            setting(Scope::Part(2), Control::Octave, -1),
            [
                0xF0, 0x41, 0x10, 0x00, 0x00, 0x00, 0x3C, 0x12, 0x10, 0x00, 0x51, 0x00, 0x3F, 0x60,
                0xF7
            ]
        );
    }

    #[test]
    fn writes_the_tuning() {
        // goplus: coarse +7 on part 1, fine −20 on part 2, bend range 12.
        let cases = [
            (1, Control::CoarseTune, 7, [0x20, 0x09, 0x47, 0x00]),
            (2, Control::FineTune, -20, [0x21, 0x0A, 0x2C, 0x19]),
            (1, Control::BendRange, 12, [0x20, 0x0D, 0x0C, 0x37]),
        ];
        for (part, control, value, [block, offset, data, checksum]) in cases {
            assert_eq!(
                setting(Scope::Part(part), control, value),
                [
                    0xF0, 0x41, 0x10, 0x00, 0x00, 0x00, 0x3C, 0x12, 0x10, 0x00, block, offset,
                    data, checksum, 0xF7
                ]
            );
        }
        // The 12 notes' offsets (just intonation in C), from +25.
        let edits = Edits::from([(Control::Temperament, 1)]);
        assert_eq!(
            settings(Scope::Part(1), &edits),
            [
                0xF0, 0x41, 0x10, 0x00, 0x00, 0x00, 0x3C, 0x12, 0x10, 0x00, 0x20, 0x25, 0x40, 0x4C,
                0x44, 0x50, 0x32, 0x3E, 0x36, 0x42, 0x4E, 0x30, 0x3C, 0x34, 0x35, 0xF7
            ]
        );
    }

    #[test]
    fn reads_and_writes_a_parts_sound() {
        // goplus: RQ1 for part 1's tone 1 (154 bytes) and part 3's mix table.
        let read = read_sound(1);
        assert_eq!(read.len(), 5 * 18);
        assert_eq!(
            read[18..36],
            [
                0xF0, 0x41, 0x10, 0x00, 0x00, 0x00, 0x3C, 0x11, 0x11, 0x00, 0x20, 0x00, 0x00, 0x00,
                0x01, 0x1A, 0x34, 0xF7
            ]
        );
        assert_eq!(
            read_sound(3)[..18],
            [
                0xF0, 0x41, 0x10, 0x00, 0x00, 0x00, 0x3C, 0x11, 0x11, 0x40, 0x10, 0x00, 0x00, 0x00,
                0x00, 0x29, 0x76, 0xF7
            ]
        );
        // goplus: DT1s for tone 1's cutoff on part 1, tone 3's index 133
        // (01 05) on part 2.
        assert_eq!(
            sound_bytes(1, &Bytes::from([((1, 0x49), 110)])),
            [
                0xF0, 0x41, 0x10, 0x00, 0x00, 0x00, 0x3C, 0x12, 0x11, 0x00, 0x20, 0x49, 0x6E, 0x18,
                0xF7
            ]
        );
        assert_eq!(
            sound_bytes(2, &Bytes::from([((3, 133), 64)])),
            [
                0xF0, 0x41, 0x10, 0x00, 0x00, 0x00, 0x3C, 0x12, 0x11, 0x20, 0x25, 0x05, 0x40, 0x65,
                0xF7
            ]
        );
        // Bytes next to each other go in one message.
        let two = sound_bytes(1, &Bytes::from([((1, 0x6E), 6), ((1, 0x6F), 4)]));
        assert_eq!(
            writes(&two),
            [([0x11, 0x00, 0x20, 0x6E], Vec::from([6, 4]))]
        );

        // What comes back: found in the part's sound, by block and index.
        let reply = dt1([0x11, 0x20, 0x25, 0x05], &[1, 2]);
        let mut noise = Vec::from([0x90, 60, 100]);
        noise.extend(&reply);
        let found = writes(&noise);
        assert_eq!(found, [([0x11, 0x20, 0x25, 0x05], Vec::from([1, 2]))]);
        assert_eq!(in_sound(2, found[0].0), Some((3, 133)));
        assert_eq!(in_sound(1, found[0].0), None);
        assert_eq!(in_sound(1, [0x11, 0x00, 0x10, 0x05]), Some((0, 5)));
        assert_eq!(in_sound(1, [0x11, 0x00, 0x00, 0x05]), None, "patch common");
        let mut bad = reply;
        bad[13] ^= 1;
        assert_eq!(writes(&bad), []);
    }

    #[test]
    fn switches_a_zone_on() {
        assert_eq!(
            dt1(zone_address(2, 0x01), &[1]),
            [
                0xF0, 0x41, 0x10, 0x00, 0x00, 0x00, 0x3C, 0x12, 0x10, 0x00, 0x51, 0x01, 0x01, 0x1D,
                0xF7
            ]
        );
    }
}
