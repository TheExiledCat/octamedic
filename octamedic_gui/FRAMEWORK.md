# UI framework

A small immediate-mode UI on top of [ggez](https://ggez.rs). The framework is in `src/framework/`,
OctaMED's screens built with it are in `src/app/`.

How it works:

- Everything is drawn into one 640x256 PAL hires screen, in its own pixels, and then scaled into
  the window (`screen.rs`, which also has `PIXEL_ASPECT` and `ScaleMode`). Hires pixels are twice
  as tall as they're wide. Positions are whole screen pixels (`i32`) so you can use the numbers
  measured off OctaMED directly.
- No widget tree. A widget is a struct implementing `OctamedicWidget` that owns its children as
  fields and gets told where to draw. Layout is just code splitting `Rect`s.
- Everything a widget shows comes from `AppState` (`app/state.rs`), and everything a click or key
  changes goes into it.
- Events go down to children until one returns `true`.

## Widgets

```rust
pub trait OctamedicWidget {
    fn update(&mut self, state: &mut AppState) {}
    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool { false }
    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter);
}
```

`update` runs every frame before drawing, for copying things out of the state (a button's text,
say). `handle_event` gets clicks and keys (`Event`, in screen pixels). `render` draws, and takes
`&mut self` because widgets remember where they were drawn: a `Clickable` needs its area to know if
a click hit it. So an area only exists once something's been drawn there.

## Layout

`Rect` (in `rect.rs`, with `Point`) is `x, y, w, h`. The helpers: `split_top(h)` /
`split_left(w)` give the first part and the rest, `columns(&[w, ...])` gives a row of rects with
those widths, `split_columns(n)` / `split_rows(n)` give `n` even ones (leftover pixels go to the
first), `inset(x, y)` shrinks it, and `inside_frame(rect, thickness)` is what's left inside
`Painter::frame`.

Keep the sizes measured from OctaMED as named constants where they're used, like the status row's
`WIDTHS`.

## Drawing

`Painter` (`painter.rs`) draws with `Pen`s, the 8 colours of the song's palette: `BLACK`, `LIGHT`,
`SHADOW`, `FILL`, `RED`, `ROSE`, `PINK`, `BLUSH`. A module with its own colours shows them.

It all comes down to filled rects, so it can draw on any `RenderTarget` (`target.rs`), meaning
anything that can fill and clip a rect. The app uses ggez's `Canvas`. To draw into a framebuffer
instead, implement the trait for it and pass it to `Painter::new`.

What's there: `fill` / `fill_translucent`, `outline` (1px box), `rect` (border on chosen
`Sides`), `triangle` and `circle` (drawn row by row so they stay on the pixel grid, from
`shape.rs`), text with `text` (Topaz), `text_in_font` and `text_bold` (bold the Amiga way, drawn
again 1px to the right), and `line` / `line_in` for styled text. `frame` is OctaMED's raised frame:
light top and right, shadow left and bottom, meeting on a slant. `frame_inverted` is the sunk in
one, like around the header panel and the mute boxes. Borders are `thickness` lines tall at top
and bottom and twice that many pixels wide at the sides, so they look even on tall pixels.

## Fonts and text

`Font` (`font.rs`): `Topaz` for the block editor, `Interface` (OctaMED's own 8x8, for numbers and
names like the status row), `Panel` (gadget labels like `FILES`, 6 lines, capitals only) and
`Small` (the status row flags like `CHRD`, 5 lines).

The last three are read from glyph files in `assets/fonts/`. Each glyph is its character on its own
line, then one line of `#` and `.` per font line, then a blank line:

```
A
..####..
.##..##.
...
```

Glyphs can be different widths, up to 16 pixels. Files are loaded the first time a font is used,
and `cargo test` checks they parse. Anything `Interface` doesn't have falls back to Topaz.

Mixed styles are a `Line` of `Span`s (`text.rs`), each with its own font, bold, title shadow or
pen:

```rust
let title = Line(vec![
    Span::new(TITLE).shadowed(),
    Span::new(" "),
    Span::new(VERSION),
]);
painter.line(x, y, &title, Pen::LIGHT);
```

A span with no pen uses the line's pen, so it inverts along with a button. `Line::fit` cuts a line
to whole characters and `Line::index_at` tells you which character is at a pixel.

## Buttons

```rust
let button = Button::new("FILES")
    .align(Align::Left)
    .font(Font::Panel)
    .on_click(|state, click| state.set_modes(HeaderMode::Files, MainMode::Files));
```

`Button::new` takes any `ButtonContent`: `&str`, `String`, `Line`, a `Triangle` (`.with_bar()`
puts a bar across the point, like jump to start/end), `Blocks` or `MusicSymbol`s. New content
types implement `render`, plus `index_at` if it matters which part got clicked. `set_text` /
`set_content` change it later.

It clicks the moment it's pressed, like OctaMED's gadgets. The handler gets the state and a
`ButtonClick` with the mouse button and `char_index` (the label character under the pointer).
`toggle` / `set_toggled` draw it inverted, which is how OctaMED shows a gadget that's on; holding
it down doesn't change the look. `border(n)` sets the frame thickness. The label is lined up with
`align` inside the frame and padding (`padding(n)` or `padding_xy(x, y)`) and cut to what fits; a
label too long for the padding moves into it, as `CHRD` does.

`Grid::new(columns, rows, widgets)` lays widgets out in rows and passes events and drawing to
them (the mode buttons, the mute boxes). There's also a slider and the audio view in
`framework/widgets/`.

## Clicks and keys

`Clickable` (`clickable.rs`) gives `Click::Clicked` on a press over its area and also takes that
press's release, wherever it is. Call `set_area` in `render` and `handle` in `handle_event`
(`Button` does both for you).

Keys go through a `Keymap` of actions (`keymap.rs`) so they can be rebound by action id later:

```rust
#[derive(Clone, Copy, PartialEq, Debug)]
enum EditorAction { Up, Down }
impl Action for EditorAction {}

let keys = Keymap::new("editor")
    .bind(Key::plain(KeyCode::Up), EditorAction::Up, "Up")
    .bind(Key::plain(KeyCode::Down), EditorAction::Down, "Down");
```

Implement `KeyActions` (`keymap`, `perform`, `describe`) on the widget and call
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
does the same for `MainMode`. To add one: write the widget, add a field for it, return it from
`panel` or `screen`.

Change project data through `AppState` methods like `enter_note` and `edit_track`, not directly.
They track which blocks changed and send them to the engine in `AppState::update`, so you hear
edits while it's playing.

`state.debug("...")` adds a line to the debug console.

`app/files.rs` knows the folders: the config folder (`config.toml`, `keybinds.toml`) and the
projects folder inside the music folder. `initialize` creates them at startup.

## Tests

Widgets only need a window to draw, so you can test their logic on a state with no audio device:

```rust
let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
```

Set the widget's area by hand if the test needs one (the button tests do), then send it events.
