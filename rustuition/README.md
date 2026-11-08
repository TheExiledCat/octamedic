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
one, and `bevel` draws one in any two pens. Borders are `thickness` lines tall at top and bottom and twice that many pixels wide at the
sides, so they look even on tall pixels. `fill_color` fills with a colour from outside the palette, like
the Amiga's sprites have. `clipped` runs some drawing clipped to a rect, and `clip` is where
drawing goes now, the whole target outside `clipped`.

## Filters

Filters (`filters.rs`) change a finished `Framebuffer` after everything's been painted. There's
`Scanlines` (darkens every `spacing`th line), `ChromaticAberration` (red moves one way, blue the
other), `VhsWobble` (lines shift sideways in a rolling wave with some jitter) and `ColorAdjust`
(hue, saturation, brightness, contrast and warmth, like a monitor's knobs). A closure taking
`&mut Framebuffer` is a filter too, and a list of them runs in order:

```rust
let wobble = VhsWobble { frame: frame_count, ..VhsWobble::default() };
let filters: [&dyn Filter; 3] = [&wobble, &ChromaticAberration::default(), &Scanlines::default()];
filters.as_slice().apply(&mut screen);
```

They work at the framebuffer's own resolution, so on a 640x256 screen a scanline is a whole line.
`pixels_mut` and `row_mut` give your own filters the pixels.

`FilterParams` lists a filter's settings as `Param`s (name, range, step, default) and gets and sets
them by index, so one dialog can edit any filter. `AdjustableFilter` is a filter with params, for
keeping different ones in a list. Call `advance_frame` once a frame to move animated ones like
`VhsWobble` on.

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

`Button::new` takes any `ButtonContent`: `&str`, `String`, `Line`, a `Triangle` (`.with_bar()` puts
a bar across the point, like jump to start/end, and `.bar_gap(n)` spaces it off), an `ArrowLabel` or
an `IncDec` (plus or minus). A closure taking `(ContentArea, Pen, &mut Painter)` works too, for a
one-off picture. Your own content types implement `render`, plus `index_at` if it matters which part
got clicked. `set_text` / `set_content` change it later.

It clicks the moment it's pressed, like Intuition's `GACT_IMMEDIATE` gadgets. The handler gets the
state and a `ButtonClick` with the mouse button and `char_index` (the label character under the
pointer); or leave the handler out and poll `clicked()`. `repeat(delay, interval)` makes it keep
clicking while it's held, like a held key; the repeats come from `update`, so call that every frame.
`toggle` / `set_toggled` draw it inverted, the Amiga way of showing a gadget that's on;
`highlight(Highlight::Recess)` sinks the frame in as well, like OctaMED's track on/off boxes.
`sunken()` keeps the frame sunk in without inverting, for a box that shows a value.
`border(n)` sets the frame thickness. The label is lined up with `align` inside the frame and
padding (`padding(n)` or `padding_xy(x, y)`) and cut to what fits; a label too long for the padding
moves into it.

`Slider` is a proportional gadget with a dotted track, horizontal or vertical, optionally
reversed; `solid()` draws the track solid and the handle flat instead. `SteppedSlider` (`gadgets/stepped_slider.rs`) is a slider with two arrow buttons after
it that step its value by one and keep stepping while held; its `update(state, current)` hands
back a new value when it's dragged or stepped, from `min` to `max`. `Grid::new(columns, rows,
gadgets)` lays gadgets out in rows and passes events and drawing to them.

`Textbox` is Intuition's string gadget. A click focuses it and puts the cursor on the char under
the pointer, dragging selects, a double click selects a word, and clicking anywhere else, Enter or
Escape lets go. It takes the usual editing keys: arrows (with Ctrl by word, with Shift to select),
Home, End, Backspace, Delete and Ctrl+A. `on_update` gets the new text whenever typing changes it,
`set_text` changes it without calling that, and `max_length` caps it. `sunken()` sinks its frame in. `pens` sets the box, text,
cursor and selection pens, and `mix_pens` draws the text over the cursor and selection in their pen
ORed with the text pen, like separate bitplanes overlapping, so white (1) on pen 6 shows as pen 7. It needs the keys as well as
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

Give text boxes your events before other gadgets, so typing doesn't reach them, and give every
text box every click (`|` rather than `||`), so a box sees the click on another one that unfocuses
it. When the cursor is past the last char and there's no room left in the box, it's drawn on the
last char.

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
(`TextField`, `Checkbox`, `Cycle`, `Dropdown`, `Toggle`, `NumberField`, `ReorderableList`, or a
`Label` whose text changes) and gives back a `Field` that reads it from the `DialogResult`,
already the right type. A `Dropdown`'s list opens under it, or as far up as it needs to stay on
screen. Each button submits or cancels; Enter presses the first submit button,
and Escape and the close gadget cancel unless `cancellable(false)`. `on_submit_checked` can
refuse with a message, which keeps the dialog open.
`on_change` runs when it opens and whenever a component takes an event, and its `DialogFields`
can read, set and disable fields, so one can follow another. `message`, `confirm` and
`text_input` are ready made. New components implement `DialogComponent`, plus `DialogInput` if
they have a value.

`NumberField` is a number you type in or step with - and + buttons (which repeat while held),
kept in its range.
`ReorderableList` (`gadgets/reorderable_list.rs`) is a list of rows, each picking one of the same
options from a dropdown, with up and down arrows to put them in order and an On button.
`can_add`, `can_remove` and `editable` add Add, Del and Edit buttons, and `on_event` gets a
`ListEvent` for every change as it's made, so whatever the list stands for can follow it, like an
effects chain.

## Clicks

`Clickable` (`clickable.rs`) gives `Click::Clicked` on a press over its area and also takes that
press's release, wherever it is, so whatever's under the pointer doesn't see half a click. Call
`set_area` in `render` and `handle` in `handle_event` (`Button` does both for you).
