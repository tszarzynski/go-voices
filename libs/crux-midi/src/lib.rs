//! MIDI for [Crux](https://github.com/redbadger/crux) apps.
//!
//! - [`effects`]: the operations a shell performs: list MIDI devices,
//!   connect to them, scan for Bluetooth LE MIDI, send bytes.
//! - [`Connections`]: the part of an app's model that decides what to
//!   connect: every listed device, except those the user disconnected or
//!   that failed; again after a dropped link. It ignores news from
//!   connections and scans it gave up on.
//!
//! Builds without `std` (`default-features = false`).

#![cfg_attr(not(feature = "std"), no_std)]
#![expect(
    clippy::unsafe_derive_deserialize,
    reason = "Facet's derive adds `unsafe` methods to the types serde deserializes"
)]

extern crate alloc;

mod connections;
pub mod effects;

pub use connections::{Connections, LinkState, MidiEffect, MidiEvent, Notice, Update};
