# Files

What OctaMEDIC reads and writes besides what `mmd/format.md` (the original MMD
documentation) covers, and how OctaMED's own files and samples work.

## config.toml and crash.log

Both are in the config folder (`files::get_config_directory_root`): `%LOCALAPPDATA%\octamedic` on
Windows, `~/.config/octamedic` on Linux and `~/Library/Application Support/octamedic` on macOS.

`config.toml` keeps the settings that aren't the song's between runs (`app/config.rs`):

```toml
video = "pal"          # or "ntsc"

[output]
device = "alsa:pulse"  # cpal's id for the device, left out for the default one
mono = false

[[filters]]            # the screen filters (Ctrl+F11) in the order they run
kind = "curvature"     # scanlines, chroma, vhs_wobble, color, curvature, vignette or noise
enabled = true
amount = 0.04          # each setting by its name in the dialog, lowercase

[[filters]]
kind = "vignette"
enabled = false
strength = 0.6
size = 0.3
roundness = 0.4
```

The filters are saved whenever they're changed in the dialog. A kind it doesn't know is left out
and a setting that's missing keeps its default.

Anything missing takes its default, and a missing file is written with the defaults. A file that
can't be read or parsed (a video standard other than pal or ntsc, say) is renamed to
`config.toml.bak` so it isn't lost, the defaults are used, and the startup errors say why.

`crash.log` is written when the GUI panics or can't open its window (`crash_log.rs`): the error, a
backtrace for a panic, and the startup errors before it. Only the last crash is kept.

## Loading instruments

LOAD INSTR (`octamedic_gui/src/app/files/load_instrument.rs`) tries the formats in its `LOADERS`
list in order, and the first `InstrumentLoader` (`octamedic_core/src/import/mod.rs`) that recognizes
the file takes it. Loaders go by the file's header, never its extension: plenty of files, especially
old Amiga ones, don't have one.

| Loader | Recognizes | Asks first |
|---|---|---|
| `WavLoader` | `RIFF` at 0 and `WAVE` at 8 | yes |
| `FlacLoader` | `fLaC` at 0 | yes |
| `Mp3Loader` | `ID3` at 0, or an MPEG Layer III frame header | yes |
| `IffLoader` | `FORM` at 0 and `8SVX` at 8, an IFF sample | no |
| `MshLoader` | `MSH\0` at 0, a synth or hybrid sound | no |
| `RawLoader` | anything | no |

`RawLoader` reads any file as signed 8-bit samples, the Amiga's own format. That's what OctaMED
does with a file it doesn't recognize, so it goes last. The MP3 frame header check looks at every
field (sync bits, version, layer, bitrate and sample rate index) so raw data that happens to start
with the sync bits still loads as raw.

A loader only decodes the file into a `DecodedSample` (`octamedic_core/src/import/sample.rs`):
its rate, bit depth if it has one, one list per channel from -1.0 to 1.0, and loop points if the
format stores them. WAV, FLAC and MP3 decode through `symphonia` (`import/decode.rs`).
`DecodedSample::to_instrument` then makes the 8-bit mono sample Paula plays, the same way for every
format, with `ImportSettings`:

- the channel: left, right, or all of them averaged;
- downsampling to a rate, never up. Each new sample is the average of the old ones it covers,
  weighted by how much of each it covers, unless `allow_aliasing` is set, in which case it's the
  old sample at the new one's start, which keeps the aliasing;
- the 8-bit conversion scales by 128 and clamps, so 8-bit data comes out byte for byte as it went
  in;
- loop points scale with the rate.

`IffLoader` and `MshLoader` skip all that: an 8SVX sample is already what Paula plays, so it
comes back as a finished instrument (`Decoded::Ready`) with its octave copies, loop (`VHDR`'s
one-shot and repeat lengths, of the shortest copy) and volume (`VHDR`'s, out of 0x10000). A
`BODY` packed with Fibonacci-delta (compression 1) is unpacked: after a pad byte and the first
sample, each 4-bit nibble picks a step from -34, -21, -13, -8, -5, -3, -2, -1, 0, 1, 2, 3, 5, 8,
13, 21.

If the loader asks first, the import dialog shows the file's rate, bit depth and channels and asks
about downsampling, to which note (C-1, C-2 or C-3) and the channel. Downsampling to a note means
converting to the rate that note plays at (`paula_clock / period`, 16574 Hz for C-3 on PAL), so
the sample plays at its original pitch on that note. Paula can't play faster than B-3's rate
(about 31 kHz, and its DMA tops out around 28.6 kHz), so a 44.1 kHz sample can never play at its
original speed; downsampling is how you get it to.

Preview plays the sample on the engine's preview channel the way it would come out with the dialog's
settings, at the chosen note, and turns into Stop. Changing a setting while it plays starts it
again with the new one, and closing the dialog stops it.

## Multi-octave samples

MMD instrument types 1 to 6 (`IFF5OCT`, `IFF3OCT`, `IFF2OCT`, `IFF4OCT`, `IFF6OCT`, `IFF7OCT`) are
IFF 8SVX style samples holding one copy of the sound per octave, each lower octave's copy twice as
long as the one above. The player picks a copy for each note and plays it at a period from the
normal table, so every copy stays in what Paula can play and the instrument covers more octaves.

In OctaMED v4:

- IFF 8SVX sounds may have 1, 3 or 5 octaves. The sample editor also takes 2, 4, 6 and 7 octave
  samples, "but the seventh octave will not play".
- Single octave IFF sounds and raw samples can only use octaves 1 to 3.
- A note an instrument doesn't have plays in the nearest octave it does have.

### How the player plays them

**Layout.** With N octaves the shortest copy (the highest octave) is `length / (2^N - 1)` bytes,
dividing by 31, 7, 3, 15, 63 and 127 for types 1 to 6 and dropping any remainder. Copy k (0 is the
shortest) is `base << k` long and starts `base * (2^k - 1)` into the data, so the copies go
shortest first, each twice as long as the one before. The loop's repeat and repeat length (in
words) are shifted left by k for the copy that's playing.

**Notes.** As for any sample, the note (after instrument and song transpose) minus one is an index
from C-1; below 0 it goes up 12 at a time until it fits, above 62 (D-6) it drops 12 once. The
period table has rows for octaves 1 to 3 and then three copies of octave 3. With
`oct = index / 12`, `semi = index % 12` and `i = oct + [0, 6, 12, 18, 24, 30][type - 1]`, the
player plays copy `shift[i]` at period `table[start[i] + semi]`:

```
shift: 4,3,2,1,1,0,  2,2,1,1,0,0,  1,1,0,0,0,0,  3,3,2,2,1,0,  5,4,3,2,1,0,  6,5,4,3,2,1
start: 12,12,12,12,24,24,  0,12,12,24,24,36,  0,12,12,24,36,36,
       0,12,12,24,24,24,  12,12,12,12,12,12,  12,12,12,12,12,12
```

That's, by the note's octave (copy k at the period table's octave; o3\* is the copy of octave 3, so
no higher than octave 5's pitch):

| Type | Oct 1 | Oct 2 | Oct 3 | Oct 4 | Oct 5 | Oct 6 (C to D) |
|---|---|---|---|---|---|---|
| IFF5OCT (1) | k4 o2 | k3 o2 | k2 o2 | k1 o2 | k1 o3 | k0 o3 |
| IFF3OCT (2) | k2 o1 | k2 o2 | k1 o2 | k1 o3 | k0 o3 | k0 o3\* |
| IFF2OCT (3) | k1 o1 | k1 o2 | k0 o2 | k0 o3 | k0 o3\* | k0 o3\* |
| IFF4OCT (4) | k3 o1 | k3 o2 | k2 o2 | k2 o3 | k1 o3 | k0 o3 |
| IFF6OCT (5) | k5 o2 | k4 o2 | k3 o2 | k2 o2 | k1 o2 | k0 o2 |
| IFF7OCT (6) | k6 o2 | k5 o2 | k4 o2 | k3 o2 | k2 o2 | k1 o2 |

IFF7OCT never plays its shortest copy, so its seventh octave never plays.

**EXTSAMPLE** (type 7, V5 and later) plays as one copy, two octaves lower than a normal sample:
the period table gets two extra-low rows (3424 down to 1812, then 1712 down to 906) in front.

**Quirks** of the player:

- the copy's length and offset are worked out in 16 bits, so a copy or base longer than 65535
  bytes wraps;
- arpeggio, tone portamento's target and FFD look periods up from the plain note, without the
  copy and octave mapping above, so on multi-octave samples they come out in the wrong octave
  where the two differ.

**v4.** OctaMED v4's own player uses the full tables above for all six types. The player libraries
that came with it only know types 1 and 2 (the first 12 entries of each table) and play every other
non-zero type as IFF5OCT. OctaMEDIC goes by the full tables.

The engine plays them (see [PLAYER.md](PLAYER.md)). Imported samples with a high enough rate could
be made into 5 octave samples instead of being downsampled to one rate: each octave's copy is the
sound downsampled to half the rate of the one below it, so the source has to be at least as fast as
the longest copy.

## Synth sounds

`format.md` describes synth and hybrid instruments as they're stored inside a module: a `SynthInstr`
with 128-byte volume and waveform tables and 64 waveform pointers. A synth sound saved on its own is
stored more compactly. Here's one with a single 128-byte waveform, 166 bytes in all:

| Offset | Bytes | Field |
|---|---|---|
| 0x00 | `4D 53 48 00` | `"MSH\0"`, where the module struct has its length |
| 0x04 | `FF FF` | type, -1 synth (-2 hybrid) |
| 0x06 | `00`, `00 00 00` | default decay, 3 reserved bytes |
| 0x0A | `00 00`, `00 00` | repeat, repeat length (hybrids only) |
| 0x0E | `00 04` | volume table length |
| 0x10 | `00 06` | waveform table length |
| 0x12 | `01`, `03` | volume speed, waveform speed |
| 0x14 | `00 01` | waveform count |
| 0x16 | `40 F2 03 FF` | volume table: volume $40, `CHD` 3, `END` |
| 0x1A | `F4 06 F5 50 00 FF` | waveform table: `VBD` 6, `VBS` $50, waveform 0, `END` |
| 0x20 | `00 00 00 24` | waveform 0's offset, from the start of the file |
| 0x24 | `00 40` | waveform 0's length in words, 64 |
| 0x26 | 128 bytes | waveform 0, signed 8-bit |

So compared with the module struct:

1. it starts with `"MSH\0"` instead of the length;
2. the volume and waveform tables are only as long as their lengths say, not 128 bytes each;
3. there's an offset per waveform instead of 64 pointers.

The fields and the waveforms themselves (a length in words, then the data) are the same as in
`format.md`.

`mmd` reads both layouts into `OctamedMMD0SynthInstrument` (`mmd/src/mmd/synth.rs`), and the
project keeps them as `InstrumentKind::Synth` or `InstrumentKind::Hybrid` with an `OctamedicSynth`.
`MshLoader` loads `MSH` files (they start with `"MSH\0"`) straight into an instrument, taking the
synth's default decay. The engine doesn't play synths yet, and they aren't written back to modules
yet.
