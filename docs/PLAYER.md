# How the player works

The player tries to sound like an Amiga running OctaMED v4. Ticks come from the same timer counts
the original uses, and the sound comes from a model of Paula, the Amiga's sound chip, written to
in the same order the original player writes to it.

The path a song takes:

```
.mmd file -> mmd::mmd::parser -> OctamedMMD (the file as stored)
          -> from_module()        -> OctamedicProject (songs, patterns, instruments)
          -> EngineHandle         -> EngineCommands to the audio thread
          -> OctamedicEngine::process() -> Paula writes -> PaulaRenderer -> audio
```

`mmd` only knows the file format. The engine lives in the audio callback and the UI never
touches it: it sends `EngineCommand`s through an `EngineHandle` (play, stop, a changed pattern, a
new tempo) and reads the position back from `EngineStatus`. Patterns are shared as `Arc`s, so
editing one while it plays makes a copy.

In the code, `octamedic_core::amiga` is the hardware (Paula, the PAL/NTSC clocks, the 68000's
`DIVU`) and `octamedic_core::engine` is what OctaMED does with it.

## Paula

Paula has four channels, each playing 8-bit signed samples out of chip memory by DMA, so the CPU
doesn't copy anything. It just sets a few registers per channel: `AUDxLC` (where the sample range
starts), `AUDxLEN` (its length in words), `AUDxPER` (the period), `AUDxVOL` (0 to 64, linear), and
the channel's bit in `DMACON` to turn it on or off.

A channel plays `clock / period` bytes a second. The clock is 3,546,895 Hz on PAL and 3,579,545 Hz
on NTSC, so a lower period is a higher note. There's no interpolation, each byte is just held
until the next, and that's where the gritty aliased Amiga sound comes from.

(A "sample range" here is just a start and length in sample memory, nothing to do with OctaMED's
blocks.)

The location and length registers are latched. When DMA turns on, Paula copies them into its own
counters and plays that range. After that you can write new values without disturbing anything,
and when the range runs out Paula picks up whatever is in the registers then and keeps going with
no gap. So to play a note the player:

1. turns the channel's DMA off,
2. sets the range to the whole sample, up to the end of its loop,
3. sets period and volume,
4. turns DMA on,
5. straight away sets the next range: the loop, or a tiny silent range if the sample doesn't loop.

Paula plays the sample once, then repeats the second range forever. That's looping (or silence for
one-shots). DMA off stops the channel immediately.

## Timing

The player doesn't count samples, it runs off a CIA timer interrupt. The timer counts down from
the timer count at the E-clock rate (CPU clock / 10: 709,379 Hz PAL, 715,909 Hz NTSC), so a tick
is `timer count / E-clock` seconds. The tempo is just which count gets loaded.

In tempo mode, tempos 1 to 10 come from a fixed table so they match old SoundTracker speeds (it
has a weird dip at 9), and higher tempos use `470000 / tempo`. One interrupt is one tick.

In BPM mode the count is `1773447 / (BPM x lines per beat)` on PAL (`1789772` on NTSC) and a tick
is every 4th interrupt.

Both are done with `DIVU`, which gives a 16-bit result. If the answer doesn't fit it overflows and
the player ends up with the dividend's low word, so very slow tempos wrap around. We copy that on
purpose.

A row is `ticks per line` ticks. Notes start on the first tick; the rest are for effects like
slides and arpeggios, later.

## The engine

`OctamedicEngine::process` fills each buffer the callback asks for. It works out how many output
frames a tick is (`sample rate x tick seconds`), renders up to the next tick, runs it, and so on.
The leftover fraction of a frame carries over, so the tempo doesn't drift.

On a row's first tick each track with a note triggers it. A track with only an instrument number
remembers it for its next note, like the original.

A note's pitch is note + instrument transpose + song transpose, added as bytes, so big transposes
wrap like on the Amiga. That indexes a period table from C-1 to B-3, and the instrument's finetune
(-8 to 7) picks one of 16 tables, the same ones ProTracker has. Notes below C-1 go up an octave at
a time until they fit, notes far above drop an octave once, and anything still above B-3 plays in
B-3's octave. That's for plain samples; instruments with extra octaves look periods up
differently.

Volume is the instrument's volume scaled by track volume and song master volume (each out of 64).
The listener's output volume comes after that and isn't part of the song.

Muting a track (one of the song's 16) drops its `PaulaWrite`s before they reach the renderer, and
cuts whatever it was playing. Everything else carries on as normal, so it picks up with the right
instrument when it's unmuted. Mutes aren't saved with the song.

The engine never makes sound itself. Everything it does to a channel is a `PaulaWrite` (period,
volume, sample range, DMA on/off) in the order of the steps above, sent to a `Renderer`. That way
you can log the writes per tick and compare them with the original player without listening, and
another renderer (like the mixer for 5 to 8 channel songs, once it exists) can go behind the same
engine.

`PaulaRenderer` is the Paula one: DMA on starts the latched range, the latch is picked up when a
range ends, DMA off silences the channel. The output runs at the sound card's rate, not Paula's,
so each output frame is the average of the bytes played during it, weighted by how long each was
held. You keep the held-byte sound without extra aliasing from the resampling.

The "LED" filter (`amiga/filter.rs`) goes over the whole mix. On the real thing it's a 2-pole
Sallen-Key low-pass that the power LED bit switches; from the board's component values it comes out
at about 3.1kHz with a Q of 0.66, so close to Butterworth but not quite. It's a biquad here, made
with the bilinear transform. `EngineCommand::SetFilter` switches it, and `load_song` sets it from
the song's filter flag. It keeps running while off, so switching it doesn't click.

16-bit and stereo samples become 8-bit mono at load time, since that's all Paula plays.

Multi-octave samples (MMD instrument types 1 to 6, and IFF 8SVX files with more than one octave)
play the octave copy and period the tables in [FILES.md](FILES.md) pick for each note
(`period::sample_note`), and `OctamedicSample::octave_copy` finds that copy and its loop in the
data. ExtSample (type 7) plays two octaves under the table. The 16-bit wrap on very long copies
isn't copied.

## Hold and decay

An instrument's HLD: is how many ticks each note plays, and DEC: how fast it fades after that: the
decay comes off the note's volume (0 to 64) every tick, and a decay of 0 drops it to nothing at
once. The channel isn't switched off, it just plays on at volume 0. Instruments with a hold of 0
ignore all of this.

A hold symbol, `-|-`, is a cell with an instrument but no note (Enter in the editor). It keeps a
note going: while the *next* line has one, the hold doesn't count down. So hold 4 then two `-|-`
lines at 6 ticks a line is 6 + 6 + 4 ticks. Like the original, it doesn't look into the next block,
and once the decay has started nothing can stop it. A hold symbol also puts the track's volume
back to the instrument's, so one after a decayed note brings it back at full volume.

Only a note with an instrument number starts a new hold and resets the volume. One without
carries on with the track's hold and volume, so after a note has decayed away, a note with no
instrument number is silent. That's what MED does.

These details come from libxmp's MED player (`src/med_extras.c`), checked against the manual's
examples.

## Not done yet

- Effects (the command column). `tick` has a spot for them.
- 5 to 8 channel songs, where OctaMED mixes two tracks into each Paula channel in software. That
  will be its own renderer.
- Stereo. Paula puts channels 0 and 3 left and 1 and 2 right; the renderer puts everything in the
  middle.
- The A500's fixed RC low-pass (about 4.4kHz, always on, unlike the LED filter).
- Paula can't fetch faster than a period of about 124 allows, the renderer doesn't limit that.
- MIDI and synth instruments.
- Command 08, which sets hold and decay for one note (left digit decay, right digit hold).
- A note on a line followed by a tone portamento (03) also keeps its hold going in OctaMED, once
  commands exist.
