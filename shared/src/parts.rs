//! The parts the keyboard plays: up to four whole sounds, each on its own
//! keys. Keys that two parts share play both (a layer); parts side by side
//! split the keyboard. Each part's keys are set on their own.
//!
//! Part `i` (from 0) is the keyboard's part `i + 1`. With seamless
//! switching it takes turns with a spare, part `i + 5`, so notes still
//! sounding keep their sound.

use alloc::vec::Vec;
use core::ops::RangeInclusive;

use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::{
    app::Model,
    go_keys::{self, Zone},
    layers::Layers,
    part::{Control, Edits, Scope},
};

/// The most parts the app plays.
pub const MOST: usize = 4;

/// The parts the keyboard has from 1 to the spares' last: those the app
/// switches off when it doesn't play them.
const USED: RangeInclusive<u8> = 1..=8;

/// One part: its sound, keys, settings and layers.
#[derive(Clone, Debug)]
pub struct Part {
    /// The sound picked, by patch id.
    pub sound: Option<usize>,
    /// Its lowest key, as a MIDI note: 0 at the keyboard's end, every key
    /// past it too.
    pub low: u8,
    /// Its highest key: 127 at the keyboard's end.
    pub high: u8,
    /// Its changed settings.
    pub edits: Edits,
    /// Its sound in the Layers tab.
    pub layers: Layers,
    /// The keyboard part it plays on now: its own, or its spare.
    pub playing: u8,
}

impl Part {
    /// Part `index`, playing `sound` on every key.
    pub fn new(index: usize, sound: Option<usize>) -> Self {
        Self {
            sound,
            low: 0,
            high: 127,
            edits: Edits::new(),
            layers: Layers::default(),
            playing: number(index),
        }
    }

    /// Its octave shift.
    pub fn octave(&self) -> i8 {
        self.edits
            .get(&Control::Octave)
            .and_then(|&shift| i8::try_from(shift).ok())
            .unwrap_or_default()
    }

    /// Its zone: on, its keys and its octave shift.
    fn zone(&self) -> Zone {
        Zone {
            on: true,
            low: self.low,
            high: self.high,
            octave: self.octave(),
        }
    }
}

/// How the keyboard starts: one sound, two side by side, or two on every
/// key. The zone map changes it from there.
#[derive(Facet, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Layout {
    /// One sound on every key.
    One,
    /// Two sounds, split at C4.
    Split,
    /// Two sounds on every key.
    Layer,
}

/// The keyboard part that part `index` is: 1 for the first.
pub fn number(index: usize) -> u8 {
    u8::try_from(index + 1).unwrap_or(1)
}

/// The spare part `index` takes turns with.
pub fn spare(index: usize) -> u8 {
    number(index) + 4
}

/// Part `index`'s settings, for defaults: its own part's, wherever it
/// plays.
pub fn scope(index: usize) -> Scope {
    Scope::Part(number(index))
}

/// Where a part's next sound goes with seamless switching: whichever of
/// its own part and its spare it isn't playing on.
pub fn idle(model: &Model, index: usize) -> u8 {
    let own = number(index);
    if model.parts[index].playing == own {
        spare(index)
    } else {
        own
    }
}

/// Where Split starts the upper part: C4.
pub const SPLIT: u8 = 60;

/// A part's keys as the keyboard has them: its ends past the keyboard's
/// brought back to them.
fn on_keyboard(part: &Part) -> (u8, u8) {
    (part.low.max(go_keys::FIRST), part.high.min(go_keys::LAST))
}

/// Whether `AddPart` can add one: there's room for another, and the part
/// being edited has two keys or more to split.
pub fn can_add(model: &Model) -> bool {
    let (low, high) = on_keyboard(model.edited());
    model.parts.len() < MOST && high > low
}

/// Starts again from one of the layouts, keeping the part being edited's
/// sound (and the others', where they're kept). Every part is written
/// again: they may have moved.
pub fn start(model: &mut Model, layout: Layout) {
    let editing = model.editing();
    let mut first = model.parts.swap_remove(editing);
    let second = model.parts.drain(..).next();
    first.low = 0;
    first.high = 127;
    model.parts.push(first);
    if layout != Layout::One {
        // Another part kept, or the same sound with its own settings.
        let mut other = second.unwrap_or_else(|| {
            let mut copy = model.parts[0].clone();
            copy.edits = Edits::new();
            copy.layers = Layers::default();
            copy
        });
        other.low = 0;
        other.high = 127;
        if layout == Layout::Split {
            // The kept part from C4 up, the other below.
            model.parts[0].low = SPLIT;
            other.high = SPLIT - 1;
        }
        model.parts.push(other);
    }
    // The next pick is the second part's: the one with no sound of its own
    // yet.
    model.editing = model.parts.len() - 1;
    renumber(model);
}

/// Splits the part being edited at its middle key: a new part, with the
/// same sound and settings, takes the upper half and is edited next. False
/// if there's no room for one.
pub fn add(model: &mut Model) -> bool {
    if !can_add(model) {
        return false;
    }
    let editing = model.editing();
    let (low, high) = on_keyboard(&model.parts[editing]);
    let middle = low + (high - low).div_ceil(2);
    let index = model.parts.len();
    let mut new = model.parts[editing].clone();
    new.low = middle;
    new.playing = number(index);
    model.parts[editing].high = middle - 1;
    model.parts.push(new);
    model.editing = index;
    true
}

/// Takes part `index` away. A neighbour it touched takes over its keys;
/// the parts after it move down one and are written again. False if it's
/// the only one.
pub fn remove(model: &mut Model, index: usize) -> bool {
    if model.parts.len() <= 1 || index >= model.parts.len() {
        return false;
    }
    let gone = model.parts.remove(index);
    let below = model
        .parts
        .iter_mut()
        .find(|part| u16::from(part.high) + 1 == u16::from(gone.low));
    if let Some(part) = below {
        part.high = gone.high;
    } else if let Some(part) = model
        .parts
        .iter_mut()
        .find(|part| u16::from(part.low) == u16::from(gone.high) + 1)
    {
        part.low = gone.low;
    }
    // The same part edited as before, now a place lower if it came after.
    let editing = model.editing;
    model.editing = if editing > index {
        editing - 1
    } else {
        editing.min(model.parts.len() - 1)
    };
    renumber(model);
    true
}

/// After parts move: each plays on its own part again.
fn renumber(model: &mut Model) {
    for (index, part) in model.parts.iter_mut().enumerate() {
        part.playing = number(index);
    }
}

/// Sets part `index`'s keys, on its own: other parts stay where they are,
/// so parts can overlap (layered there) or leave keys unplayed. At the
/// keyboard's ends they take in every key past them.
pub fn set_keys(model: &mut Model, index: usize, low: u8, high: u8) -> bool {
    let Some(part) = model.parts.get_mut(index) else {
        return false;
    };
    let (low, high) = go_keys::to_ends(low.min(127), high.min(127));
    let keys = (low.min(high), high.max(low));
    let changed = keys != (part.low, part.high);
    (part.low, part.high) = keys;
    changed
}

/// Which keys each part plays, written to the part it plays on now. With
/// seamless switching, each spare not in use is switched off.
pub fn zones(model: &Model) -> Vec<u8> {
    let mut bytes: Vec<u8> = model
        .parts
        .iter()
        .flat_map(|part| go_keys::zone(part.playing, part.zone()))
        .collect();
    if model.seamless {
        for index in 0..model.parts.len() {
            bytes.extend(go_keys::zone(idle(model, index), Zone::OFF));
        }
    }
    bytes
}

/// Every part from 1 to the spares' last that no part plays on, switched
/// off: after parts move, or when a keyboard connects.
pub fn others_off(model: &Model) -> Vec<u8> {
    USED.filter(|&number| model.parts.iter().all(|part| part.playing != number))
        .flat_map(|number| go_keys::zone(number, Zone::OFF))
        .collect()
}
