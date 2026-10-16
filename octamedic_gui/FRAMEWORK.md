# UI framework

A small retained-mode UI on top of [ggez](https://ggez.rs) (drawing) and [taffy](https://github.com/DioxusLabs/taffy) (flexbox layout). It lives in `src/framework/`; the screens built with it live in `src/app/`.

## The model

- **One tree.** `Ui` owns a `TaffyTree` where every node carries its widget. Layout and widgets can't drift apart, and every widget is addressed by its `NodeId`.
- **Widgets are reached by id and type, not through their parents.** `ui.get_mut::<Label>(id)` gives you the concrete type, so a feature like `set_text` exists only on the widgets that have it.
- **`Ui` handles input.** It hit-tests by walking down the tree only into rects under the cursor. It tracks hover and press centrally, and turns press + release on the same widget into a `UiEvent::Clicked`.
- **Events bubble.** An event goes to the widget it happened on, then to each ancestor until one handles it. Events nobody handles end up in `ui.take_unhandled_events()` for the app.

| File | What's in it |
|---|---|
| `framework/ui.rs` | `Ui`, `UiEvent` |
| `framework/widget.rs` | the `Widget` trait, `DrawState` |
| `framework/widgets/` | `Panel` (a plain rect, like an HTML div), `Label`, `Button` |
| `framework/color.rs` | `Color::hex("E4A0B7")` |

## The `Widget` trait

Only `draw` is required. Every other method has a default that does nothing.

| Method | Called when | Use it for |
|---|---|---|
| `mount(&mut self, ui, node)` | once, by `ui.add`, before the widget goes into the tree | adding child widgets and keeping their ids |
| `update(&mut self, ui, node)` | every frame, before layout | timers, syncing children to state |
| `on_event(&mut self, ui, node, event) -> bool` | an event on this widget or a descendant | reacting to clicks. Return `true` to stop bubbling |
| `draw(&self, canvas, state)` | every frame, before its children | backgrounds, text |
| `draw_after_children(&self, canvas, state)` | every frame, after its children | borders, overlays |
| `measure_content(&self, ctx, known, available)` | during layout, for leaf nodes taffy can't size from their style | text size |
| `is_interactive(&self) -> bool` | hit testing | `true` for widgets that take clicks. Clicks on non-interactive widgets go to the nearest interactive ancestor |

`DrawState` gives `draw` the widget's screen `rect`, its `content_rect` (the rect without padding and border), and `hovered` / `pressed`.

## Common tasks

### Setting up and running a `Ui`

```rust
let mut ui = Ui::new(root_style, Panel::new());
ui.add(ui.root(), TitleBar::style(), TitleBar::new());

// every frame, in EventHandler::update:
ui.update();
ui.compute_layout(size, |widget, known, available| widget.measure_content(ctx, known, available));
ui.mouse_moved(pos);                                  // plus mouse_pressed / mouse_released
for event in ui.take_unhandled_events() { /* app-level handling */ }

// in EventHandler::draw:
ui.draw(&mut canvas);
```

Always drain `take_unhandled_events()`, even if you ignore the events, so they don't pile up.

### Adding widgets

```rust
let panel = ui.add(parent, Panel::default_style(), Panel::new().with_background(Color::BLACK));
let title = ui.add(panel, Label::default_style(), Label::new("OctaMEDIC", 24.0));
let play = ui.add(panel, Button::default_style(), Button::new("Play", 18.0));
```

The style is plain taffy: spread a default and override what you need.

```rust
Style { margin: Rect { left: auto(), ..Rect::zero() }, ..Label::default_style() }
```

Children are drawn in the order they're added, so the last child is on top and wins hit tests.

### Changing a widget

```rust
if let Some(label) = ui.get_mut::<Label>(title) {
    label.set_text("New title");
}
Button::set_text(&mut ui, play, "Stop");    // reaches into the button's label
```

`get_mut` marks the node for layout again, because text and similar changes can change its size. Don't call it every frame when nothing changed. Check with `ui.get` first, or only update on change, as `TitleBar` does with its timer.

### Reacting to a click

Handle the click in the nearest widget that owns the button:

```rust
fn on_event(&mut self, ui: &mut Ui, _node: NodeId, event: &UiEvent) -> bool {
    let UiEvent::Clicked { node, button: MouseButton::Left } = *event else { return false };
    if Some(node) != self.play { return false; }    // not ours: let it bubble on
    // ...
    return true;
}
```

Or handle it in the app, if no widget handled it:

```rust
for event in ui.take_unhandled_events() {
    if event == (UiEvent::Clicked { node: play, button: MouseButton::Left }) { /* ... */ }
}
```

### Writing a widget made of other widgets

Keep the settings you need in the struct. Build the children in `mount` and store their ids. They are `Option`s because the ids only exist once mounted.

```rust
pub struct Counter {
    count: u32,
    label: Option<NodeId>,
    button: Option<NodeId>,
}

impl Widget for Counter {
    fn mount(&mut self, ui: &mut Ui, node: NodeId) {
        self.label = Some(ui.add(node, Label::default_style(), Label::new("0", 18.0)));
        self.button = Some(ui.add(node, Button::default_style(), Button::new("+1", 18.0)));
    }

    fn on_event(&mut self, ui: &mut Ui, _node: NodeId, event: &UiEvent) -> bool {
        let UiEvent::Clicked { node, .. } = *event;
        if Some(node) != self.button { return false; }
        self.count += 1;
        if let Some(label) = self.label.and_then(|id| ui.get_mut::<Label>(id)) {
            label.set_text(self.count.to_string());
        }
        return true;
    }

    fn draw(&self, _canvas: &mut Canvas, _state: &DrawState) {}
}

ui.add(parent, Panel::default_style(), Counter { count: 0, label: None, button: None });
```

`Button` itself is built this way: a background plus a `Label` child.

### Switching views (tabs)

Hidden nodes take no space, aren't drawn and get no input, but keep their widgets and state:

```rust
for &(tab_button, view) in &self.tabs {
    ui.set_hidden(view, tab_button != clicked);
}
```

The `Tabs` test widget in `framework/ui.rs` is a complete example.

### Removing widgets

```rust
ui.remove(node);   // removes the node and everything below it
```

A widget can remove its own node from inside its own `on_event`.

### Writing a leaf widget

Implement `draw`. Implement `measure` if taffy needs to size it from content. Implement `is_interactive` if it takes clicks. `panel::fill_rect(canvas, rect, color)` draws a filled rect, and `label::draw_text` / `label::measure_text` handle text in the app font.

## Rules worth knowing

- **During a widget's own `update` or `on_event`, the widget is out of the tree.** `ui.get(node)` on itself returns `None`, so use `self`. Its node holds a private placeholder until the handler returns.
- **`update` doesn't mark anything for layout.** If a widget changes something that affects its own size, it calls `ui.invalidate_layout(node)`. Changes made through `ui.get_mut` are marked automatically.
- **Mouse positions are in window coordinates**, the same space as layout. They aren't mapped onto the virtual screen yet.

## Testing

`Ui` doesn't need a ggez `Context` except for text measuring, so input and layout logic can be unit-tested without a window. Lay the tree out with a measure function that ignores content:

```rust
ui.compute_layout(Size { width: 800.0, height: 600.0 }, |_, known, _| Size {
    width: known.width.unwrap_or(0.0),
    height: known.height.unwrap_or(0.0),
});
ui.mouse_pressed([20.0, 20.0], MouseButton::Left);
ui.mouse_released([20.0, 20.0], MouseButton::Left);
assert_eq!(ui.take_unhandled_events(), vec![UiEvent::Clicked { node: button, button: MouseButton::Left }]);
```

See the tests at the bottom of `framework/ui.rs`.
