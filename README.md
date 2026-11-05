# OctaMEDIC Pro V4 (WIP v0.0.1 IN DEVELOPMENT)

**Note**: the contents of this README are placeholder until the first demo version releases, there are no releases published yet.

A faithful clone of OctaMED Professional v4

![Octamed Placeholder](octamedic.png)

## Features (Not finished)

- Faithful recreation of the 8 bit sound engine written in Rust using ggez for rendering and cpal for audio playback
- Reads and Writes real MED modules
- All the original settings and keybinds
- Full Midi support
- Original synthesiser and sample editor
- Song rendering as WAV files

## Quality of life goals

- Undo buffer, for fixing accidental deletions
- Optional setting to let the block editor's cursor onto the instrument's left digit, so instruments above 0V can be typed in directly (the original skips it, so only 0 to V can be typed)
- Configurable line highlights, like a default spacing added to every new block automatically, or showing them only on the line numbers instead of across the whole line
- Moving an entry in the play sequence list (the original only has insert and delete)
- Extra keybinds for block selection, creation, UI highlighting etc (Full mouseless possibilities)
- Support for importing modern WAV, MP3, FLAC formats (in mono, converted to 8 bit)
- Support for rendering as WAV and MP3
- Automatic Sample folder storage for quick loading samples and modules
- 41-44khz sample rate, which still has the original aliasing simulated
- Cross platform for Windows 10+, Linux and Mac
- Full audio interface support using direct or ASIO style drivers (based on platform)
    - Includes midi in and out for audio interfaces, with both transport and clock sync capabilities
    - Track separation support for sending up to 8 octamed channels out
    - Record master from Internal audio or from interface IO or both while rendering
- Can simulate both PAL and NTSC clock speeds and views
- original 12 bit amiga color palette

## Contributing

- [CONTRIBUTING.md](CONTRIBUTING.md) for building, code style and PRs
- [docs/PLAYER.md](docs/PLAYER.md) for how the player works (Paula, DMA, the CIA timer)
- [rustuition/README.md](rustuition/README.md) for the Amiga style GUI library, and
  [octamedic_gui/FRAMEWORK.md](octamedic_gui/FRAMEWORK.md) for how the GUI uses it
- issue templates: [bug report](.github/ISSUE_TEMPLATE/bug_report.md), [feature request](.github/ISSUE_TEMPLATE/feature_request.md)

## Documentation

For a manual, acquire the original OctaMED manual

There will be a tiny addendum that will specify all the changes made from the original.

For questions about the documentation and or the project or bug / authenticity problems please make a github issue.

## Installation

To install download a release from the [releases](https://github.com/TheExiledCat/octamedic/releases) page

On macOS the GUI comes as `OctaMEDIC.app`: `aarch64-apple-darwin` is for Apple Silicon Macs and `x86_64-apple-darwin` for Intel ones. It isn't notarized by Apple, so the first time you open it macOS says it can't check it for malware and won't open it. To open it anyway, go to System Settings > Privacy & Security and click Open Anyway, or run `xattr -dr com.apple.quarantine OctaMEDIC.app`. The CLI needs the same the first time.

It is also possible to build from source using the rust cargo toolchain. This way you get the newest changes (but also the breaking ones!) immediately.

## In this repo
- mmd
    - parser and structs for original octamed files
- mmd_cli
    - cli tool for reading octamed mmds and extracting wav files
- octamedic_core
    - core library for octamedic, including the playback engine
- octamedic_gui
    - full octamed clone made using ggez
- octamedic_util
    - small helpers shared by the other crates
- rustuition
    - custom made UI toolkit to recreate the AmigaOS look from Intuition
## Roadmap

What's needed for the first release is tracked in [V0.0.1TARGETS.md](V0.0.1TARGETS.md).

- [X] Loading real mmd file's metadata, note and instrument data
- [ ] Loading protracker/soundtracker supported modules
- [X] Loading extra metadata from mmd files
- [X] Initial engine iteration which can playback real mmd files with just note data (no commands yet)
- [X] Initial editor iteration that can edit notes and patterns of existing mmd files but cant edit instrument data
- [ ] Write back modules to:
    - [ ] MMD0
    - [ ] MMD1 (No Instr)
    - [X] MMD1 +Instruments
- [ ] Sample Commands
- [ ] Midi commands
- [ ] Synth commands
- [X] Loading wav, mp3, flac files as instruments
- [X] Loading IFF files
- [ ] Editing instruments
- [ ] Keybind mapping
- [ ] Midi in support
    - [ ] Omni in support
    - [ ] Midi control mapping 
- [ ] Midi mapping
- [ ] Track to audio interface routing support
- [X] Gui overhaul to make it look like octamed
- [ ] Octamed configuration support
- [ ] Custom configuration overrides
