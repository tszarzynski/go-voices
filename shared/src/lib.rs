//! GO Voices' Crux core: browse, search and pick the sounds of a Roland
//! GO:KEYS, and send the pick to the keyboard over MIDI. Shared by every
//! shell; the Apple one is in `apple/`.
//!
//! Builds without `std` (`default-features = false`).

#![cfg_attr(not(feature = "std"), no_std)]
#![expect(
    clippy::unsafe_derive_deserialize,
    reason = "Facet's derive adds `unsafe` methods to the types serde deserializes"
)]

extern crate alloc;

mod app;
mod browse;
/// The GO:KEYS' SysEx: sounds, keys and settings.
pub mod go_keys;
/// A sound's layers, which the Layers editor changes.
pub mod layer;
mod layers;
/// The part and keyboard settings the Edit panel changes.
pub mod part;
mod parts;
/// The effects the core asks the shells to perform, besides MIDI.
pub mod settings;
mod view;

pub use app::{Effect, Event, GoVoices, Model, Place, Platform};
pub use catalogue::{Bank, Category};
pub use crux_core::Core;
/// MIDI and Bluetooth operations, from [`crux_midi`].
pub use crux_midi::effects as midi;
pub use layers::Tab;
pub use parts::Layout;
pub use view::{
    BankNode, BluetoothView, ChannelOption, ChoiceView, ConnectionView, ControlView, CurrentView,
    DeviceState, DeviceView, DisplayView, EditorView, GroupView, KeyRangeView, LayerCardView,
    LayersView, LayoutOption, LayoutView, LoadedView, NoteTick, PlaceView, PointView, PresetView,
    RowView, SectionView, Setting, SwitchView, TabView, ViewModel, WavePickerView, WaveRowView,
    WaveView, ZoneView,
};
