# Contributing to OctaMEDIC

Thanks for your interest in contributing. This is an open-source Rust clone of OctaMED Professional v4 — a classic Amiga music tracker. The project is in early development; the immediate goal is a working CLI playback engine.

## Project layout

| Crate | Purpose |
|---|---|
| `octamed` | Parses and writes MMD0/MMD1 binary files. No playback logic lives here. |
| `octamedic_core` | In-memory song model and the playback engine (`OctamedicEngine`). This is where most of the near-term work is. |
| `octamed_cli` | Interactive REPL for loading, inspecting, and playing `.mmd` files via `cpal`. |
| `octamedic_gui` | ggez-based graphical editor. Not the current focus — UI comes after the engine works. |

## Getting started

Prerequisites: Rust stable (1.85+), a working audio device.

```sh
git clone <repo>
cd octamedic
cargo build
# Run the CLI REPL
cargo run -p octamed_cli -- example_meds/example.mmd0
```

On WSL, each program has two scripts in its folder, which pass their arguments on:

- `run-windows.sh` builds and runs the Windows build, for native audio, e.g.
  `octamedic_gui/run-windows.sh`.
- `run-x11.sh` runs it in WSL itself through X11, since windows exit straight away under WSLg's
  Wayland.

Inside the REPL:
```
> inspect      # show file metadata
> blocks       # list patterns
> instruments  # list samples
> play         # attempt playback (engine stub — produces silence for now)
> exit
```

## Code style

- Run `./fmt.sh` before committing. It applies clippy's fixes, formats with `rustfmt.toml` (import
  grouping needs nightly rustfmt) and runs clippy again.
- `cargo clippy` must pass with no errors.
- Keep functions focused. If a function is growing to explain itself with comments, split it.
- No doc comments are needed for internal types. Only add them when the _why_ is non-obvious.

## Making a contribution

1. Fork the repo and create a branch from `main`.
2. Make your change. If it touches playback logic, test it against `example_meds/example.mmd0` and `example.mmd1`.
3. Run `./fmt.sh && cargo build && cargo test` — all must pass.
4. Open a PR with a clear description of what changed and why. Reference any related issue.

## Architecture notes

### Sound engine

[docs/PLAYER.md](docs/PLAYER.md) explains how playback works: Paula, DMA, the CIA timer and how
the engine in `octamedic_core/src/engine/` follows them. The hardware itself is modelled in
`octamedic_core/src/amiga/`.

### GUI

The GUI draws OctaMED's 640×256 PAL hires screen in its own pixels, then scales it into the window
(`framework/screen.rs`); hires pixels are twice as tall as they are wide. Each widget draws itself
from `AppState` every frame in `render`, with the song's palette pens (`Painter`), and gets input
through `handle_event`, which returns whether it took the event. Clickable areas use `Clickable`,
which clicks on release over the area it was pressed in, like an Amiga gadget. A `Button`'s click
handler changes `AppState`, so anything a button changes lives in the state. Keys go through a
`Keymap` of actions (`KeyActions`), as in mTrak, so they can be rebound later by action id, e.g.
`main.quit`.

### MMD format reference

See `octamed/format.md` for binary layout documentation. The MMD0/MMD1 parsers in `octamed/src/mmd/parser.rs` are the authoritative reference implementation.

## What not to work on yet

- `octamedic_gui` — the GUI is on hold until the engine produces real audio.
- MIDI I/O, synth instruments, effect commands — these all come after basic sample playback works.
- `OctamedicProject::to_module()` — the round-trip back to MMD format is not needed for CLI playback.
