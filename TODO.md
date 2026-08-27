# OctaMEDIC Developer TODO

> **Current milestone**: get existing MMD modules playing in the CLI with no UI and no effect commands.

Everything below is ordered by dependency — later items depend on earlier ones. Each section describes *what* needs to exist and *why*, with enough context to design your own implementation. How you get there is up to you.

---

## Milestone 1 — CLI playback (no effects)

### 1. Voice allocator (`octamedic_core/src/engine/`)

OctaMED is a tracker — it plays samples at varying pitches across multiple simultaneous channels. A "voice" is one of those channels: it holds everything needed to know where we are in a sample, how fast to read it, and at what volume to output it.

You need some kind of per-track voice state, and a way to manage it. Think about: what information does a playing note need to carry? What happens when a new note fires on a track that's already playing? What does "silence" look like in voice state?

OctaMED supports up to 16 tracks per pattern. Start with a fixed-size allocation — dynamic allocation can come later.

```
// rough shape — implementation is your call
voice {
    which sample it's playing
    where it is in that sample (fractional, so pitch shifting works)
    how fast to advance through the sample per output sample
    current volume
    whether it is active
    loop region (if any)
}
```

### 2. Sample data in the project (`octamedic_core/src/data/instrument.rs`)

`OctamedicInstrument` is currently an empty stub. Before voices can play anything, the project needs to actually carry sample data into the engine.

Look at `OctamedMMD0SampleTable` in `octamed` — that's where the raw bytes live after parsing. You need to carry them through `OctamedicProject::from_module()` and into each `OctamedicInstrument` so the engine can reach them at runtime.

Beyond raw bytes, think about what else an instrument needs to hand a voice when it triggers: loop region, base pitch. OctaMED's loop convention is in `OctamedMMD0Sample` — look at `repeat` and `repeat_length`, and check how OctaMED signals "no loop" vs a real loop point.

The base pitch question is subtle: OctaMED samples don't carry an explicit sample rate. The rate is implied by the Amiga PAL clock and the period value at which the sample was recorded. The utilities in `octamed/src/utility/` (`period.rs`, `amiga.rs`, `frequency.rs`) have everything you need to reason about this.

### 3. Row dispatch (`octamedic_core/src/engine/transport.rs`)

Right now, `transport.process()` has a `// TODO` comment where note-on events should fire. This is where the sequencer hands work off to the audio side: on tick 0 of each row, read the row's track data and decide which voices need to start (or restart, or stop).

The interesting design question here is the boundary between transport and engine. The transport knows *what* notes are in the row; the engine owns the voices. How should information flow between them? Consider whether the transport should reach into the engine directly, or produce an intermediate event type, or something else. There is no single right answer — think about what keeps each module's responsibilities clean.

A note event at minimum needs: which track triggered it, which instrument, which note number. The engine uses that to find the sample data and compute the playback rate.

### 4. Pitch calculation

When a voice triggers on a note, you need to know how fast to advance through the sample data to produce the right pitch at the output sample rate.

The core idea: if you have a sample recorded at rate `R_sample` and you play it back at rate `R_output`, reading one sample per output frame gives you pitch at `R_sample`. To shift the pitch up by a factor of 2 (one octave), you advance two sample frames per output frame. So:

```
advance_per_output_frame = target_frequency / native_sample_rate
```

The `note` and `frequency` utilities in `octamed/src/utility/` handle the conversion from OctaMED note number to Hz. Work out the native sample rate from the Amiga PAL clock and the base period (`period.rs`, `amiga.rs`).

Linear interpolation between sample frames is enough for now — it avoids clicks when the advance is fractional.

### 5. Mixing (`octamedic_core/src/engine/engine.rs`)

`OctamedicEngine::process()` currently fills every output byte with 128 (silence). Replace that with a real mix: for each output sample, sum the contribution of every active voice.

Think about: how do you handle multiple voices contributing to one output sample? What happens when the sum clips? The output format is unsigned 8-bit PCM with 128 as silence — how does that map to the signed range your voice math will produce?

Start with mono. Stereo panning is a later concern.

### 6. Loop handling

OctaMED samples can loop. Once a voice reaches the end of its loop region, it should wrap back to the loop start and continue rather than stopping. Check the OctaMED convention for how `repeat` and `repeat_length` encode the loop region (and how to tell whether a sample loops at all).

### 7. End-to-end test

`cargo run -p octamed_cli -- example_meds/example.mmd0`, then `play`. You should hear something. Once that works, try `example.mmd1`.

---

## Milestone 2 — Essential effect commands

`OctamedicPatternTrack` already carries `command_id` and `command_value` for every note in every row — they're just not acted on yet. The tick > 0 branch of `transport.process()` is where per-tick effect updates should happen.

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

Hold off on this until Milestone 1 produces audible output. The `octamedic_gui` crate has a widget skeleton (ggez + taffy) but no connection to `octamedic_core`. That integration is the first thing to tackle when the time comes.

---

## Known bugs (fix anytime)

- ~~`command_value` in `OctamedicPatternTrack` was being copied from `command_number`~~ — **fixed**
- ~~`OctamedicTransport::process()` returned early before incrementing `tick`~~ — **fixed**
- ~~`OctamedicEngine::process()` had an infinite loop because `pos` was never mutated~~ — **fixed**
- `OctamedTempo::get_tick_rate` is duplicated verbatim in `OctamedicTempo` — the copy in `octamedic_core/src/data/tempo.rs` should delegate to avoid the two drifting out of sync
- `OctamedMMD::get_type()` panics on unknown format IDs — should return a `Result`
- Several `.unwrap()` calls in `octamed/src/mmd/parser.rs` will panic on malformed files — needs proper error propagation
