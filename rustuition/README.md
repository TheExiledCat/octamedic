# rustuition

A small immediate-mode GUI library for Amiga style apps, named after Intuition, AmigaOS's GUI
library. It draws everything in screen pixels with a 12-bit palette, Topaz and bevelled gadgets,
so you can use numbers measured off the real thing directly.

It came out of [OctaMEDIC](../README.md), whose screens are a good example of using it
(`octamedic_gui/src/app/`). [Intuition Gadgets](https://wiki.amigaos.net/wiki/Intuition_Gadgets) on
the AmigaOS wiki is useful for how gadgets should behave (e.g. `GACT_IMMEDIATE`, which is why
buttons click on press).

## Features

- `ggez`: `BatchedCanvas`, a `RenderTarget` for ggez's `Canvas`, and conversions to ggez's rects.
  Without it rustuition only needs `ab_glyph`, and you draw into a `Framebuffer` or your own
  target.

## How it works

- Everything is drawn into one screen in its own pixels, and then scaled into the window.
  `Screen` (`screen.rs`) has the size and how tall the pixels are (`Screen::PAL_HIRES` is 640x256
  with pixels twice as tall as wide). `Screen::viewport` places it in a window with a `ScaleMode`,
  and `Screen::to_screen` turns window positions back into screen pixels. Positions are whole
  screen pixels (`i32`).
- No widget tree. A gadget is a struct implementing `Gadget` that owns its children as fields and
  gets told where to draw. Layout is just code splitting `Rect`s.
- Everything a gadget shows comes from your app's state, and everything a click changes goes into
  it.
- Events go down to children until one returns `true`.

## Gadgets

```rust
pub trait Gadget<S, E = MouseEvent> {
    fn update(&mut self, state: &mut S) {}
    fn handle_event(&mut self, state: &mut S, event: &E) -> bool { false }
    fn render(&mut self, state: &S, area: Rect, painter: &mut Painter);
}
```

`S` is your app's state and `E` its events. `update` runs every frame before drawing, for copying
things out of the state (a button's text, say). `handle_event` gets the events, in screen pixels.
`render` draws, and takes `&mut self` because gadgets remember where they were drawn: a
`Clickable` needs its area to know if a click hit it. So an area only exists once something's been
drawn there.

rustuition's own gadgets only need the mouse. They take any event type that implements
`PointerEvent`, so you can give them your own event enum with keys or whatever else in it:

```rust
enum Event {
    Mouse(MouseEvent),
    Key(MyKey),
}

impl PointerEvent for Event {
    fn mouse(&self) -> Option<MouseEvent> {
        return match self {
            Event::Mouse(mouse) => Some(*mouse),
            _ => None,
        };
    }
}
```

The gadgets take the state and event types as parameters (`Button<S, E = MouseEvent>`), so a type
alias saves writing them every time and lets handlers know what they get:

```rust
type Button = rustuition::gadgets::button::Button<AppState, Event>;
```

Keyboard handling is up to you. OctaMEDIC has a keymap of rebindable actions in
`octamedic_gui/src/framework/keymap.rs`.

## Layout

`Rect` (in `rect.rs`, with `Point`) is `x, y, w, h`. The helpers: `split_top(h)` /
`split_left(w)` give the first part and the rest, `columns(&[w, ...])` gives a row of rects with
those widths, `split_columns(n)` / `split_rows(n)` give `n` even ones (leftover pixels go to the
first), `inset(x, y)` shrinks it, and `inside_frame(rect, thickness)` is what's left inside
`Painter::frame`.

## Colours

Like on the Amiga, everything is drawn with a `Pen`, an index into the screen's palette, so
changing the palette changes everything drawn with it. A palette `Color` (`palette.rs`) is 12-bit
`0RGB` like the Amiga's colour registers, `Color(0x99A)` or `Color::rgb(9, 9, 10)`, with
`channel` / `with_channel` for one of red, green or blue (0 to 15) and `to_rgba` for 8 bits a
channel. `IntuitionColorPalette` holds any number of them, with `get` / `set` and `channel` /
`set_channel` by pen. Pens past the end wrap around.

rustuition's own drawing uses the first four: `Pen::BACKGROUND` (0), `Pen::SHINE` (1),
`Pen::SHADOW` (2) and `Pen::FILL` (3), named after Intuition's draw pens. What the rest mean is up
to your app; OctaMEDIC names them in `octamedic_gui/src/app/pens.rs`.

## Drawing

`Painter` (`painter.rs`) draws with pens from the palette it's given. It all comes down to filled
rects, so it can draw on any `RenderTarget` (`target.rs`), meaning anything that can fill and clip
a rect. There are two: `Framebuffer`, RGB pixels in memory you can read back or upload yourself,
and `BatchedCanvas` for ggez with the `ggez` feature.

```rust
let palette = IntuitionColorPalette::new([Color(0x000), Color(0xCCC), Color(0x667), Color(0x99A)]);
let mut screen = Framebuffer::new(640, 256);
let mut painter = Painter::new(&mut screen, &palette);
painter.frame(Rect::new(10, 10, 100, 20), 1);
let pixels: &[u8] = screen.as_bytes();
```

What's there: `fill` / `fill_translucent`, `outline` (1px box), `rect` (border on chosen
`Sides`), `triangle` and `circle` (drawn row by row so they stay on the pixel grid, from
`shape.rs`), text with `text` (Topaz), `text_in_font` and `text_bold` (bold the Amiga way, drawn
again 1px to the right), and `line` / `line_in` for styled text. `frame` is the raised bevel:
shine top and right, shadow left and bottom, meeting on a slant. `frame_inverted` is the sunk in
one. Borders are `thickness` lines tall at top and bottom and twice that many pixels wide at the
sides, so they look even on tall pixels.

## Fonts and text

`Font` (`font.rs`) is a cheap handle to a `BitmapFont`. `Font::TOPAZ` is Kickstart 2's Topaz,
rasterized from a TTF so it stays sharp. Make your own fonts from glyphs drawn in text, each one
its character on its own line, then one line of `#` and `.` per font line, then a blank line:

```
A
..####..
.##..##.
...
```

```rust
static PANEL_FONT: LazyLock<BitmapFont> = LazyLock::new(|| {
    BitmapFont::new("panel", 6)
        .bearing(1)
        .space(4)
        .capitals_only()
        .with_glyphs(include_str!("panel.txt"))
        .with_fallback(Font::TOPAZ, Glyph::clone)
});
pub const PANEL: Font = Font::new(|| &PANEL_FONT);
```

Glyphs can be different widths, up to 32 pixels. `bearing` is blank pixels either side of every
glyph, `space` how wide a character with no glyph is, and `with_fallback` fills in what the font
doesn't have from another font (`Glyph::widened` makes a double width one). `parse_glyphs` reads a
glyph file on its own, which is handy in a test that checks yours parse.

Mixed styles are a `Line` of `Span`s (`text.rs`), each with its own font, bold, shadow or pen:

```rust
let title = Line(vec![
    Span::new(TITLE).shadowed(),
    Span::new(" "),
    Span::new(VERSION),
]);
painter.line(x, y, &title, Pen::SHINE);
```

A span with no pen uses the line's pen, so it inverts along with a button. `Line::fit` cuts a line
to whole characters and `Line::index_at` tells you which character is at a pixel.

## Buttons

```rust
let button = Button::new("FILES")
    .align(Align::Left)
    .font(PANEL)
    .on_click(|state, click| state.show_files());
```

`Button::new` takes any `ButtonContent`: `&str`, `String`, `Line`, a `Triangle` (`.with_bar()`
puts a bar across the point, like jump to start/end), an `ArrowLabel` or an `IncDec` (plus or
minus). A closure taking `(ContentArea, Pen, &mut Painter)` works too, for a one-off picture.
Your own content types implement `render`, plus `index_at` if it matters which part got clicked.
`set_text` / `set_content` change it later.

It clicks the moment it's pressed, like Intuition's `GACT_IMMEDIATE` gadgets. The handler gets the
state and a `ButtonClick` with the mouse button and `char_index` (the label character under the
pointer); or leave the handler out and poll `clicked()`. `toggle` / `set_toggled` draw it
inverted, the Amiga way of showing a gadget that's on; `highlight(Highlight::Recess)` sinks the
frame in as well, like OctaMED's track on/off boxes. `border(n)` sets the frame thickness. The
label is lined up with `align` inside the frame and padding (`padding(n)` or `padding_xy(x, y)`)
and cut to what fits; a label too long for the padding moves into it.

`Slider` is a proportional gadget with a dotted track, horizontal or vertical, optionally
reversed. `Grid::new(columns, rows, gadgets)` lays gadgets out in rows and passes events and
drawing to them.

`Textbox` is Intuition's string gadget. A click focuses it and puts the cursor on the char under
the pointer, dragging selects, a double click selects a word, and clicking anywhere else, Enter or
Escape lets go. It takes the usual editing keys: arrows (with Ctrl by word, with Shift to select),
Home, End, Backspace, Delete and Ctrl+A. `on_update` gets the new text whenever typing changes it,
`set_text` changes it without calling that, and `max_length` caps it. It needs the keys as well as
the mouse, so your event type implements `TextEvent` too, turning key presses into `TextInput`:

```rust
impl TextEvent for Event {
    fn text_input(&self) -> Option<TextInput> {
        return match self {
            Event::Typed(c) => Some(TextInput::Char(*c)),
            Event::Key(Key::Left) => Some(TextInput::Key {
                key: EditKey::Left,
                shift: false,
                ctrl: false,
            }),
            _ => None,
        };
    }
}
```

Give text boxes your events before other gadgets, so typing doesn't reach them and the box sees
the click that unfocuses it.

Any `Button`, `Textbox` or `Slider` can be disabled with `set_enabled(false)`. It's drawn as usual
and then ghosted with `Painter::ghost`, a dot every fourth pixel and two along on every other line,
like AmigaOS, and it ignores the mouse until it's enabled again.

## Windows and dialogs

`Window` draws a Workbench window: a frame and a title bar with a close gadget
(`closable(false)` leaves that out). It only draws itself; put the contents in
`Window::content_area`, and `Window::height_for` gives the height for so much of them.

`Dialog` is a window with components stacked in it and a row of buttons, like an Amiga
requester, and takes every event while it's open:

```rust
let mut dialog = Dialog::new("Rename").submit_button("OK").cancel_button("Cancel");
let name = dialog.add_field(TextField::new("Name:").text("old").focused());
let dialog = dialog.on_submit(move |state, result| state.rename(result.get(name)));
```

`with` adds something that only shows (a `Label`), `add_field` something with a value
(`TextField`, `Checkbox`, `Cycle`, `PopupList`, `Toggle`, or a `Label` whose text changes) and
gives back a `Field` that reads it from the `DialogResult`, already the right type. Each button submits or
cancels; Enter presses the first submit button, and Escape and the close gadget cancel unless
`cancellable(false)`. `on_submit_checked` can refuse with a message, which keeps the dialog open.
`on_change` runs when it opens and whenever a component takes an event, and its `DialogFields`
can read, set and disable fields, so one can follow another. `message`, `confirm` and
`text_input` are ready made. New components implement `DialogComponent`, plus `DialogInput` if
they have a value.

## Clicks

`Clickable` (`clickable.rs`) gives `Click::Clicked` on a press over its area and also takes that
press's release, wherever it is, so whatever's under the pointer doesn't see half a click. Call
`set_area` in `render` and `handle` in `handle_event` (`Button` does both for you).
