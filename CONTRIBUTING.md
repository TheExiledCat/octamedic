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

Inside the REPL:
```
> inspect      # show file metadata
> blocks       # list patterns
> instruments  # list samples
> play         # attempt playback (engine stub — produces silence for now)
> exit
```

## Code style

- `rustfmt` is configured via `rustfmt.toml` — run `cargo fmt` before committing.
- `cargo clippy` must pass with no errors.
- Keep functions focused. If a function is growing to explain itself with comments, split it.
- No doc comments are needed for internal types. Only add them when the _why_ is non-obvious.

## Making a contribution

1. Fork the repo and create a branch from `main`.
2. Make your change. If it touches playback logic, test it against `example_meds/example.mmd0` and `example.mmd1`.
3. Run `cargo fmt && cargo clippy && cargo build` — all must pass.
4. Open a PR with a clear description of what changed and why. Reference any related issue.

## Architecture notes

### Sound engine

The engine is in `octamedic_core/src/engine/`. The key interfaces:

- **`OctamedicEngine::process(&mut [u8], u32) -> bool`** — called by the `cpal` audio callback to fill a PCM buffer (u8, 128 = silence). Returns `true` when the song ends.
- **`OctamedicTransport::process(&OctamedicProject) -> bool`** — advances the sequencer by one tick. Returns `true` when playback is finished (paused).

The transport handles row/tick sequencing and pattern navigation. The engine is responsible for calling the transport at the right sample offsets and mixing audio into the output buffer.

### Data flow: file → playback

```
.mmd file
  └─ octamed::parser  →  OctamedMMD
       └─ OctamedicProject::from_module()  →  OctamedicProject
            └─ OctamedicEngine::new()
                 └─ OctamedicEngine::process()  →  PCM buffer  →  cpal output
```

### Tempo

`OctamedicTempo::get_tick_rate()` returns ticks per second as a `Frequency`. The engine converts this to samples-per-tick using the output sample rate. The formula used comes from [NostalgicPlayer](https://github.com/neumatho/NostalgicPlayer/blob/main/Source/Agents/Players/OctaMed/Implementation/Mixer.cs) (line 150).

### MMD format reference

See `octamed/format.md` for binary layout documentation. The MMD0/MMD1 parsers in `octamed/src/mmd/parser.rs` are the authoritative reference implementation.

## What not to work on yet

- `octamedic_gui` — the GUI is on hold until the engine produces real audio.
- MIDI I/O, synth instruments, effect commands — these all come after basic sample playback works.
- `OctamedicProject::to_module()` — the round-trip back to MMD format is not needed for CLI playback.
