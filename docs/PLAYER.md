# How the player works

OctaMEDIC plays songs the way an Amiga running OctaMED v4 does: the same timer counts decide when
each tick happens, and the sound comes from a model of the Amiga's sound chip, Paula, driven the
way the original player drives it. This page explains the hardware first, then how the engine
follows it.

## From file to sound

```
.mmd file
  └─ octamed::mmd::parser       →  OctamedMMD        (the file, as stored)
       └─ from_module()          →  OctamedicProject  (songs, patterns, instruments)
            └─ EngineHandle      →  EngineCommand     (sent to the audio thread)
                 └─ OctamedicEngine::process()  →  Paula writes  →  PaulaRenderer  →  audio
```

- `octamed` only reads and writes the file format. It knows nothing about playback.
- `octamedic_core::data` is the editable song: patterns, the sequence, instruments, the tempo.
- The engine runs inside the audio callback. The UI never touches it directly; it sends
  `EngineCommand`s through an `EngineHandle` (play, stop, a changed pattern, a new tempo) and reads
  the play position back from `EngineStatus`. Patterns are shared as `Arc`s, so editing one while
  it plays copies it instead of changing what the engine has.

## The Amiga side

### Paula and DMA

Paula has four sound channels. Each one plays 8-bit signed samples straight out of chip memory
(the RAM the custom chips can reach).

DMA, direct memory access, is what makes that cheap: once a channel's DMA is switched on, Paula
fetches the sample bytes from memory by itself, without the CPU copying anything. The CPU only
sets up a few registers per channel and then leaves the channel alone:

| Register  | What it holds                                               |
| --------- | ----------------------------------------------------------- |
| `AUDxLC`  | where in memory the sample range starts                     |
| `AUDxLEN` | how long the range is (in words, two bytes each)            |
| `AUDxPER` | the period: how many clock cycles each byte is held for     |
| `AUDxVOL` | the volume, 0 to 64                                         |
| `DMACON`  | the switch that turns each channel's DMA on or off          |

The pitch comes from the period. A channel plays `clock / period` bytes a second, where the clock
is 3,546,895 Hz on a PAL Amiga and 3,579,545 Hz on an NTSC one. A lower period is a higher note.
Paula doesn't interpolate: each byte is simply held until the next one, which is where the
Amiga's slightly gritty, aliased sound comes from.

The volume scales the output linearly, 64 being full volume.

### Looping and the latch

A sample range is a stretch of sample memory: where it starts and how long it is. (Not to be
confused with OctaMED's blocks, which are patterns.)

The location and length registers work like a latch. When DMA turns on, Paula copies them into
its own counters and starts playing that range. From then on, the CPU can write new values
without disturbing what's playing. When the range runs out, Paula picks up whatever is in the
registers at that moment and carries on from there, with no gap.

Players use this to loop samples. To play a note, a player:

1. turns the channel's DMA off,
2. writes the whole sample (up to the end of its loop) as the range,
3. writes the period and volume,
4. turns DMA on, so Paula starts the sample,
5. straight away writes the next range: the loop part of the sample, or a tiny silent range for a
   sample that doesn't loop.

When the first range ends, Paula moves on to the second, and since the registers don't change
again, it keeps replaying that one. A looped sample repeats its loop forever; a one-shot sample
falls into silence. Turning DMA off stops the channel at once.

### Timing: the CIA timer

The player doesn't count samples. It runs on an interrupt from one of the CIA chips' timers, and
each interrupt (or every few) is a tick.

A CIA timer counts down from a number, the timer count, at the E-clock rate (the CPU clock / 10:
709,379 Hz on PAL, 715,909 Hz on NTSC) and interrupts when it reaches zero. So a tick lasts
`timer count / E-clock` seconds, and the whole tempo comes down to which count the player loads.

OctaMED v4 has two ways to set it:

- **Tempo mode.** The song's tempo is a number. Tempos 1 to 10 use a fixed table of counts so
  they play at old SoundTracker speeds (the table even has an odd dip at 9). Higher tempos use
  `470000 / tempo`. One interrupt is one tick.
- **BPM mode.** The count is `1773447 / (BPM × lines per beat)` on PAL (`1789772` on NTSC), and a
  tick is every 4th interrupt.

Both are worked out with the 68000's `DIVU` instruction, which gives a 16-bit result. When the
answer doesn't fit, `DIVU` overflows and the player ends up with the dividend's low word instead,
so very slow tempos wrap around. The engine copies that on purpose.

A row lasts `ticks per line` ticks. Notes start on a row's first tick; the other ticks are where
effects like slides and arpeggios will do their work.

## The engine

The code is split the same way as this page. `octamedic_core::amiga` is the hardware: Paula, the
PAL and NTSC clocks and the 68000's `DIVU`. `octamedic_core::engine` is what OctaMED does with it:
sequencing, tempo, notes and the order of the register writes.

### Ticks in output frames

`OctamedicEngine::process` fills each audio buffer the callback asks for. It works out how many
output frames a tick takes (`sample rate × tick seconds`), renders up to the next tick, runs the
tick, and carries on. The fraction of a frame is kept between ticks, so the tempo never drifts,
however the frame counts round.

On a row's first tick, every track with a note triggers it. A track with only an instrument number
remembers it for its next note, like in the original.

### Notes

A note's pitch is its note number plus the instrument's transpose and the song's global transpose,
added as bytes, so a large transpose wraps around like it does on the Amiga. That gives an index
into a period table covering C-1 to B-3, and the instrument's finetune (-8 to 7) picks which of 16
tables to use. They're the same tables as ProTracker's.

The range is only three octaves, so notes outside it are moved:

- below C-1, they move up an octave at a time until they fit;
- far above it, they drop an octave once;
- everything left above B-3 plays in the top octave, B-3's octave.

So notes never play lower than C-1 or higher than B-3. That's for plain samples; instruments with
their own extra octaves look periods up differently.

Its volume is the instrument's volume, scaled by the track's volume and the song's master volume
(each out of 64). The listener's output volume is applied after everything else and isn't part of
the song.

### Paula writes

The engine never makes sound itself. Everything it does to a channel is a `PaulaWrite`: a period,
a volume, a sample range, DMA on or DMA off, in the order the five steps above give. Those writes
go to a `Renderer`.

Keeping that boundary means:

- the engine's output can be logged as register writes per tick and compared with what the
  original player writes, without listening to anything;
- a different renderer, such as the Octa mixer below, can sit behind the same engine.

### PaulaRenderer

`PaulaRenderer` turns the writes into audio, channel by channel, the way Paula would: DMA on starts
the latched range, the latch is picked up when a range ends, and DMA off silences the channel.

The output runs at the sound card's rate rather than Paula's, so a byte rarely lines up with an
output frame. Each frame is the average of the bytes played during it, weighted by how long each
was held. That keeps Paula's held-byte sound without adding aliasing of its own.

16-bit and stereo samples are turned into 8-bit mono when a song is loaded, since that's all Paula
can play.

## Not there yet

- Effects (the command column) don't run yet; `tick` has a place for them.
- Songs with 5 to 8 channels: OctaMED mixes two tracks into each of Paula's channels in software
  for these. That will be its own renderer.
- The Amiga's low-pass filter (the song's filter flag), and stereo: Paula plays channels 0 and 3
  on the left and 1 and 2 on the right, while the renderer puts every channel in the middle.
- Paula can't fetch bytes faster than a period of about 124 allows; the renderer doesn't limit
  this.
- MIDI and synth instruments.
