# UI framework

The GUI is built on [rustuition](../rustuition/README.md), the Amiga style GUI library in this
workspace; read that first for gadgets, layout, drawing, fonts and buttons. This is what the app
adds on top, in `src/framework/`, and how OctaMED's screens in `src/app/` are put together.

## What the app adds

- `framework/event.rs`: the app's `Event`, rustuition's mouse events plus `Key` and `Text` from
  ggez. It implements `PointerEvent`, so rustuition's gadgets take it as is.
- `framework/widgets/mod.rs`: `Button` and `Slider` aliases with `AppState` and `Event` filled in.
  Use these rather than rustuition's, so handlers know they get an `AppState`. App gadgets
  implement `Gadget<AppState, Event>`.
- `framework/widgets/symbols.rs`: OctaMED's picture button contents (`Blocks`, `MusicSymbol`), and
  `audio_view.rs` the scopes and VU meters.
- `app/fonts.rs`: OctaMED's own fonts, redrawn by hand in `assets/fonts/` since the originals
  aren't around anywhere. `INTERFACE` (OctaMED's own 8x8, for numbers and names like the status
  row, Topaz for anything it doesn't have), `PANEL` (gadget labels like `FILES`, 6 lines, capitals
  only), `SMALL` (the status row flags like `CHRD`, 5 lines), `LABEL` (play panel labels), `TITLE`,
  `AUTHOR` and `EDITOR` (16px wide block editor chars). `cargo test` checks the files parse.
- `app/pens.rs`: OctaMED's pens past rustuition's four, `RED`, `ROSE`, `PINK` and `BLUSH`. The
  palette is the song's, turned into the screen's with `state::screen_palette`.
- `app/mod.rs`: `SCREEN`, PAL hires.

Keep the sizes measured from OctaMED as named constants where they're used, like the status row's
`WIDTHS`.

## Keys

Keys go through a `Keymap` of actions (`keymap.rs`) so they can be rebound by action id later:

```rust
#[derive(Clone, Copy, PartialEq, Debug)]
enum EditorAction { Up, Down }
impl Action for EditorAction {}

let keys = Keymap::new("editor")
    .bind(Key::plain(KeyCode::Up), EditorAction::Up, "Up")
    .bind(Key::plain(KeyCode::Down), EditorAction::Down, "Down");
```

Implement `KeyActions` (`keymap`, `perform`, `describe`) on the gadget and call
`self.dispatch(state, event)` from `handle_event`. Ids are `scope.action`, e.g. `editor.up`.

## The app

`MainView` (`app/main_view.rs`) is the whole screen and handles global keys like Ctrl+Q. Inside
it: `TitleBar` (timer, title, author), `CopyrightBar` (the screen title over the title bar, toggled
by a right click nothing else takes, its gadget quits), `Header` (`app/header/mod.rs`),
`ModeButtons`, `StatusBar` (song position, block, instrument, song), the `BlockEditor`
(`app/main/block_editor.rs`, with track numbers and the block's `TrackCount` over its top rows),
`Footer` (info line and scopes) and `DebugConsole` (toggle with `~`). Most live in a file named
after them in `app/`.

The mode buttons set two modes: `HeaderMode` (which panel the header shows) and `MainMode` (what
fills the screen below). Each button's `ModeBinding` in `app/mode_buttons.rs` says which. Header
panels are in `app/header/`, one file each (`misc.rs` is MISC), and `HeaderPanels::panel` maps
each `HeaderMode` to one. A panel draws inside the header's sunken area and gets the header's
updates and events while it's showing. Main screens are in `app/main/` and `MainScreens::screen`
does the same for `MainMode`. To add one: write the gadget, add a field for it, return it from
`panel` or `screen`.

Change project data through `AppState` methods like `enter_note` and `edit_track`, not directly.
They track which blocks changed and send them to the engine in `AppState::update`, so you hear
edits while it's playing.

`state.debug("...")` adds a line to the debug console.

`app/files.rs` knows the folders: the config folder (`config.toml`, `keybinds.toml`) and the
projects folder inside the music folder. `initialize` creates them at startup.

## Tests

Gadgets only need a window to draw, so you can test their logic on a state with no audio device:

```rust
let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
```

Set the gadget's area by hand if the test needs one (the button tests do), then send it events.
