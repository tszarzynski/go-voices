# The GO:KEYS over MIDI: what we found

What GO Voices relies on, and what we tried that didn't work. Most addresses come from [GO:Plus](https://github.com/waldt/goplus) (`goplus.py`). Everything marked *checked* was tried on a GO:KEYS (GO-61K) over Bluetooth MIDI from a Mac in November 2026. The code that writes these is in `shared/src/go_keys.rs` and `shared/src/part.rs`, and its tests compare the bytes with GO:Plus's own functions.

## Connecting

- Over Bluetooth on the Mac, the port is called **`GO:KEYS MIDI Bluetooth`**. The app connects by itself only to ports whose name contains `GO:KEYS`; other MIDI gear is listed in the Keyboards sheet and connects only when tapped.
- The keyboard sends what you play (note on/off on each active part's channel), and answers SysEx read requests (RQ1).
- A program change on the MIDI channel (default 1) with bank select (CC 0 / CC 32) changes **part 1** only.

## Roland SysEx

- **Write (DT1):** `F0 41 10 00 00 00 3C 12 <address, 4 bytes> <data> <checksum> F7`.
- **Read (RQ1):** `F0 41 10 00 00 00 3C 11 <address> <size, 4 bytes> <checksum> F7`. The keyboard answers with a DT1.
- **Checksum:** `(128 − (sum of the address and data bytes) mod 128) & 0x7F`.
- **Model IDs:** `00 00 00 3C` is the GO:KEYS and `00 00 00 3D` the GO:PIANO. The GO keyboards also answer to `00 00 00 28`, whose map isn't known.
- **Values:** signed values are written with an offset of 64 (0 is 64). Glide time is two 4-bit bytes, high then low.

## The temporary performance (`10 00 xx xx`)

The keyboard plays a JUNO-DS-style performance of 16 parts. Each part has a patch, a zone (whether the keys play it, which keys, the octave) and settings.

### Part n: `10 00 (20 + n − 1) xx`

| Offset | Setting | Values | Power-on value (checked) |
|---|---|---|---|
| `04–06` | Patch: bank MSB, LSB, program − 1 | | |
| `07` | Level | 0–127 | 100 |
| `08` | Pan | 0–127, 64 centre | 64 |
| `09` | Coarse tune | ±48 semitones, +64 | 0 |
| `0A` | Fine tune | ±50 cents, +64 | 0 |
| `0B` | Mono/poly | 0 mono, 1 poly, 2 the sound's own | 2 |
| `0C` | Legato | 0 off, 1 on, 2 the sound's own | 2 |
| `0D` | Bend range | 0–24 semitones | 2 |
| `0E` | Glide (portamento) | 0 off, 1 on, 2 the sound's own | **0** (GO:Plus says 2) |
| `0F–10` | Glide time | 0–127, 128 the sound's own; two 4-bit bytes | **0** (GO:Plus says 128) |
| `11` | Cutoff offset | ±64, +64 | 0 |
| `12` | Resonance offset | ±64, +64 | 0 |
| `13` | Attack offset | ±64, +64 | 0 |
| `14` | Release offset | ±64, +64 | 0 |
| `16` | Velocity sensitivity offset | ±63, +64 | 0 |
| `1D` | Chorus send | 0–127 | 0 |
| `1E` | Reverb send | 0–127 | **50, 60, 75, 75** on parts 1–4 (GO:Plus says 0) |
| `21` | Decay offset | ±64, +64 | 0 |
| `22–24` | Vibrato rate, depth, delay | ±64, +64 | 0 |
| `25–30` | Scale tune, C to B | ±64 cents each, +64 | 0 |

- **Checked by ear:** every setting above that the Edit panel offers. Changing the program (a pick) doesn't reset them; the app sends them again after a new sound anyway.
- **Power-on values** were read with GO:Plus (`goplus.py --model GK part get N`) after switching the keyboard off and on. The app's RESET goes back to these, and reading part 1 back after a RESET confirmed them.

### Zone n: `10 00 (50 + n − 1) xx`

| Offset | Setting |
|---|---|
| `00` | Octave shift, ±3, +64 |
| `01` | On (the keys play the part) or off |
| `0C`, `0D` | Lowest and highest key (MIDI notes) |

- **Split and layer (checked):** two zones on, over different or the same keys.
- **Seamless switching (checked):** with part 3 set to a new sound and its zone off, switching zone 3 on and zone 1 off while a chord is held:
  - the held chord keeps its sound and releases normally;
  - the next notes play only the new sound.
  
  The app's seamless switching swaps parts 1 and 3 (upper) and 2 and 4 (lower) this way. **Not checked with a sustain pedal**, where lifting the pedal may not reach the old part.
- The keyboard went on sending notes on channel 1 after zone 1 was switched off. Its MIDI output doesn't show which parts actually sound.

### Performance chorus and reverb (JUNO-DS addresses, not in GO:Plus)

| Address | Setting | Read on the GO:KEYS |
|---|---|---|
| `10 00 04 00` | Chorus type | 3 |
| `10 00 04 01` | Chorus level | 127 |
| `10 00 06 00` | Reverb type | 3 |
| `10 00 06 01` | Reverb level | 24 |
| `10 00 06 03…` | Reverb parameters (4-bit groups) | `08 00 00 0A`… |

- **Levels: checked.** They write, read back, and are audible: an A/B test of reverb level 0 and 127 every 3 seconds.
- **Reverb type: no audible change.** Writing types 0–7 changed nothing, in two runs by ear. Either the GO:KEYS has one fixed reverb, or a type only takes effect with its parameters loaded too. Chorus type wasn't tried; it's probably the same.

## Each part's sound: the temporary patch (`11 …`, JUNO-DS layout)

The GO:KEYS takes the JUNO-DS's full patch map. [GO Tone Bridge](https://github.com/madbadware/go-tone-bridge) relies on this to let Roland's JUNO-DS Tone Manager edit GO:KEYS sounds, and GO:Plus calls this address `PATCH_ADDRESS_BASE`. The layout is in Roland's [JUNO-DS MIDI Implementation](https://static.roland.com/assets/media/pdf/JUNO-DS_MIDI_Imple_eng02_W.pdf) ("Patch Common", "Patch TMT", "Patch Tone").

- **Part n's patch:** `11 00 00 00` + (n − 1) × `00 20 00 00`. Part 1 is at `11 00 …`, part 2 at `11 20 …`, part 3 at `11 40 …`, part 4 at `11 60 …`, part 5 at `12 00 …`. A drum kit is at `+10 00 00` instead.
- **Blocks:**

  | Offset | Block | Size |
  |---|---|---|
  | `00 00 00` | Common | `50` |
  | `00 02 00` | MFX (the GO:KEYS has no multi-effect; not used) | |
  | `00 04 00` | Chorus | |
  | `00 06 00` | Reverb | |
  | `00 10 00` | Tone mix table | `29` |
  | `00 20 00`, `22`, `24`, `26` | Tones 1–4 | `01 1A` each |
- **Common:**
  - name (12 ASCII bytes), category `0C`, level `0E`, pan `0F`;
  - coarse and fine tune `11`/`12`, octave `13`;
  - mono/poly `16`, legato `17`, portamento `19…1D`;
  - offsets (cutoff, resonance, attack, release, velocity) `22…26`;
  - bend up and down `29`/`2A`;
  - matrix controls from `2B`.
- **Tone mix table:** structure and booster `00…03`, then 9 bytes per tone from `05`: on/off, then key and velocity ranges.
- **Tone:**
  - level `00`, coarse and fine `01`/`02`, pan `04`, sends `0C…10`;
  - wave group type `27`, and wave group, wave L and wave R as four 4-bit groups each at `28`, `2C`, `30`;
  - pitch envelope `3A…47`;
  - filter: type `48` (0–6), cutoff `49`, resonance `4D`, envelope depth `4F`, envelope times and levels `55…5D`;
  - amp envelope: times `66…69`, levels `6A…6C`;
  - LFO 1 from `6D` and LFO 2 from `7B`, with the rate as two 4-bit groups;
  - step LFO from `01 09`.

**Checked:**
- **Reading part 1's patch** gave the sound playing, "Cmp'd Fng Bs" (category 12, level 90). It has 4 tones on:
  - waves 291, 294, 701, 148;
  - filter type 1, cutoff 40 / 40 / 50 / 50;
  - amp envelope attack 0, decay 40, sustain level 125.
- **Writing the four tones' cutoff** (110, then back) was audible as dull and bright, in an A/B every 3 seconds.
- **Writing the tone switches** to solo each tone in turn was audible: four different layers.
- Everything was restored from the values read first.
- **Key and velocity ranges (read, November 2026):** the four Combination sounds split inside the sound, through each layer's key range in the tone mix table. None uses fades.

  | Sound | Left hand | Right hand |
  |---|---|---|
  | Lounge Kit | up to F3: Ac.Bass B, Ride Cymbal | from F#3: Stage EP p C (velocity 1–121), Stage EP f B (122–127) |
  | 80s Combo | up to B3: MG Bass 1 B, Sine, White Noise | from C4: Juno Saw HD |
  | Analog Days | up to B3: JP-8 Square, Sine, White Noise | from C4: Trance Saw A |
  | Techno Craft | up to B3: Metal Vox W1, W2, W3 | from C4: Trance Saw B |

  Lounge Kit's electric piano also switches by velocity: a harder sample above 121. The full ranges run C-1 to G9 (MIDI 0–127), past the keyboard's 61 keys.
- **The app's Layers editor** (reading the patch, then writing waves, filter, envelopes, pitch envelope, LFOs and tone switches) works on the keyboard: checked by hand.

## Setup (`01 00 00 xx`)

| Address | Setting | Result |
|---|---|---|
| `0D` | Chorus on/off | Used by the app |
| `0E` | Reverb on/off | **Checked by ear** (A/B test) |
| `12` | Transpose, −5 to +6, +64 | Works per GO:Plus; taken out of the app (the keyboard has its own) |
| `19` | "Arpeggio switch" | **Turns Loop Mix mode on** (checked). There's no arpeggiator. |
| `21` | Drum pattern switch | **Nothing audible** (checked); a JUNO-DS leftover |
| `29` | Chord memory switch | **Nothing audible** (checked): one key, one note |

## System (`02 00 xx xx`)

| Address | Setting | Notes |
|---|---|---|
| `02 00 00 05` | Master level | Scales the volume under the keyboard's own volume knob, so it can only turn it down. Taken out of the app. |
| `02 00 40 02` | Fixed velocity, 0 = play by touch | Taken out of the app |
| `02 00 40 03` | Touch curve, light/medium/heavy | Taken out of the app |

## Loop Mix (from GO:Plus, not used)

GO:Plus drives Loop Mix with NRPNs on channel 16:
- style (22);
- a pattern per part (5 parts × 11 variations);
- stop a part, or all of them;
- key (12);
- tempo, read at `01 00 01 08` and stepped up or down through model `00 00 00 28`.

Leaving Loop Mix writes `01 00 00 19` = 0. Not used by the app.

## The sound list

- **Source:** GO:Plus's `GO-sounds.md`, turned into `catalogue/data/patches.tsv` by `tools/import-sounds`.
- **Two corrections:** GO:Plus lists **Clavi Bass** (121·3) and **Hammer** (121·4) at program 40, but in GM2 they're variations of Synth Bass 1, program 39. At 40, Clavi Bass played Attack Pulse. The import tool corrects both, and a test checks that no two sounds share an address.
- **"GO Grand"** (MSB 0) is left out: no bank on the GO:KEYS has MSB 0.

## How the findings were made

These are throwaway Python scripts using `python-rtmidi` and GO:Plus's functions:
- read blocks with RQ1 and print them;
- write one value, read it back, and put the original back;
- for checks by ear, script the change while someone plays. Examples are a top-key-to-advance stepper, a 3-second A/B toggle, and a script that swaps zones one second after the first note.

Every experiment restored the values it found; switching the keyboard off and on resets the temporary performance too.
