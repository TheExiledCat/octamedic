# How the player works

The player tries to sound like an Amiga running OctaMED v4. Ticks come from the same timer counts
the original uses, and the sound comes from a model of Paula, the Amiga's sound chip, written to
in the same order the original player writes to it.

The path a song takes:

```
.mmd file -> octamed::mmd::parser -> OctamedMMD (the file as stored)
          -> from_module()        -> OctamedicProject (songs, patterns, instruments)
          -> EngineHandle         -> EngineCommands to the audio thread
          -> OctamedicEngine::process() -> Paula writes -> PaulaRenderer -> audio
```

`octamed` only knows the file format. The engine lives in the audio callback and the UI never
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

The engine never makes sound itself. Everything it does to a channel is a `PaulaWrite` (period,
volume, sample range, DMA on/off) in the order of the steps above, sent to a `Renderer`. That way
you can log the writes per tick and compare them with the original player without listening, and
another renderer (like the mixer for 5 to 8 channel songs, once it exists) can go behind the same
engine.

`PaulaRenderer` is the Paula one: DMA on starts the latched range, the latch is picked up when a
range ends, DMA off silences the channel. The output runs at the sound card's rate, not Paula's,
so each output frame is the average of the bytes played during it, weighted by how long each was
held. You keep the held-byte sound without extra aliasing from the resampling.

16-bit and stereo samples become 8-bit mono at load time, since that's all Paula plays.

## Not done yet

- Effects (the command column). `tick` has a spot for them.
- 5 to 8 channel songs, where OctaMED mixes two tracks into each Paula channel in software. That
  will be its own renderer.
- The low-pass filter (the song's filter flag), and stereo. Paula puts channels 0 and 3 left and
  1 and 2 right; the renderer puts everything in the middle.
- Paula can't fetch faster than a period of about 124 allows, the renderer doesn't limit that.
- MIDI and synth instruments.
