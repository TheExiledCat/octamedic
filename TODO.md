# OctaMEDIC Developer TODO

> **Current milestone**: get existing MMD modules playing in the CLI with no UI and no effect commands.

Everything below is ordered by dependency — later items depend on earlier ones.

---

## Milestone 1 — CLI playback (no effects)

### 1. Voice allocator (`octamedic_core/src/engine/`)

Create a `Voice` struct and a `VoiceAllocator` that maps each pattern track to a playback voice.

```rust
struct Voice {
    sample_data: Arc<Vec<i8>>,  // raw signed 8-bit sample from the MMD file
    position: f64,              // fractional read position into sample_data
    playback_rate: f64,         // samples advanced per output sample
    volume: f32,                // 0.0–1.0
    active: bool,
    looping: bool,
    loop_start: usize,
    loop_end: usize,
}
```

`VoiceAllocator` should hold one `Voice` per track (OctaMED supports up to 16 tracks).

### 2. Sample data in the project (`octamedic_core/src/data/instrument.rs`)

`OctamedicInstrument` is currently an empty struct. Populate it with:

- The raw signed 8-bit sample bytes (copied from `OctamedMMD0SampleTable` during `from_module()`).
- Loop points (`repeat`, `repeat_length` from `OctamedMMD0Sample`).
- Native sample rate. OctaMED samples are recorded at an implicit rate derived from the Amiga PAL clock; for a note at middle C the period is 428 and the effective rate is ~8287 Hz. Use `AmigaPalPeriod` + `Frequency` utilities already in `octamed/src/utility/`.

Wire `OctamedicProject::from_module()` to copy sample data from `module.sample_table.samples` into each `OctamedicInstrument`.

### 3. Row dispatch (`octamedic_core/src/engine/transport.rs`)

The `// TODO: dispatch note-on events` comment in `transport.process()` (tick == 0 branch) needs an actual implementation. At minimum:

- Read the current row from `project → song → pattern → lines[self.row]`.
- For each track in that row, if `note != 0` and `instrument_id != 0`, send a note-on event to the voice allocator: set `Voice.sample_data`, `Voice.position = 0`, `Voice.playback_rate` (computed from note number + instrument), `Voice.active = true`.

The cleanest approach is to have `OctamedicTransport::process()` return a `Vec<NoteEvent>` (or write into a caller-supplied slice) so the engine — not the transport — owns the voices.

### 4. Pitch calculation

To compute `playback_rate` for a given note number:

```
target_frequency = frequency of the note (use octamed::utility::note + frequency)
playback_rate = target_frequency.as_hertz() / native_sample_rate
```

`native_sample_rate` for a sample played at note number N can be derived from `AmigaPalPeriod` (see `octamed/src/utility/period.rs` and `amiga.rs`).

### 5. Mixing (`octamedic_core/src/engine/engine.rs`)

Replace the `fill(128)` stub in `OctamedicEngine::process()` with a real mix:

```rust
for (out, voice) in sample_buffer[pos..pos + chunk].iter_mut().zip(voices) {
    if !voice.active { continue; }
    let s = voice.read_sample();  // linear-interpolated read at voice.position
    *out = (128.0 + s * voice.volume * 127.0).clamp(0.0, 255.0) as u8;
    voice.advance(1);             // advance position by playback_rate
}
```

Sum multiple voices with saturation clamping. Start with mono; stereo panning can come later.

### 6. Loop handling

After advancing `voice.position` past `loop_end`, wrap back to `loop_start` if `looping == true`. Use the `repeat` and `repeat_length` fields from `OctamedMMD0Sample` (repeat_length == 1 means no loop in OctaMED convention).

### 7. End-to-end test

Run `cargo run -p octamed_cli -- example_meds/example.mmd0` and issue the `play` command. You should hear the module play through to the end and the REPL should return control.

---

## Milestone 2 — Essential effect commands

Once basic playback works, add command processing in the `tick > 0` branch of `transport.process()`:

| Command | Code | Description |
|---|---|---|
| Set volume | `C xx` | Set voice volume to `xx` |
| Pattern break | `D xx` | Jump to next pattern at row `xx` |
| Position jump | `B xx` | Jump to sequence position `xx` |
| Tone portamento | `3 xx` | Slide pitch toward target note |
| Volume slide | `A xx` | Slide volume up/down |
| Vibrato | `4 xx` | Periodic pitch modulation |

`CommandId` and `command_value` are already stored on each `OctamedicPatternTrack`.

---

## Milestone 3 — Write-back

`OctamedicProject::to_module()` currently `todo!()`s. Implement it so edited projects can be saved back to MMD0/MMD1. This is needed before any editing workflow makes sense.

---

## Milestone 4 — GUI

Only start this after Milestone 1 is working. The `octamedic_gui` crate has a skeleton widget framework but no functional integration with `octamedic_core`.

---

## Known bugs (fix anytime)

- ~~`command_value` in `OctamedicPatternTrack` was copied from `command_number` instead of `command_value`~~ — **fixed**.
- `OctamedicTransport::process()` early-returned before incrementing `tick`, causing the sequencer to never advance past row 0 — **fixed**.
- `OctamedicEngine::process()` had an infinite loop because `pos` was never mutated — **fixed**.
- `OctamedTempo::get_tick_rate` is duplicated verbatim in `OctamedicTempo`. The copy in `octamedic_core/src/data/tempo.rs` should delegate to the one in `octamed` to avoid drift.
- `OctamedMMD::get_type()` panics on unknown IDs. Should return a `Result` instead.
- Several `.unwrap()` calls in `octamed/src/mmd/parser.rs` will panic on malformed files. Replace with proper error propagation.
