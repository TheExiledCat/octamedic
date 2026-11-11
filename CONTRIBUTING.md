# Contributing

OctaMEDIC is a Rust clone of OctaMED Pro v4, the Amiga tracker. It's early days: modules play in
the CLI, and the GUI is slowly being built up to match OctaMED's screen pixel for pixel.

The crates:

- `mmd` reads and writes MMD0/MMD1 files. No playback in here.
- `octamedic_core` is the song model and the playback engine (`OctamedicEngine`). Most of the work
  right now happens here.
- `mmd_cli` is a REPL for loading, inspecting and playing `.mmd` files (audio through `cpal`).
- `octamedic_gui` is the editor, on ggez.
- `rustuition` is the Amiga style GUI library the editor is built on (gadgets, Topaz, the
  palette). It doesn't know about OctaMEDIC and will get its own repo at some point, so keep
  anything OctaMED specific in `octamedic_gui`.
- `octamedic_util` has small helpers like `.flip()`, `.boxed()` and base-32 numbers.

## Building

You need Rust 1.85 or newer and an audio device. 1.85 is the MSRV (`rust-version` in the workspace
`Cargo.toml`), so don't use std functions newer than that. If you need one, add a stand-in to
`octamedic_util`, like `.flip()` for `bool::toggle`.

```sh
cargo build
cargo run -p mmd_cli -- example_meds/example.mmd0
cargo run -p octamedic_gui -- example_meds/example.mmd1   # the module is optional
```

On WSL, the CLI and GUI folders each have two scripts that pass their arguments on:
`run-windows.sh` builds and runs the Windows build (native audio), and `run-x11.sh` runs it in WSL
over X11, because under WSLg's Wayland the window closes straight away.

On Windows the GUI doesn't open a console, even from `cargo run`, so nothing it prints shows up.
Errors from starting up go in the debug console (Ctrl+`), and a panic or any logged error goes in
`crash.log` in the config folder (`%LOCALAPPDATA%\octamedic`); [docs/FILES.md](docs/FILES.md)
has what's in it. The settings
kept between runs are in `config.toml` there; [docs/CONFIG.md](docs/CONFIG.md) has every setting.

On a Mac, `octamedic_gui/macos/bundle.sh` turns a release build into `OctaMEDIC.app`, the same
way the release workflow does. How to run it is at the top of the script.

In the REPL, `help` lists everything. The ones you'll use most are `inspect`, `blocks`,
`sequence`, `instruments`, `play`, `record` (song or block to wav) and `exit`.

## Before you open a PR

Run `./fmt.sh`. It applies clippy's fixes, formats with `rustfmt.toml` (the import grouping needs
nightly rustfmt) and runs clippy again. Then `cargo build && cargo test`. Clippy shouldn't have any
errors.

If you touched playback, listen to `example_meds/example.mmd0` and `example.mmd1` before and after.

Keep functions small. Doc comments on internal stuff aren't needed, only when the why isn't
obvious from the code.

Branch off `main` and say in the PR what changed and why.

## Using AI

The code itself should be written by you. Small help is fine, but if a PR reads like it was
generated wholesale I'll probably ask you to rework it.

Tests are a different story: write them with AI if you like, as long as you've actually gone
through them and checked they test the right thing (a test that just repeats what the code does
proves nothing).

Docs that aren't code (this file, the guides in `docs/`, PR descriptions) can be AI written, I
don't mind.

## Where things are explained

- [docs/PLAYER.md](docs/PLAYER.md): how playback works (Paula, DMA, the CIA timer) and how the
  engine in `octamedic_core/src/engine/` copies it. The hardware model is in
  `octamedic_core/src/amiga/`.
- [rustuition/README.md](rustuition/README.md): the GUI library, and how to write gadgets with
  it. OctaMED's gadgets are Amiga Intuition gadgets, so
  [Intuition Gadgets](https://wiki.amigaos.net/wiki/Intuition_Gadgets) on the AmigaOS wiki is
  useful for how they should behave (e.g. `GACT_IMMEDIATE`, which is why our buttons click on
  press).
- [octamedic_gui/FRAMEWORK.md](octamedic_gui/FRAMEWORK.md): what the GUI adds on top of
  rustuition (keys, OctaMED's fonts and pens) and how its screens fit together.
- `mmd/format.md` is the original MMD format documentation. The parsers in
  `mmd/src/mmd/parser.rs` are what we actually go by.
- [docs/CONFIG.md](docs/CONFIG.md): `config.toml`, the settings kept between runs.
- [docs/FILES.md](docs/FILES.md): the rest of the files: how instruments get loaded (WAV, FLAC,
  MP3, raw), multi-octave samples, and how synth sounds are stored on their own.

Not yet please: MIDI, and playing synth instruments (they come after effect commands). Synths
are read and kept, just not played.
