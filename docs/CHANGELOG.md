# Changelog

## v0.0.1 - 2026-11-08

The first demo. Someone who knows OctaMED Pro v4 can sit down, make a short song from samples start
to finish, save it, and open it again later, here or in the real OctaMED. The original OctaMED
manual is the manual; what's different is listed below. Anything that isn't there yet is ghosted
in the interface and says "Not implemented yet".

### Added

**The editor (GUI)**

- OctaMED Pro v4's screen, redrawn pixel for pixel: its panels, gadgets, fonts, 12-bit palette and
  keys
- Paula's four channels emulated with the PAL or NTSC clock, OctaMED v4's period tables with
  finetune, and the LED filter on the FILTER button
- Songs: loading MMD0 and MMD1 modules with their notes, instruments and song settings, saving as
  MMD1 with instruments (SAVE SONG), which the real OctaMED opens, CLEAR SONG and CLEAR ALL
- Play and continue song or block, with tempo and the volume command (0C), read as hex or decimal
  per VOL's HEX/DEC. Shift-click on HEX/DEC converts the song's volume commands so they play the
  same
- Instrument and sample editor previews play on a channel of their own, so they don't cut into the
  song's tracks
- The block editor: notes, instruments, command digits, holds (-|-), highlights and octaves,
  deleting and inserting notes, spaces and tracks (Del, Backspace and their Shift and Alt
  versions), and selecting ranges with a right drag, SELECT TRACK, SELECT BLOCK and CANCEL
- Cut, copy and paste for tracks and blocks (the BLOCK panel's TRK:/BLK: buttons, Amiga-x/c/v and
  Amiga-X/C/V) and for ranges (the RANGE panel's CUT, COPY, PASTE, PASTE SELECT and CLEAR)
- The BLOCK panel: NEW BLOCK, NEW HERE and their DEFs, LINES, EXPAND/SHRINK, inserting and deleting
  lines and tracks, highlight spacing, DELETE LAST/THIS, SPLIT and JOIN
- The RANGE panel's volume slides (SLIDE: VOL and Amiga+O) and chords (CHRD on the status bar or
  CHORD, with RST, and Shift+Return for holds on the last chord's tracks)
- The EDIT panel: track mutes, keypad maps (TRK ON/OFF, SEL.INSTR, MAP2), octaves, EDIT ON, SPC,
  SPACE=DEL and ADV. Shift+/ and Shift+* on the keypad are the Amiga's ( and )
- The TRANS panel: half step and octave transposes, changing and exchanging notes and instruments,
  deleting an instrument's notes, PLAY TRANSPOSE and TEXT
- Sequence editing: inserting and deleting entries and changing an entry's block
- Instruments: loading WAV, MP3, FLAC, IFF and raw samples (converted to 8-bit mono the way an
  Amiga plays them), saving them as raw, IFF or WAV, and their volume, transpose, finetune, hold
  and decay (0 to 99)
- The sample editor: ranges, zoom, cut, copy, paste, clear, reverse, mix, freehand drawing, play
  range and display, BUFFSIZE, loops (LOOP ON, S/E, the arrows and dragging the markers),
  PITCH/DEST, CHANGE RATE, OCT up and down, and a line showing the play position
- Quitting with Ctrl+Q, the close button or Cmd+Q asks first if there are unsaved changes
- On Windows the GUI opens without a console. Startup errors show in the app (Ctrl+` opens the
  debug console) and crashes are written to `crash.log` (see [docs/FILES.md](FILES.md))

**Beyond the original**

- Undo and redo in the block editor: Ctrl+Z, and Ctrl+Y or Ctrl+Shift+Z
- Rendering the song to WAV from the GUI
- VOL's OUTPUT: picking the sound device and mixing to mono
- Switching between PAL and NTSC with Ctrl+F12
- CRT style screen filters (Ctrl+F11): scanlines, chroma, VHS wobble, colour, curvature, vignette
  and noise, in any order and each with its own settings, and Toggle all
- All of these settings are remembered between runs in `config.toml` (see
  [docs/CONFIG.md](CONFIG.md))

**The command line tool (`mmd_cli`)**

`mmd_cli song.mmd1` opens a module in a prompt with these commands (`help` lists them):

- `inspect`, `blocks`, `block`, `sequence`, `instruments` and `palette` to look inside the module
- `play` to play it (`play -v 48` for a volume from 0 to 64)
- `record song out.wav` to render the song to WAV (`-s` for the sequence position to start from),
  or `record pattern 3 out.wav` for one block, with `-v` for the volume and `-r` for the sample
  rate
- `sample` and `wavexport` to look at and export an instrument's sample as WAV
- `rewrite` to write the module back out, a check that a module survives a round trip

`mmd_cli song.mmd1 --rewrite` does the rewrite without opening the prompt.

### Fixed

- Rendering the song to WAV no longer counts as saving it, so quitting, LOAD SONG and CLEAR ALL
  still ask about unsaved changes afterwards
- The gadget on the copyright bar asks about unsaved changes before quitting, like QUIT and Ctrl+Q

### What's different from the original

Things the real OctaMED Pro v4 has that this version doesn't, or does differently:

- No programmable keys yet: the EDIT panel's Shift+0 to 9, L.ALT, R.ALT and PICK: are ghosted.
  Right Alt is the Amiga key here, so R.ALT will need another key
- The volume command (0C) is the only sample command that plays. The others are kept in the song
  and saved but have no effect
- No SPREAD, SLIDE: 1 and 2 or NOTE ECHO on the RANGE panel yet (ghosted)
- Synth and hybrid sounds are loaded and kept, but don't play
- Four tracks only: the BLOCK panel's 8, 12 and 16 are ghosted, as there's no split channel mode or
  MIDI yet
- No MIDI in or out and no routing to audio interface outputs
- Songs are saved as MMD1 with instruments only. MMD0, and MMD1 without instruments, can be loaded
  but not saved
- ProTracker and SoundTracker modules can't be loaded
- The footer's Mem shows the song's chip memory and its size as a module, not the Amiga's memory
  left, as there's no machine memory setting yet
- Keys can't be remapped
- Undo only covers the block editor. Block list changes (new, insert, delete, split, join) can't be
  undone and clear the undo history
- Hold symbols (-|-) work, but command 08 (hold and decay) doesn't yet
- Not yet checked against the real OctaMED, so they may differ:
  - Whether tracks and blocks have separate copy buffers, and what PASTE and SWAP do when the
    buffer and the track or block differ in length, and whether a block PASTE takes on the
    buffer's size
  - Whether ranges share a buffer with tracks or blocks. Here they have their own
- Not in the original: undo, rendering to WAV, the output settings, the PAL/NTSC key and the screen
  filters (see Added)

### Testing it

This is a demo, and its point is to find where it doesn't behave like OctaMED Pro v4. The most
useful testing is side by side with the real thing:

1. **Get the real OctaMED running.** OctaMED Pro v4 on a real Amiga, or in an emulator such as
   WinUAE or FS-UAE with Kickstart 2.0 or later. That's your reference: when the two disagree, the
   real OctaMED is right, not this one.
2. **Do the same thing in both.** Load the same module, or make the same short song, and compare
   how it sounds and what the screen shows: notes, commands, holds, the sequence, instrument
   settings and the panels' numbers. Try the edges too: empty blocks, the first and last line,
   very short or very long samples, maximum and minimum values.
3. **Round-trip your songs.** Save a song here, open it in the real OctaMED, and check that it
   looks and plays the same, then save it there and open it here again. A song that doesn't
   survive the trip is a bug worth reporting.
4. **Compare the sound by ear, or better, by recording.** Render the song with `mmd_cli`'s
   `record song` or the GUI's render to WAV, and record the same song from the emulator. Timing,
   volume, pitch and loop differences show up clearly side by side.
5. **Try the ghosted features last.** Anything ghosted is known to be missing (see above), so it
   doesn't need a report.

**Reporting what you find.** Open an issue on
[GitHub](https://github.com/TheExiledCat/octamedic/issues/new/choose):

- Use the **bug report** template for crashes and for anything that behaves differently from the
  real OctaMED. Say what you did step by step, what OctaMEDIC did, and what the real OctaMED does
  instead. If you can, attach the module (no copyrighted samples, please) or WAV recordings of
  both.
- Say which system you're on, and whether it was the GUI or `mmd_cli`. If the GUI crashed, attach
  `crash.log` from the config folder: `%LOCALAPPDATA%\octamedic` on Windows, `~/.config/octamedic`
  on Linux, `~/Library/Application Support/octamedic` on macOS.
- One issue per problem, and check the open issues and the known issues below first.
- Ideas, and things the original does that you'd like next, go in a **feature request**.

### Known issues

- Low volume commands go silent when the track or master volume is below 64: `0C01` on a track at
  63 rounds down to nothing. What the real OctaMED does here still needs checking.
- Sample editor: REV over a range that crosses from one octave of a multi-octave sample into
  another scrambles the sample, even though its length stays the same. Edits that would change a
  multi-octave sample's length are turned down.
- macOS: double-clicking a module in Finder doesn't open it. Pass it with
  `open -a OctaMEDIC --args /path/to/song.mmd1` instead, with OctaMEDIC closed.
- The builds aren't signed or notarized, so Windows and macOS warn the first time they open (the
  README's installation steps say how to get past it).
