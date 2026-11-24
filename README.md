# Go:Voices

Browse, search and pick any of the ~1,380 sounds a Roland GO:KEYS can play over MIDI (its JUNO-DS patches and the GM2 set), and send the pick to the keyboard over Bluetooth MIDI. Runs on iPhone (portrait), iPad and Mac.

![Go:Voices on the Mac: two parts across the keys (88StgGrand 3 over the upper keys, Cmp'd Fng Bs on all of them) above the keyboard, the tree of banks, the Electric piano sounds under the orange display with −, + and EDIT, and the Edit panel's Layers tab showing the amp envelope and LFO 1](docs/images/go-voices-mac.png)

The sound list comes from [GO:Plus](https://github.com/waldt/goplus)'s `GO-sounds.md` (see [Acknowledgements](#acknowledgements)). Selecting a sound sends bank select (CC 0 / CC 32) and a program change on the chosen channel (default 1).

## What it does

- **Browse and search** the sounds by bank and category, with favourites and recents. − / + step through the list.
- **Arrangement**, across the top: up to four parts, each a whole sound on its own keys, one lane each, drawn as the keyboard's pads: the selected part silver, the others wine-red. Drag either end of a lane to set its keys, or its middle to move it: side by side the parts split the keyboard, overlapping they layer. Add part, under the lanes, splits the selected part; Remove, in a lane's menu, gives its keys to a neighbour. Back to One Sound, in the preset's menu, keeps only the selected part. Tap a lane to pick sounds for that part (the display over the list says which); double-tap it to edit it. The keys of a layer, in the Layers tab, are set the same way.
- **Edit panel**, beside the list (a sheet on iPhone), for the selected part. It follows the part you select:
  - Mix: level, pan;
  - Play: octave, mono/poly, legato, glide, velocity;
  - Tune: temperament and key, coarse, fine, bend range;
  - Tone, Envelope and Vibrato offsets;
  - effect sends.
  
  Reverb and chorus for the whole keyboard (on/off and level) are in the keyboard sheet.
- **Layers:** edit the sound itself. Each of its up to four layers can be switched on or off, played alone, given another wave (step through them or search 1,045 by name), and tuned. Each layer also has a filter, filter, amp and pitch envelopes (with their shapes drawn), and two LFOs (vibrato, wah, tremolo, auto-pan). Each layer plays a range of keys and velocities, shown on its card and set by dragging on a drawn keyboard, so you can split a sound inside itself as the Combination sounds do. A layer can be copied and pasted onto another, even of another sound, from its card's menu. The app reads the sound from the keyboard first.
- **Presets:** save the arrangement and bring it back in one tap, from Presets in the tree (each with a small map of its parts). − / + step through them like a setlist.
- **Seamless switching** (optional): notes you're holding keep their sound when you pick a new one.
- **Everything is kept** between launches and sent again when the keyboard connects.
- **Text size** on the Mac: View › Text Size, Comfortable (the default) or Compact (as the iPad app looks on a Mac); ⌘− and ⌘+ switch.
- **Help and About**: in the Help and app menus on the Mac (⌘?), at the bottom of the keyboard sheet on iPhone and iPad.

What the app writes to the keyboard, and what we found by trying it, is in [docs/go-keys-midi.md](docs/go-keys-midi.md).

## Layout

| Path | What |
|---|---|
| `catalogue/` | The sounds (`data/patches.tsv`), categories, banks, search. `no_std`. |
| `shared/` | The Crux core: the tree of places, search, selection, favourites and recents, the parts and their keys, the Edit panel's settings and their SysEx, saved settings, the view model. |
| `shared_ffi/` | BoltFFI bridge and Swift type generator. |
| `apple/` | SwiftUI app for iOS 18 and macOS 15 (XcodeGen). |
| `tools/import-sounds/` | Rebuilds `patches.tsv` from `GO-sounds.md`. |

## Building

You need a Mac with:

- Xcode 16 or later
- Rust through [rustup](https://rustup.rs) (`rust-toolchain.toml` installs the targets)
- [`just`](https://github.com/casey/just), [`xcodegen`](https://github.com/yonaskolb/XcodeGen) and [`swiftlint`](https://github.com/realm/SwiftLint) (optional): `brew install just xcodegen swiftlint`
- [`cargo-nextest`](https://nexte.st): `cargo install cargo-nextest`
- the BoltFFI CLI, this exact version: `cargo install boltffi_cli@0.30.1`

`just apple doctor` checks the Apple tools.

```sh
just                 # fmt check, clippy (pedantic + nursery), tests
just check-nostd     # catalogue and core without std
just apple build     # Swift types, BoltFFI package, Xcode project, Mac + iOS simulator builds
just apple open      # generate, then open in Xcode
just import-sounds   # regenerate the catalogue (needs GO:Plus cloned next to this repo)
```

The Xcode project, the Swift packages and the generated types aren't committed: `just apple build` (or `just apple open`) makes them.

To run on an iPhone or iPad, put your Apple developer team in `apple/Local.xcconfig`. It isn't committed, and it survives regenerating the project:

```
DEVELOPMENT_TEAM = ABCDE12345
```

You'll also need a bundle identifier of your own: change `bundleIdPrefix` and `PRODUCT_BUNDLE_IDENTIFIER` in `apple/project.yml`. The Mac app and the simulator build without either.

## Pairing

- **iPhone/iPad**: tap the keyboard button at the top right ("No keyboard"), then *Pair Bluetooth keyboard…*. This opens the system's Bluetooth MIDI sheet. A keyboard paired once connects by itself afterwards.
- **Mac**: pair in Audio MIDI Setup (Window › Show MIDI Studio › Configure Bluetooth); the app connects to it.

## Acknowledgements

- **[GO:Plus](https://github.com/waldt/goplus)** by waldt (Apache License 2.0): the list of GO:KEYS sounds and their bank and program numbers in `GO-sounds.md`, which `tools/import-sounds` turns into `catalogue/data/patches.tsv` (what was changed is in [`catalogue/data/NOTICE`](catalogue/data/NOTICE)), and the research into what the GO keyboards accept over MIDI.
- **[GO Tone Bridge](https://github.com/madbadware/go-tone-bridge)** by madbadware (MIT License): the names of the waves the keyboard's sounds are built from (`catalogue/data/waves.tsv`, see its [`NOTICE`](catalogue/data/NOTICE)), and the knowledge that the GO:KEYS takes the JUNO-DS's whole patch map, which the Layers editor uses.
- **[Space Grotesk](https://github.com/floriankarsten/space-grotesk)**, © 2020 The Space Grotesk Project Authors, **[JetBrains Mono](https://github.com/JetBrains/JetBrainsMono)**, © 2020 The JetBrains Mono Project Authors, **[Quicksand](https://github.com/andrew-paglinawan/QuicksandFamily)**, © 2011 The Quicksand Project Authors, and **[VT323](https://github.com/google/fonts/tree/main/ofl/vt323)**, © 2011 The VT323 Project Authors: all under the SIL Open Font License 1.1, bundled with their licences in `apple/GoVoices/Resources/Fonts/`.
- **[Crux](https://github.com/redbadger/crux)** by Red Badger (Apache License 2.0), for the core and its shell, and **[BoltFFI](https://crates.io/crates/boltffi)** for the Swift bridge.
- **[crux-midi](https://github.com/tszarzynski/crux-midi)**, the MIDI devices and connection handling, is its own crate, shared with chord-detector, where it was extracted from that app.

The app icon, the keyboard's round button with GO:V on it, is drawn by `apple/scripts/icon.swift` (see its first lines to run it again).

Roland, GO:KEYS, GO:PIANO and JUNO-DS are trademarks of Roland Corporation. This app isn't affiliated with or endorsed by Roland.
