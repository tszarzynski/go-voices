# GO Voices: browse and select GO:KEYS sounds over BLE MIDI

## Outcome
On iPhone, iPad or Mac, you browse, search and tap any of the ~1,380 GO:KEYS voices (JUNO-DS and GM2) and the keyboard switches to it over Bluetooth MIDI.

## Acceptance checks
- The catalogue imports every row of GO-sounds.md, and bad rows are reported — unit: catalogue import tests {#catalogue-import}
- A search for "warm pad" finds Warm Pad patches and the SPD category, prefix matches first — unit: catalogue search tests {#search}
- Selecting a voice sends CC0 MSB, CC32 LSB and PC (pc-1) on the chosen channel to each connected keyboard — unit: shared select tests {#select-bytes}
- A selection made while disconnected is sent once a keyboard connects — unit: shared queued-send test {#queued-send}
- Favourites, recents and the channel persist across launches — unit: shared storage tests {#persistence}
- chord-detector behaves as before on crux-midi: all of its checks pass — integration: chord-detector just/check-nostd/apple/android/flutter/cardputer {#cd-unchanged}
- The app builds for macOS and the iOS simulator — integration: just apple build {#apple-builds}
- On the GO:KEYS, voices from the JUNO-DS, GM2 and drum banks change the sound, from Mac and iPhone — manual {#hardware-sound}
- In Split or Layer mode the keyboard plays one sound on the lower keys and another on the upper keys (or both on every key), with a chosen split point and octave per half; Single restores one sound — manual: on the GO:KEYS; unit: SysEx bytes and app tests {#split-layer}
- The Edit panel changes the edited part's level, pan, voice, legato and glide audibly, keeps them across sound changes and reconnects, and resets them; touch curve and fixed velocity apply to the whole keyboard — manual: on the GO:KEYS; unit: settings bytes and app tests {#edit-mix-play}
- The app has a clean, flat look after the GO:KEYS (red title bar, grey panels, red dot-matrix display), places are browsed in a sidebar tree with full names (banks open to their categories, with counts), and every earlier action is still reachable — manual: screenshot review {#device-look}

## Context
The GO:KEYS (GO-61K) panel exposes only a fraction of its engine. goplus (`~/Sites/goplus`) showed that the rest is reachable over MIDI: about 1,380 JUNO-DS and GM2 patches are listed in `goplus/GO-sounds.md` (name | category | MSB | LSB | PC). Today you browse them with a CLI or a TouchOSC jog wheel. The goal is a clean app that lets you browse, search and tap a voice to send it to the keyboard over Bluetooth MIDI.

Decided with you:
- **A Crux core, with the same pattern as chord-detector.** chord-detector already handles MIDI device listing, connecting, reconnecting, attempt numbers and stale events in Rust (`shared/src/app.rs`, `shared/src/effects/midi.rs`).
- **A native SwiftUI app for iPhone, iPad and Mac.** Native shells get Crux's generated `Core` and `EffectHandler`, so no hand-written port is needed as it would be for Dart. Android can be added later on the same core: chord-detector's `MidiShell.kt` and `BleScanner.kt` are ready to adapt.

## Approach

### 1. Extract the MIDI connection logic in chord-detector into a library (publishable later)
The connection code in `shared/src/app.rs` is already separate from the chord logic, so it can move into a library rather than be copied.
- New crate `chord-detector/libs/crux-midi` (MIT OR Apache-2.0, no_std, no app dependencies). It contains:
  - the effect types moved from `shared/src/effects/midi.rs`: `WatchDevices`, `ConnectMidi`, `DisconnectMidi`, `ForgetDevice`, `ScanBluetooth`, `StopBluetoothScan`, `OpenBluetoothSettings`, `MidiDevice`, `Connection`, `ScanEvent` and `MidiError`;
  - a new **`SendMidi { device_id, bytes }`** notify operation, for output;
  - a `Connections` component that holds the `devices`, `links`, `no_auto_connect`, `attempts`, `scan`, `found` and `status` state, and the functions `set_devices`, `auto_connect`, `connect`, `close_link`, `on_link`, `start_scan`, `stop_scan` and `on_scan` from chord-detector's `shared/src/app.rs:236-400`. It has its own `Event` and `Effect` and is composed into a parent app with Crux's `Command::map_event` / `map_effect`. It reports incoming `Bytes` and the end of a link back to the parent, so chord-detector can keep calling `detector.feed` and `release`.
- `chord-detector/shared` then depends on `crux-midi`. Behaviour must not change: the existing tests and wire fixtures (`shared/tests/json_wire.rs`) pass unchanged, or with a reviewed diff that only adds `SendMidi`. Its shells don't change: `Connections` is generic over the app's `Effect`.
- Work on a branch in chord-detector. Your uncommitted `libs/eg-ttf-atlas` changes stay untouched. Follow the repo's CLAUDE.md: `just`, `just check-nostd`, `just apple build`, `just android build`, `just flutter-test`, `just cardputer build` and `just cardputer clippy`. Keep the crux pin `974d2a9`.

### 2. New repo `~/Sites/go-voices`, with the same layout as chord-detector
```
go-voices/
  Cargo.toml          workspace; crux pin 974d2a9, the same as chord-detector
  catalogue/          no_std crate: Patch {name, category, msb, lsb, pc}, Bank, Category, search
    data/patches.csv  generated once from goplus/GO-sounds.md, checked in
  tools/import-sounds converts GO-sounds.md to CSV; validates rows and reports bad ones (e.g. "PNO | 0 | 68")
  shared/             Crux app GoVoices: composes crux_midi::Connections and the catalogue
  shared_ffi/         BoltFFI 0.30.1 + codegen bin (copied from chord-detector/shared_ffi)
  apple/              XcodeGen project.yml, Justfile (typegen → boltffi pack apple → xcodegen → xcodebuild)
    GoVoices/         SwiftUI multiplatform app (iOS 18 / macOS 15)
  docs/plans/go-voices.md   spec (Blume)
```
`crux-midi` is a path dependency on `../chord-detector/libs/crux-midi` until it's published. Publishing needs your go-ahead.

### 3. Core behaviour (`shared`)
- **Catalogue**: `patches.tsv` is embedded with `include_str!` and parsed once. Categories use the goplus `PATCH_CATEGORIES` codes, with display names (PNO → Piano, SBS → Synth Bass, …). Bank groups: JUNO-DS (MSB 87), GM2 (121) and Drums (86/120).
- **Events**: `Start`, `Search(String)`, `Open(Place)` (a place in the sidebar tree: All, Favourites, Recent, a bank or a category), `Select(patch_id)`, `Next`/`Previous` (step through the list, like the TouchOSC wheel), `ToggleFavourite(id)` and `SetChannel(1..=16)`; for splits and the Edit panel `SetMode`, `EditHalf`, `SetSplitPoint`, `ToggleEditor`, `SetControl`, `ResetControl` and `ResetPart`; plus the connection events, which are forwarded.
- **Search**: case-insensitive token match on the name and the category name, so "warm pad" matches "Warm Pad 2" and pads in SPD. Prefix matches come first, then catalogue order.
- **Sending**: `Select` sends one `SendMidi` to each connected device: `Bn 00 msb`, `Bn 20 lsb`, `Cn pc-1` on the chosen channel (default 1). goplus's `--patch` sets the same values; with one sound they go as plain channel messages, and in a split or layer each part's patch is also written with SysEx (`go_keys.rs`). If nothing is connected, the selection is kept and sent when a device connects.
- **Persistence**: favourites, recents (the last 20), the channel, the sounds, the split and the edits go through `LoadSettings`/`SaveSettings` as JSON (`settings.rs`), which tolerates added and removed fields. The shell keeps the JSON in `UserDefaults`.
- **View model**: all text on screen comes from the core (chord-detector's rule): rows, counts, the connection label and errors.

### 4. Apple shell (`apple/GoVoices`)
- **`MidiShell.swift`**, adapted from `chord-detector/apple/ChordDetector/MidiShell.swift`. It compiles against each app's generated `App` module, so it is adapted rather than shared. Changes:
  - `AppKit`/`NSWorkspace` go behind `#if os(macOS)`.
  - `sendMidi`: from the device's source, find its entity (`MIDIEndpointGetEntity`), then that entity's destination (`MIDIEntityGetDestination`). Create an output port and send with `MIDISendEventList` (MIDI 1.0 UMP) or `MIDISend`.
  - `openBluetoothSettings`: on macOS it opens Audio MIDI Setup, as now. On iOS it presents Apple's `CABTMIDICentralViewController` (CoreAudioKit), the system BLE MIDI pairing screen. After pairing, the keyboard shows up as an ordinary CoreMIDI device, the core auto-connects it, and later launches reconnect without the sheet. The core starts with `bluetooth_scan: false` on both platforms.
- **UI** (SwiftUI, `NavigationSplitView` on iPad and Mac, a stack on iPhone):
  - Toolbar: a connection pill (● GO:KEYS MIDI / Connecting… / Not connected). Tapping it opens a devices sheet with the list, connect/disconnect and "Pair Bluetooth keyboard…".
  - `.searchable` field. Category chips in a horizontal scroll; bank picker (All · JUNO-DS · GM2 · Drums).
  - A `List` with sections per category. Each row shows the name, a monospaced `87·64·004` caption and a favourite star. The current voice is highlighted; tap to send.
  - A bottom bar with ◀ current voice ▶ and large targets, so you can step through the results with one hand on the keys. On Mac and iPad, ↑/↓ and Return also work.
  - Tabs or a sidebar for All / Favourites / Recent. Design tokens are carried over from `chord-detector/apple/ChordDetector/Design`. Light and dark.
- Info.plist: `NSBluetoothAlwaysUsageDescription` (needed by the iOS pairing sheet). Bundle id prefix `com.redbadger.govoices`.

### Not in v1
- Android (later, on the same core).
- Sound preview via SysEx (`SOUND_DEMO_SWITCH` in goplus).
- Parts, splits and layers.

## Decisions
- Crux core in the chord-detector pattern (the person chose it over pure Dart).
- Native SwiftUI for iPhone, iPad and Mac instead of Flutter: native shells get Crux's generated Core and EffectHandler (the person suggested native, then chose Apple only).
- MIDI connection logic is extracted from chord-detector into libs/crux-midi rather than copied (the person pointed to chord-detector for reuse).
- Voices are sent with Bank Select + Program Change, not SysEx.
- Splits and layers through Roland DT1 SysEx to the temporary performance's parts and zones, as GO:Plus does; the person chose Split + Layer, an Upper/Lower toggle, and a split point with octave per half.
- Blume tracks all work in this project (the person said "Always in this project").
- Redesign after the GO:KEYS hardware (the person: "i would like it to look more like the device itself"): keep all 39 category chips, one fixed look (no light/dark), no on-screen piano keys. The display sits at the bottom with − and +, in thumb reach.
- Simplified after the first pass (the person: "now it looks a little bit too rich. all the filtering chips should be visible without scrolling. think more like Ableton live ui. clean and simple"): flat grey controls with red only for what's selected, no grille, glows or round buttons; one-line rows.
- Tree instead of filter chips (the person: "i do not like the acronyms for filters. would be better to use full names as it is difficult to understand. i am also thinking that a tree view could be cleaner", then chose "Sidebar tree + list"): All sounds, Favourites, Recent, then each bank opening to its categories, with counts. One `Open(Place)` event replaces the tab, bank and category filters. Below 620 pt wide (iPhone) the tree and the list take turns, with a back arrow; typing a search shows the list. Search looks within the selected place.

## Why these choices
- **Native with Crux**: the shell calls Crux's generated `Core` and `EffectHandler` directly, as `apple/` in chord-detector does. Flutter would need `core.dart`, a hand port of the effect loop (its own header says "Crux has no Dart generator").
- **CoreMIDI and CABTMIDICentralViewController rather than our own BLE stack**: Apple's documented way to pair BLE MIDI on iOS. A paired device is then a normal CoreMIDI endpoint, which the existing shell already handles. No third-party BLE library is needed.
- **Bank Select and Program Change rather than SysEx DT1**: three standard messages. No model ID, and no read-modify-write of the part block as in goplus's `part_set` (`goplus.py:644`).
- **Extract `crux-midi` rather than copy it**: one tested connection state machine (attempt numbers, stale connects, rescans) for both apps and any later shell. It fits your plan to publish reusable crates.
- **Catalogue as CSV generated from GO-sounds.md**: the community data stays the source, the import can be repeated, and bad rows fail the import instead of reaching the app.

## Tasks
### Plan
- [x] Explore goplus and chord-detector; agree platform and architecture {#explore}
### Build
- [x] Branch chord-detector; create `libs/crux-midi` (effects, `Connections`, `SendMidi`) {#crux-midi}
- [x] Refactor `chord-detector/shared` onto `crux-midi`; all chord-detector checks pass {#cd-refactor}
- [x] Scaffold `go-voices` (workspace, `shared_ffi`, apple project.yml + Justfile for iOS and macOS) {#scaffold}
- [x] `tools/import-sounds` + `catalogue` crate with search {#catalogue}
- [x] `shared` GoVoices app: filters, select/next/prev, send bytes, favourites/recents, Storage effect, view model {#core}
- [x] `MidiShell.swift` (send, iOS pairing sheet, UserDefaults storage) {#apple-shell}
- [x] SwiftUI screens: browser, search, filters, bottom bar, devices sheet {#ui}
### Test
- [x] Rust unit tests: import (row count, no bad rows), search ranking, `Select` → exact bytes on the chosen channel, queued send on connect, next/prev wrap, persistence round-trip {#unit}
- [x] Builds: `just apple build` for the macOS and iOS simulator schemes (swiftlint isn't installed here, so it wasn't run) {#apple-build}
- [ ] Hardware check with the GO:KEYS on Mac and iPhone: pair, then JUNO-DS, GM2 and drum patches change the sound; confirm which channel the keyboard part listens on {#hardware}

### Redesign
- [x] Core: `CurrentView` gains `category`, `bank`, `address`; `CategoryChip` gains `code`; tests updated {#lcd-fields}
- [x] GoKeys theme tokens and the VT323 pixel font (Space Mono removed) {#theme}
- [x] Flat components: toggle cells and groups, flat buttons, LED dot, display {#panel-kit}
- [x] BrowserScreen rebuilt: title bar, filters (category code grid), one-line list, display with − and +; custom search with ⌘F {#browser}
- [x] DevicesSheet restyled; app scene (dark scheme, hidden title bar, narrower minimum width) {#sheet-scene}
- [x] Verify: `just`, `just apple build`, snapshots at Mac and iPhone width {#verify-design}
- [x] Core: `Place` and `Open(Place)` replace tabs and filters; the view model has the tree with counts and the place title; rows name their category only when ungrouped {#tree-core}
- [x] Sidebar tree beside the list (split at ≥620 pt, turn-taking below), place header with back arrow; codes removed {#tree-ui}

### Split and layer
- [x] Hardware check: done with the app itself instead of GO:Plus (the person: "just build it, i'll test with the app") {#split-check}
- [x] `shared/src/go_keys.rs`: DT1 SysEx for part patches and zones, tested against goplus.py's own output {#split-sysex}
- [x] Shell: UMP SysEx7 packing and a sized event-list buffer; round-trip through CoreMIDI verified {#split-shell}
- [x] Core: Single/Split/Layer modes, Upper/Lower editing, split point, octave per half, send on connect, settings, view model, tests {#split-core}
- [x] UI: mode menu, Upper/Lower toggle, split point and octave menus; the display shows the other half {#split-ui}
- [x] Hardware: split, layer, back to single, reconnect (the person: "all works great") {#split-hardware}

### Edit panel (stage 1 of 3)
- [x] `part.rs`: controls described once (target, encoding, range, default, words); `go_keys::setting` tested against goplus.py {#edit-model}
- [x] Core: per-half edits, keyboard touch, ToggleEditor/SetControl/ResetControl/ResetPart, re-sent after a sound change and on connect, settings {#edit-core}
- [x] UI: Edit panel (Mix: level, pan; Play: voice, legato, glide, glide time), knobs with 40 ms throttle and reset, Touch in the Keyboards sheet {#edit-ui}
- [x] Hardware: level, pan, mono, legato, glide per half; edits kept after a new sound; touch curve and fixed velocity {#edit-hardware}
### Edit panel (stage 2 of 3): Tone, Envelope, Vibrato, Effects
- [x] Cutoff, resonance, attack, decay, release, vibrato rate/depth/delay, reverb and chorus send, velocity sensitivity {#edit-tone}
- [x] Hardware: each new knob audible on a few sounds, especially cutoff and resonance {#edit-tone-hardware}
- [x] Defaults match a GO:KEYS after power-on (glide off, reverb send 50/60/75 per part); RESET verified by reading part 1 back {#edit-defaults}
- [x] Effect sends labelled per part; a Whole keyboard section: reverb/chorus on/off and level (volume, transpose and touch removed at the person's request) {#edit-keyboard}
- [x] Hardware: reverb level and on/off audible (A/B test, 2026-11-23) {#edit-keyboard-hardware}
- [x] The upper sound is kept across launches and sent on connect {#keep-upper}
- [x] Split/layer: a display per half (lower, upper), tap to edit, replacing the Lower/Upper toggle {#two-displays}
- [x] Octave is a per-half Edit panel setting (zone +00), also with one sound; reset and kept like the others {#octave-edit}
- [x] Hardware: octave with one sound and per half {#octave-hardware}
### Review fixes (2026-11-23)
- [x] Settings loaded after a keyboard connects are sent to it {#fix-load-race}
- [x] Settings kept as JSON: missing fields default, unknown settings skipped (no more lost settings when a field is added) {#fix-settings-format}
- [x] Clavi Bass and Hammer corrected to program 39 (import tool, NOTICE); no two sounds share an address {#fix-catalogue}
- [x] Sound list sections identified by position (titles repeat) {#fix-section-ids}
- [x] A keyboard that connects in Single gets the keys too (undoes a leftover split) {#fix-single-zones}
- [x] Only GO:KEYS ports connect by themselves; other MIDI gear only from the Keyboards sheet {#fix-auto-connect}
- [x] State built only when a keyboard connects; sidebar tree counted once; search query prepared once; one `commit` for save, send and redraw {#fix-tidy}
### Seamless switching (2026-11-23)
- [x] Experiment on the GO:KEYS: swapping two parts' zones while a chord is held keeps it sounding, it releases normally, the next notes play the new sound {#seamless-experiment}
- [x] Setting in the Keyboards sheet, off by default, kept: each pick goes to the half's spare part (upper 1↔3, lower 2↔4) with its sound and every setting, then the zones swap; off goes back to parts 1 and 2 {#seamless}
- [x] Hardware: seamless switching from the app works (2026-11-23); untested with a sustain pedal (none available) {#seamless-hardware}
- [x] Findings about the keyboard (addresses, power-on values, experiments and their results) written up in `docs/go-keys-midi.md` {#findings}
### More sound shaping (2026-11-23)
- [x] Experiments: the GO:KEYS has no arpeggiator, chord memory or drum patterns (the "arpeggio" switch turns on Loop Mix mode). Loop Mix control left out at the person's request {#juno-leftovers}
- [x] Tune group: coarse, fine, bend range, temperament and key (part scale tune +25) {#tune}
- [ ] Hardware: tune group audible (coarse/fine on a layer, bend range, temperaments) {#tune-hardware}
- [x] Effect types: writing the reverb type (10 00 06 00, 0–7) changes nothing audible, twice by ear; left out (fixed reverb, or the type's settings would need loading too) {#effect-types}
- [x] Own presets: a Presets place in the tree, Save (⌘S) with a suggested name, recall writes every setting (seamless on the spare parts), − / + step through them, update/rename/delete; kept in the settings {#presets}
- [x] Hardware: presets recalled on the keyboard {#presets-hardware}
- [x] The loaded preset is tracked (kept across launches): Save updates it, Save new adds one, both in the bottom bar; Edited shows when it's changed {#presets-loaded}
- [x] Investigated tone-level sound shaping (go-tone-bridge, JUNO-DS MIDI Implementation): each part's patch (`11 …`) reads and writes on the GO:KEYS, audibly (cutoff A/B, tone solos). See docs/go-keys-midi.md {#tone-editing}
- [x] Layers editor (Edit panel tab): the shell passes incoming SysEx to the core; the core reads the edited half's sound (RQ1 for the tone mix table and 4 tones), shows 4 layer cards (on/off, solo, wave, level), the wave (‹ › and a searchable picker of 1,045 names from GO Tone Bridge, stereo pairs set together), and groups: Layer, Filter, Filter/Amp envelope (shape drawn), Pitch envelope, LFO 1/2. Changes are written byte by byte, reset per setting, per layer or all; a new sound forgets them; presets and relaunches keep them {#tone-editor}
- [x] Hardware: Layers editor on the GO:KEYS (checked 2026-11-23) {#tone-editor-hardware}
- [x] Edit panel on one grid of equal cells (knobs 1, choices 1–2, envelope shapes 2, layer cards 2); groups whole, in signal order, at least as wide as their headings {#edit-grid}
- [x] Copy and paste a layer (card menu), across sounds; paste writes the whole tone and switches it on {#layer-copy}
- [ ] Hardware: a pasted layer sounds like the one copied {#layer-copy-hardware}
- [x] Layer key and velocity ranges (tone mix table): shown on each card (bar and words), edited on a drawn keyboard (click or drag the nearer end; the keyboard's ends take in every key past them) and Softest/Hardest knobs; ends can't cross; reset per layer includes them {#layer-ranges}
- [ ] Hardware: a layer limited to some keys or velocities plays only there {#layer-ranges-hardware}
- [x] Layer fades: key and velocity fade widths (Fade below/above knobs), drawn as ramps on the key strip {#layer-fades}
- [x] Up to 4 parts on a zone map: lanes with bands (tap to edit, drag an end; each part's keys move on their own), + Part splits the edited part, Remove gives its keys to a neighbour, Start from One/Split/Layer; one LCD per part (lowest keys first, 2×2 on iPhone); spares for seamless switching are parts 5–8; settings and presets from halves read as parts {#zone-map}
- [ ] Hardware: 3–4 parts, dragging a split while playing, add/remove, Layers on part 3, presets, reconnect; seamless switching on parts 5–8 {#zone-map-hardware}
- [x] Dropped: the patch multi-effect. The GO:KEYS has no MFX, so nothing to offer {#mfx-experiment}
- [ ] Parked: saving edited sounds to the keyboard's user patch slots (overwrites stored data; needs a backup and a check) {#user-patches}
### Separation of concerns (2026-11-23)
- [x] Displays described by the core (`DisplayView`), one Swift display view {#soc-displays}
- [x] The keyboard's keys and split points stated once (`go_keys::KEYS`, `split_points`), and enforced {#soc-keys}
- [x] Channel menu options from the core {#soc-channels}
- [x] Settings mapping in `settings.rs`; `Scope` (part or keyboard) instead of part 0; one `go_keys::settings` {#soc-scope}
- [x] Octave words, range and encoding from `part.rs`; part controls derived from the groups {#soc-octave}
- [x] Shared Swift pieces (`FlatMenu`, `MenuChoice`, `StarButton`, `StatusLine`, compact `FlatButtonStyle`); ids from core values, not labels or positions {#soc-swift}
- [x] Unused view fields removed; `state` and the split/layer sending merged; `Filters::searching`, `Model::picked_mut`, `Model::octave` {#soc-tidy}
- [ ] Parked: accessibility and chrome wording from the core {#soc-wording}
- [ ] Parked: crux-midi keeps typed errors (`Option<MidiError>`), the app's core words them. Do it in one shared crux-midi: bring go-voices' copy (adds `auto_connect_only`) and chord-detector's together, publish to GitHub (ask first), and have both apps depend on it {#crux-midi-shared}
### Zone map (stage 3 of 3)
- [ ] Up to 4 parts on a key-range bar, replacing Lower/Upper and the split menu {#zone-map}

### Review
- [ ] The person tries the app with the GO:KEYS {#review}

## Verification
- chord-detector on the branch: `just`, `just check-nostd`, `just apple build`, `just android build`, `just flutter-test`, `just cardputer build` and `just cardputer clippy` are all green, and the wire fixtures are unchanged apart from `SendMidi`.
- go-voices: `cargo nextest run`, `cargo clippy` (pedantic and nursery, as in chord-detector), `just apple build`.
- Manual: run on the Mac, connect "GO:KEYS MIDI", search "pad", tap results and check that the sound changes. Use ◀ ▶ to step through. Then on an iPhone: pair through the sheet, relaunch and check that it reconnects without the sheet and that favourites and recents persist.
- Blume: once the repo exists, record "Always in this project" and create the delivery with this spec at `docs/plans/go-voices.md`.
