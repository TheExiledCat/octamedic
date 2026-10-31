# OctaMEDIC Developer TODO

> **Current milestones**: effect commands in the engine, and the GUI.

Everything below is ordered by dependency — later items depend on earlier ones. Each section describes *what* needs to exist and *why*, with enough context to design your own implementation. How you get there is up to you.

---

## Milestone 1 — CLI playback (no effects) — **done**

Existing MMD0 and MMD1 modules play in the CLI (`play`, and `record` to a file), without effect
commands. The engine plays them the way OctaMED v4 does on an Amiga: a model of Paula's four DMA
channels, OctaMED's period tables with finetune, and tempo from the CIA timer.
[docs/PLAYER.md](docs/PLAYER.md) explains how it works.

---

## Milestone 2 — Essential effect commands

`OctamedicPatternTrack` already carries `command_id` and `command_value` for every note in every row — they're just not acted on yet. The engine (`octamedic_core/src/engine/engine.rs`) runs each tick of a line; effects that change over a line belong in the ticks after the first.

Effects worth implementing first, roughly in order of how often they appear in real modules:

| Command | Code | What it does |
|---|---|---|
| Set volume | `C` | Immediately set the track's playback volume |
| Pattern break | `D` | End the current pattern early, jump to a specific row in the next one |
| Position jump | `B` | Jump to a different position in the sequence list |
| Tone portamento | `3` | Slide the pitch gradually toward the target note over multiple ticks |
| Volume slide | `A` | Ramp volume up or down across the ticks of a row |
| Vibrato | `4` | Oscillate pitch periodically |

The OctaMED documentation (or the original source) describes each command's parameter encoding. The community reference at [modland.com](https://www.modland.com) and MED-specific trackers documentation are useful here.

---

## Milestone 3 — Write-back

`OctamedicProject::to_module()` currently panics with `todo!()`. Implementing it would let edited projects be saved back to MMD0 or MMD1 format — a prerequisite for any real editing workflow. Look at `octamed/src/mmd/writer.rs` and `conversion.rs` to understand the existing serialization infrastructure.

---

## Milestone 4 — GUI

The GUI (`octamedic_gui`) recreates OctaMED's screen pixel for pixel: the title bar, the header with
its mode buttons and status row, the block editor showing the loaded module with cursor movement,
and the footer. `octamedic_gui/FRAMEWORK.md` explains how it's built.

Next:

- The header's panels for each mode (play, instrument, block, edit and so on).
- Entering and editing notes in the block editor.
- The other main views: notation, sample editor, synth, song list, MIDI and files.
- Wiring the status row's flags and the mute boxes.

---

## Known bugs (fix anytime)

- ~~`command_value` in `OctamedicPatternTrack` was being copied from `command_number`~~ — **fixed**
- ~~`OctamedicTransport::process()` returned early before incrementing `tick`~~ — **fixed**
- ~~`OctamedicEngine::process()` had an infinite loop because `pos` was never mutated~~ — **fixed**
- ~~`OctamedTempo::get_tick_rate` was duplicated in `OctamedicTempo`~~ — **fixed**: `OctamedTempo` only holds the stored fields, timing lives in `octamedic_core`
- ~~`OctamedMMD::get_type()` panicked on unknown format IDs~~ — **fixed**: it returns an `Option`
- Several `.unwrap()` calls in `octamed/src/mmd/parser.rs` will panic on malformed files — needs proper error propagation
