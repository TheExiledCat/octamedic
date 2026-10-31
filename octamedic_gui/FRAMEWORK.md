# UI framework

A small immediate-mode UI on top of [ggez](https://ggez.rs). It lives in `src/framework/`; OctaMED's
screens built with it live in `src/app/`.

## The model

- **One Amiga screen.** Everything is drawn into a 640×256 PAL hires screen in its own pixels, then
  scaled into the window (`screen.rs`). Hires pixels are twice as tall as they're wide. Positions
  and sizes are whole screen pixels (`i32`), so layouts can use the numbers measured from OctaMED.
- **Widgets draw themselves every frame.** There's no tree to keep in sync. A widget is a struct
  that implements `OctamedicWidget`, owns its children as fields, and is told where to draw in
  `render`. Layout is plain code: split the area you're given with the `Rect` helpers.
- **The state is the model.** Everything a widget shows comes from the one `AppState`
  (`app/state.rs`), and everything a click or key changes goes into it.
- **Events go down until one is taken.** A widget passes an event to its children and returns
  whether something took it, which stops it there.

| File | What's in it |
|---|---|
| `framework/screen.rs` | the screen size, `PIXEL_ASPECT`, `ScaleMode` |
| `framework/rect.rs` | `Rect` and its layout helpers, `Point` |
| `framework/widget.rs` | the `OctamedicWidget` trait |
| `framework/event.rs` | `Event`, in screen pixels |
| `framework/painter.rs` | `Painter` and `Pen`: fills, frames, shapes, text |
| `framework/target.rs` | `RenderTarget`, what a `Painter` draws on, and `Color` |
| `framework/font.rs` | `Font` and the glyph files in `assets/fonts/` |
| `framework/text.rs` | `Span` and `Line`, text in more than one style |
| `framework/shape.rs` | the rows of triangles and circles, `Direction`, `Sides` |
| `framework/clickable.rs` | `Clickable`, an area that takes clicks |
| `framework/keymap.rs` | `Key`, `Keymap` and `KeyActions` |
| `framework/widgets/` | `Button` and its contents, `Grid` |

## The widget trait

```rust
pub trait OctamedicWidget {
    fn update(&mut self, state: &mut AppState) {}
    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool { false }
    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter);
}
```

| Method | Called | Use it for |
|---|---|---|
| `update` | every frame, before drawing | copying from the state into the widget, like a button's text |
| `handle_event` | for each input event, until a widget returns `true` | clicks and keys |
| `render` | every frame, with the area to draw in | drawing, and remembering where children went |

`render` takes `&mut self` because widgets remember where they were drawn: a `Clickable` needs its
area to know whether a click hit it. So an area is only known once something has been drawn there.

## Layout

`Rect` is `x, y, w, h` in screen pixels. Cut areas up with:

| Helper | Gives |
|---|---|
| `split_top(h)` / `split_left(w)` | the first `h` lines or `w` pixels, and the rest |
| `columns(&[w, ...])` | a row of rects with these widths |
| `split_columns(n)` / `split_rows(n)` | `n` even columns or rows filling the rect; leftover pixels go to the first ones |
| `inset(x, y)` | the rect made smaller on every side |
| `inside_frame(rect, thickness)` | what's left inside `Painter::frame` |

Widths and heights are the ones measured from OctaMED, so keep them as named constants where
they're used, e.g. the status row's `WIDTHS`.

## Drawing

`Painter` draws with pens, the 8 colours of the song's palette (`Pen::BLACK`, `LIGHT`, `SHADOW`,
`FILL`, `RED`, `ROSE`, `PINK`, `BLUSH`), so a module with its own colours shows them.

Everything it draws comes down to filled rects, so it draws on any `RenderTarget`: something that
can fill a rect with a colour and clip to a rect. The app's target is ggez's `Canvas`; to draw into
a framebuffer instead, implement the trait for it and pass it to `Painter::new`.

| Method | Draws |
|---|---|
| `fill(rect, pen)` / `fill_translucent` | a solid rect |
| `frame(rect, thickness)` | OctaMED's raised frame: light top and right, shadow left and bottom, meeting on a slant |
| `frame_inverted(rect, thickness)` | the same, sunk in, as around the header's panel and the mute boxes |
| `outline(rect, pen)` | a one pixel box |
| `rect(rect, fill, border, sides)` | a rect with a border on chosen sides |
| `triangle(rect, fill, border, direction)` / `circle(rect, fill, border)` | shapes, row by row so their edges stay on the pixel grid |
| `text(x, y, text, pen)` | text in Topaz |
| `text_in_font(x, y, text, pen, font)` / `text_bold` | text in any font; bold the Amiga way, drawn again a pixel to the right |
| `line(x, y, &line, pen)` / `line_in(rect, &line, pen, align)` | a `Line` of spans |

Borders are `thickness` lines tall at the top and bottom and twice as many pixels wide at the
sides, so they look as thick on the tall pixels.

## Fonts and text

| `Font` | Where OctaMED uses it |
|---|---|
| `Topaz` | the block editor |
| `Interface` | numbers and names, like the status row; OctaMED's own 8 by 8 font |
| `Panel` | gadget labels like `FILES`; 6 lines tall, capitals only |
| `Small` | the status row's flags, like `CHRD`; 5 lines tall |

`Interface`, `Panel` and `Small` are read from glyph files in `assets/fonts/`. Each glyph is its
character on a line of its own, then a line of `#` and `.` for every line of the font, and an empty
line before the next character:

```
A
..####..
.##..##.
...
```

A glyph is as wide as its lines, up to 16 pixels, so glyphs can be different widths. The files are
read when a font is first used, and `cargo test` checks they parse. Characters `Interface` doesn't
have are drawn in Topaz.

Text in more than one style is a `Line` of `Span`s, each with its own font, bold, the title's
shadow, or pen:

```rust
let title = Line(vec![
    Span::new(TITLE).shadowed(),
    Span::new(" "),
    Span::new(VERSION),
]);
painter.line(x, y, &title, Pen::LIGHT);
```

A span without a pen of its own is drawn in the pen the line is drawn in, so it inverts with a
button. `Line::fit` cuts a line to whole characters, and `Line::index_at` says which character is
at a pixel.

## Buttons

```rust
let button = Button::new("FILES")
    .align(Align::Left)
    .font(Font::Panel)
    .on_click(|state, click| state.set_modes(HeaderMode::Files, MainMode::Files));
```

- **Content.** `Button::new` takes anything that's `ButtonContent`: a `&str`, `String` or `Line`,
  a `Triangle` (`.with_bar()` adds a bar across its point, as on jump to start and end), `Blocks`
  or `MusicSymbol`s. For a new kind of content, implement `render` (and
  `index_at` if clicks on parts of it matter). `set_text` and `set_content` change it.
- **Clicks.** A button clicks the moment it's pressed, as OctaMED's gadgets do. The handler gets
  the state and a `ButtonClick` with the mouse button and `char_index`, the character of the label
  under the pointer.
- **Looks.** `toggle` and `set_toggled` draw it inverted, as OctaMED shows a switched on gadget.
  Holding it down doesn't change how it looks. `border(n)` sets the frame's thickness.
- **Placing the label.** `align` lines it up inside the frame and the padding (`padding(n)`, or
  `padding_xy(x, y)` for different amounts across and down), and it's cut to the characters that
  fit inside the frame. A label too long for the padding moves into it, as `CHRD` does.

`Grid::new(columns, rows, widgets)` lays widgets out in rows and passes events and drawing on to
them, as the mode buttons and the mute boxes do.

## Clicks and keys

`Clickable` turns mouse events into `Click::Clicked` when it's pressed over its area, and takes the
release of that press wherever it is. Call `set_area` in `render` and `handle` in `handle_event`;
`Button` does both.

Keys go through a `Keymap` of actions, so they can be rebound later by action id:

```rust
#[derive(Clone, Copy, PartialEq, Debug)]
enum EditorAction { Up, Down }
impl Action for EditorAction {}

let keys = Keymap::new("editor")
    .bind(Key::plain(KeyCode::Up), EditorAction::Up, "Up")
    .bind(Key::plain(KeyCode::Down), EditorAction::Down, "Down");
```

Implement `KeyActions` (`keymap`, `perform`, `describe`) on the widget and call `self.dispatch(state,
event)` from `handle_event`. The action ids are `scope.action`, e.g. `editor.up`.

## The app

| Widget | File | What it is |
|---|---|---|
| `MainView` | `app/main_view.rs` | the whole screen; global keys like Ctrl+Q |
| `TitleBar` | `app/title_bar.rs` | the timer, the title and the author |
| `CopyrightBar` | `app/copyright_bar.rs` | the screen title over the title bar, shown and hidden by a right click nothing else takes; its gadget quits |
| `Header` | `app/header/mod.rs` | the header panel: the header mode's panel, the mode buttons, the status row |
| `Misc` | `app/header/misc.rs` | the MISC panel |
| `ModeButtons` | `app/mode_buttons.rs` | the buttons at the header's top right |
| `StatusBar` | `app/status_bar.rs` | song position, block, instrument, song |
| `BlockEditor` | `app/main/block_editor.rs` | the block editor, with the track numbers and the block's `TrackCount` over its top rows |
| `Footer` | `app/footer.rs` | the info line and the scopes |
| `DebugConsole` | `app/debug_console.rs` | debug lines, shown with the `~` key |

The mode buttons set two modes in the state: `HeaderMode`, which panel the header shows, and
`MainMode`, what fills the screen under it. Each button's `ModeBinding` in `app/mode_buttons.rs`
says which it switches. The header's panels live in `app/header/`, one file each, and
`HeaderPanels::panel` binds each `HeaderMode` to its panel; a panel draws inside the header's
sunken panel and gets the header's updates and events while it shows. The main screens live in
`app/main/`, and `MainScreens::screen` binds each `MainMode` the same way. To add one, make its
widget, add a field for it and return it from `panel` or `screen`.

Change project data through `AppState` methods like `enter_note` and `edit_track` rather than
directly. They track which blocks changed and send those to the engine in `AppState::update`, so
edits are heard while playing.

For debugging, `state.debug("...")` adds a line to the debug console.

`app/files.rs` knows OctaMEDIC's folders: the config folder (for `config.toml` and
`keybinds.toml`) and the projects folder in the music folder. `initialize` creates them at startup.

## Testing

Widgets don't need a window except to draw, so their logic can be tested on a state without an
audio device:

```rust
let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
```

Set a widget's area by hand where a test needs one, as the button tests do, then send it events.
