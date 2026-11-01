use std::{any::Any, marker::PhantomData};

use crate::{
    event::{EditKey, MouseButton, MouseEvent, PointerEvent, TextEvent, TextInput},
    gadget::Gadget,
    gadgets::{button::Button, textbox::Textbox, window::Window},
    painter::{CHAR_HEIGHT, CHAR_WIDTH, Painter, Pen, text_width, text_y},
    rect::Rect,
};

const DEFAULT_WIDTH: i32 = 320;
// around the components and buttons, inside the window
const PADDING: [i32; 2] = [8, 4];
const COMPONENT_GAP: i32 = 3;
const BUTTONS_GAP: i32 = 6;
const BUTTON_HEIGHT: i32 = CHAR_HEIGHT + 4;
const BUTTON_LABEL_PADDING: i32 = 8;
const FIELD_HEIGHT: i32 = CHAR_HEIGHT + 3;
// tall pixels, so it's about square
const CHECKBOX_SIZE: [i32; 2] = [26, FIELD_HEIGHT];
const POPUP_ROW_HEIGHT: i32 = CHAR_HEIGHT + 2;
// the arrow at the right of a popup list's button, rows of 6, 4 and 2
const POPUP_ARROW_WIDTH: i32 = 6;
const POPUP_ARROW_SPACE: i32 = POPUP_ARROW_WIDTH + 6;

/// Something stacked in a dialog, top to bottom, as wide as the dialog.
#[allow(unused_variables)]
pub trait DialogComponent<S, E>: Gadget<S, E> {
    fn height(&self) -> i32;

    /// Draws on top of the whole dialog, after everything else, like an open popup list.
    fn render_overlay(&mut self, state: &S, painter: &mut Painter) {}

    /// Whether it gets every event, before the dialog's own keys and buttons, like a popup list
    /// while it's open.
    fn captures_events(&self) -> bool {
        return false;
    }
}

/// A component with a value: one the user fills in, or one [`Dialog::on_change`] sets. Its value
/// comes back in the [`DialogResult`].
pub trait DialogInput<S, E>: DialogComponent<S, E> {
    type Value: 'static;

    fn value(&self) -> Self::Value;

    fn set_value(&mut self, value: Self::Value);
}

/// Hands back an input's value from a [`DialogResult`], already the right type. Only works on the
/// result of the dialog it came from.
pub struct Field<T> {
    index: usize,
    value: PhantomData<fn() -> T>,
}

// derive would want T: Clone
impl<T> Clone for Field<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Field<T> {}

/// Every input's value, and which button submitted the dialog.
pub struct DialogResult {
    values: Vec<Option<Box<dyn Any>>>,
    button: usize,
}

impl DialogResult {
    pub fn get<T: 'static>(&self, field: Field<T>) -> &T {
        return self.values[field.index]
            .as_ref()
            .and_then(|value| value.downcast_ref())
            .expect("a field gets the value of the input it was made for");
    }

    /// 0 for the leftmost button, which Enter presses too.
    pub fn button(&self) -> usize {
        return self.button;
    }
}

/// A dialog's fields while it's open, handed to [`Dialog::on_change`]. Any field can be read, and
/// set, so one can follow another, like a label showing what a cycle is on.
pub struct DialogFields<'a, S, E> {
    slots: &'a mut [Box<dyn Slot<S, E>>],
}

impl<S, E> DialogFields<'_, S, E> {
    pub fn get<T: 'static>(&self, field: Field<T>) -> T {
        let value = self.slots[field.index].value();
        return *value
            .and_then(|value| value.downcast().ok())
            .expect("a field gets the value of the input it was made for");
    }

    pub fn set<T: 'static>(&mut self, field: Field<T>, value: T) {
        self.slots[field.index].set_value(Box::new(value));
    }
}

/// One place in a dialog's stack of components. The dialog keeps a list of these, since the
/// components themselves are all different types and only inputs have a value.
trait Slot<S, E> {
    fn component(&mut self) -> &mut dyn DialogComponent<S, E>;
    fn height(&self) -> i32;
    /// The input's value with its type hidden, for the [`DialogResult`]. `None` if the component
    /// only shows something.
    fn value(&self) -> Option<Box<dyn Any>>;
    /// Sets the input's value from one with its type hidden. Values of the wrong type are ignored.
    fn set_value(&mut self, value: Box<dyn Any>);
}

/// Holds a component that only shows something, like a [`Label`]. Nothing comes back from it.
struct StaticSlot<C>(C);

impl<S, E, C: DialogComponent<S, E>> Slot<S, E> for StaticSlot<C> {
    fn component(&mut self) -> &mut dyn DialogComponent<S, E> {
        return &mut self.0;
    }

    fn height(&self) -> i32 {
        return self.0.height();
    }

    fn value(&self) -> Option<Box<dyn Any>> {
        return None;
    }

    fn set_value(&mut self, _value: Box<dyn Any>) {}
}

/// Holds a component that gets filled in. Its value is boxed as `Any` here, and the [`Field`]
/// made with it is how [`DialogResult::get`] turns it back into the right type.
struct InputSlot<C>(C);

impl<S, E, C: DialogInput<S, E>> Slot<S, E> for InputSlot<C> {
    fn component(&mut self) -> &mut dyn DialogComponent<S, E> {
        return &mut self.0;
    }

    fn height(&self) -> i32 {
        return self.0.height();
    }

    fn value(&self) -> Option<Box<dyn Any>> {
        return Some(Box::new(self.0.value()));
    }

    fn set_value(&mut self, value: Box<dyn Any>) {
        if let Ok(value) = value.downcast::<C::Value>() {
            self.0.set_value(*value);
        }
    }
}

type OnSubmit<S> = Box<dyn FnMut(&mut S, &DialogResult) -> Result<(), String>>;
type OnCancel<S> = Box<dyn FnOnce(&mut S)>;
type OnChange<S, E> = Box<dyn FnMut(&mut S, &mut DialogFields<S, E>)>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ButtonRole {
    Submit,
    Cancel,
}

struct DialogButton<S, E> {
    label: String,
    role: ButtonRole,
    button: Button<S, E>,
}

/// A [`Window`] with components stacked in it and a row of buttons along the bottom, like an
/// Amiga requester. It takes every event while it's open.
///
/// Each button either submits or cancels, see [`Dialog::submit_button`] and
/// [`Dialog::cancel_button`]. Enter presses the first submit button. The close gadget and Escape
/// cancel, unless the dialog isn't [cancellable](Dialog::cancellable).
pub struct Dialog<S, E> {
    window: Window<S, E>,
    width: i32,
    slots: Vec<Box<dyn Slot<S, E>>>,
    buttons: Vec<DialogButton<S, E>>,
    on_submit: Option<OnSubmit<S>>,
    on_cancel: Option<OnCancel<S>>,
    on_change: Option<OnChange<S, E>>,
    // on_change runs on the first update, so whatever follows another field starts out right
    opened: bool,
    // from the last submit that was refused
    error: Option<String>,
    cancellable: bool,
    closed: bool,
}

impl<S: 'static, E: PointerEvent + TextEvent + 'static> Dialog<S, E> {
    /// With no buttons yet.
    pub fn new(title: impl Into<String>) -> Self {
        return Self {
            window: Window::new(title),
            width: DEFAULT_WIDTH,
            slots: Vec::new(),
            buttons: Vec::new(),
            on_submit: None,
            on_cancel: None,
            on_change: None,
            opened: false,
            error: None,
            cancellable: true,
            closed: false,
        };
    }

    /// Just a message and OK.
    pub fn message(title: impl Into<String>, text: &str) -> Self {
        return Self::new(title).with(Label::new(text)).submit_button("OK");
    }

    pub fn confirm(title: impl Into<String>, text: &str) -> Self {
        return Self::new(title)
            .with(Label::new(text))
            .submit_button("OK")
            .cancel_button("Cancel");
    }

    /// One text field, focused, already holding `text`.
    pub fn text_input(title: impl Into<String>, label: &str, text: &str) -> (Self, Field<String>) {
        let mut dialog = Self::new(title).submit_button("OK").cancel_button("Cancel");
        let field = dialog.add_field(TextField::new(label).text(text).focused());
        return (dialog, field);
    }

    /// Adds something that only shows, like a [`Label`].
    pub fn with(mut self, component: impl DialogComponent<S, E> + 'static) -> Self {
        self.slots.push(Box::new(StaticSlot(component)));
        return self;
    }

    /// Adds something to fill in, and gives back what reads its value from the result.
    pub fn add_field<C: DialogInput<S, E> + 'static>(&mut self, input: C) -> Field<C::Value> {
        self.slots.push(Box::new(InputSlot(input)));
        return Field {
            index: self.slots.len() - 1,
            value: PhantomData,
        };
    }

    pub fn width(mut self, width: i32) -> Self {
        self.width = width;
        return self;
    }

    /// Adds a button that runs `on_submit`, after the ones already there.
    /// [`DialogResult::button`] says which button it was, counting all of them from the left.
    pub fn submit_button(self, label: &str) -> Self {
        return self.add_button(label, ButtonRole::Submit);
    }

    /// Adds a button that closes it and runs `on_cancel`, after the ones already there.
    pub fn cancel_button(self, label: &str) -> Self {
        return self.add_button(label, ButtonRole::Cancel);
    }

    fn add_button(mut self, label: &str, role: ButtonRole) -> Self {
        self.buttons.push(DialogButton {
            label: label.to_owned(),
            role,
            button: Button::new(label.to_owned()),
        });
        return self;
    }

    pub fn on_submit(self, mut handler: impl FnMut(&mut S, &DialogResult) + 'static) -> Self {
        return self.on_submit_checked(move |state, result| {
            handler(state, result);
            return Ok(());
        });
    }

    /// An `Err` keeps the dialog open and shows the message in it.
    pub fn on_submit_checked(
        mut self,
        handler: impl FnMut(&mut S, &DialogResult) -> Result<(), String> + 'static,
    ) -> Self {
        self.on_submit = Some(Box::new(handler));
        return self;
    }

    /// Whether the close gadget and Escape cancel it. They do by default.
    pub fn cancellable(mut self, cancellable: bool) -> Self {
        self.cancellable = cancellable;
        self.window = self.window.closable(cancellable);
        return self;
    }

    pub fn on_cancel(mut self, handler: impl FnOnce(&mut S) + 'static) -> Self {
        self.on_cancel = Some(Box::new(handler));
        return self;
    }

    /// Runs when the dialog opens and whenever one of its components takes an event, to make
    /// fields follow each other.
    pub fn on_change(
        mut self,
        handler: impl FnMut(&mut S, &mut DialogFields<S, E>) + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(handler));
        return self;
    }

    pub fn is_closed(&self) -> bool {
        return self.closed;
    }

    fn result(&self, button: usize) -> DialogResult {
        let values = self.slots.iter().map(|slot| slot.value()).collect();
        return DialogResult { values, button };
    }

    fn submit(&mut self, state: &mut S, button: usize) {
        let result = self.result(button);
        let outcome = match &mut self.on_submit {
            Some(handler) => handler(state, &result),
            None => Ok(()),
        };
        match outcome {
            Ok(()) => self.closed = true,
            Err(error) => self.error = Some(error),
        }
    }

    fn cancel(&mut self, state: &mut S) {
        self.closed = true;
        if let Some(handler) = self.on_cancel.take() {
            handler(state);
        }
    }

    fn changed(&mut self, state: &mut S) {
        let Some(mut handler) = self.on_change.take() else {
            return;
        };
        handler(
            state,
            &mut DialogFields {
                slots: &mut self.slots,
            },
        );
        self.on_change = Some(handler);
    }

    // a component that captures events gets them all, while it does
    fn handle_captured(&mut self, state: &mut S, event: &E) -> bool {
        let captured = self
            .slots
            .iter_mut()
            .position(|slot| slot.component().captures_events());
        let Some(index) = captured else {
            return false;
        };
        self.slots[index].component().handle_event(state, event);
        self.changed(state);
        return true;
    }

    fn handle_components(&mut self, state: &mut S, event: &E) -> bool {
        let handled = self
            .slots
            .iter_mut()
            .any(|slot| slot.component().handle_event(state, event));
        if handled {
            self.changed(state);
        }
        return handled;
    }

    fn press(&mut self, state: &mut S, button: usize) {
        match self.buttons[button].role {
            ButtonRole::Submit => self.submit(state, button),
            ButtonRole::Cancel => self.cancel(state),
        }
    }

    fn first_submit_button(&self) -> Option<usize> {
        return self
            .buttons
            .iter()
            .position(|button| button.role == ButtonRole::Submit);
    }

    fn handle_key(&mut self, state: &mut S, event: &E) -> bool {
        let Some(TextInput::Key { key, .. }) = event.text_input() else {
            return false;
        };
        match key {
            EditKey::Enter => match self.first_submit_button() {
                Some(button) => self.submit(state, button),
                None => return false,
            },
            EditKey::Escape if self.cancellable => self.cancel(state),
            _ => return false,
        }
        return true;
    }

    fn handle_buttons(&mut self, state: &mut S, event: &E) -> bool {
        if self.window.handle_event(state, event) {
            if self.window.take_close_click() {
                self.cancel(state);
            }
            return true;
        }
        let pressed =
            self.buttons
                .iter_mut()
                .enumerate()
                .find_map(|(index, DialogButton { button, .. })| {
                    let handled = button.handle_event(state, event);
                    handled.then(|| button.clicked().map(|_| index))
                });
        match pressed {
            Some(Some(index)) => self.press(state, index),
            Some(None) => {}
            None => return false,
        }
        return true;
    }

    fn components_height(&self) -> i32 {
        let heights: i32 = self.slots.iter().map(|slot| slot.height()).sum();
        let gaps = COMPONENT_GAP * (self.slots.len() as i32 - 1).max(0);
        return heights + gaps;
    }

    fn error_height(&self) -> i32 {
        return if self.error.is_some() {
            COMPONENT_GAP + CHAR_HEIGHT
        } else {
            0
        };
    }

    fn window_rect(&self, area: Rect) -> Rect {
        let contents = self.components_height() + self.error_height();
        let inside = 2 * PADDING[1] + contents + BUTTONS_GAP + BUTTON_HEIGHT;
        let height = Window::<S, E>::height_for(inside);
        let width = self.width.min(area.w);
        let x = area.x + (area.w - width) / 2;
        let y = area.y + (area.h - height) / 2;
        return Rect::new(x, y, width, height);
    }

    fn render_components(&mut self, state: &S, body: Rect, painter: &mut Painter) {
        let mut y = body.y;
        for slot in &mut self.slots {
            let height = slot.height();
            let area = Rect::new(body.x, y, body.w, height);
            slot.component().render(state, area, painter);
            y += height + COMPONENT_GAP;
        }
        if let Some(error) = &self.error {
            let row = Rect::new(body.x, y, body.w, CHAR_HEIGHT);
            painter.clipped(row, |painter| {
                painter.text(row.x, row.y, error, Pen::BACKGROUND)
            });
        }
    }

    // the first at the left edge, the last at the right, any others spread out between
    fn render_buttons(&mut self, state: &S, row: Rect, painter: &mut Painter) {
        let widths: Vec<i32> = self
            .buttons
            .iter()
            .map(|button| text_width(&button.label) + 2 * BUTTON_LABEL_PADDING)
            .collect();
        let spare = row.w - widths.iter().sum::<i32>();
        let gaps = (widths.len() as i32 - 1).max(1);
        let mut x = row.x;
        for (DialogButton { button, .. }, width) in self.buttons.iter_mut().zip(&widths) {
            button.render(state, Rect::new(x, row.y, *width, row.h), painter);
            x += width + spare / gaps;
        }
    }
}

impl<S: 'static, E: PointerEvent + TextEvent + 'static> Gadget<S, E> for Dialog<S, E> {
    fn update(&mut self, state: &mut S) {
        for slot in &mut self.slots {
            slot.component().update(state);
        }
        if !self.opened {
            self.opened = true;
            self.changed(state);
        }
    }

    fn handle_event(&mut self, state: &mut S, event: &E) -> bool {
        if self.closed {
            return false;
        }
        let _ = self.handle_captured(state, event)
            || self.handle_key(state, event)
            || self.handle_buttons(state, event)
            || self.handle_components(state, event);
        return true;
    }

    /// Centred in `area`.
    fn render(&mut self, state: &S, area: Rect, painter: &mut Painter) {
        let window = self.window_rect(area);
        self.window.render(state, window, painter);
        let body = Window::<S, E>::content_area(window).inset(PADDING[0], PADDING[1]);
        let (contents, buttons) = body.split_top(body.h - BUTTON_HEIGHT);
        self.render_components(state, contents, painter);
        self.render_buttons(state, buttons, painter);
        for slot in &mut self.slots {
            slot.component().render_overlay(state, painter);
        }
    }
}

/// Text, a line per `\n`.
pub struct Label {
    lines: Vec<String>,
}

impl Label {
    pub fn new(text: &str) -> Self {
        return Self {
            lines: text.lines().map(str::to_owned).collect(),
        };
    }
}

impl<S, E> Gadget<S, E> for Label {
    fn render(&mut self, _state: &S, area: Rect, painter: &mut Painter) {
        painter.clipped(area, |painter| {
            for (line, text) in self.lines.iter().enumerate() {
                let y = area.y + line as i32 * CHAR_HEIGHT;
                painter.text(area.x, y, text, Pen::BACKGROUND);
            }
        });
    }
}

impl<S, E> DialogComponent<S, E> for Label {
    fn height(&self) -> i32 {
        return self.lines.len() as i32 * CHAR_HEIGHT;
    }
}

/// As a field, so [`Dialog::on_change`] can change its text.
impl<S, E> DialogInput<S, E> for Label {
    type Value = String;

    fn value(&self) -> String {
        return self.lines.join("\n");
    }

    fn set_value(&mut self, text: String) {
        self.lines = text.lines().map(str::to_owned).collect();
    }
}

/// A text box, with its label in front of it.
pub struct TextField<S, E> {
    label: String,
    textbox: Textbox<S, E>,
}

impl<S, E> TextField<S, E> {
    pub fn new(label: &str) -> Self {
        return Self {
            label: label.to_owned(),
            textbox: Textbox::new(),
        };
    }

    pub fn text(mut self, text: &str) -> Self {
        self.textbox.set_text(text);
        self.textbox.cursor_to_end();
        return self;
    }

    /// So it can be typed in straight away.
    pub fn focused(mut self) -> Self {
        self.textbox.set_focused(true);
        return self;
    }

    fn label_width(&self) -> i32 {
        if self.label.is_empty() {
            return 0;
        }
        return text_width(&self.label) + CHAR_WIDTH;
    }
}

impl<S, E: PointerEvent + TextEvent> Gadget<S, E> for TextField<S, E> {
    fn handle_event(&mut self, state: &mut S, event: &E) -> bool {
        return self.textbox.handle_event(state, event);
    }

    fn render(&mut self, state: &S, area: Rect, painter: &mut Painter) {
        let (label, field) = area.split_left(self.label_width());
        painter.text(
            label.x,
            text_y(label, CHAR_HEIGHT),
            &self.label,
            Pen::BACKGROUND,
        );
        self.textbox.render(state, field, painter);
    }
}

impl<S, E: PointerEvent + TextEvent> DialogComponent<S, E> for TextField<S, E> {
    fn height(&self) -> i32 {
        return FIELD_HEIGHT;
    }
}

impl<S, E: PointerEvent + TextEvent> DialogInput<S, E> for TextField<S, E> {
    type Value = String;

    fn value(&self) -> String {
        return self.textbox.text().to_owned();
    }

    fn set_value(&mut self, text: String) {
        self.textbox.set_text(text);
        self.textbox.cursor_to_end();
    }
}

/// A box that's ticked or not, with its label after it.
pub struct Checkbox<S, E> {
    label: String,
    checked: bool,
    button: Button<S, E>,
}

impl<S, E> Checkbox<S, E> {
    pub fn new(label: &str) -> Self {
        return Self {
            label: label.to_owned(),
            checked: false,
            button: Button::new(""),
        };
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        return self;
    }
}

impl<S, E: PointerEvent> Gadget<S, E> for Checkbox<S, E> {
    fn handle_event(&mut self, state: &mut S, event: &E) -> bool {
        if !self.button.handle_event(state, event) {
            return false;
        }
        if self.button.clicked().is_some() {
            self.checked = !self.checked;
        }
        return true;
    }

    fn render(&mut self, state: &S, area: Rect, painter: &mut Painter) {
        let [w, h] = CHECKBOX_SIZE;
        let (box_area, label) = area.split_left(w);
        let box_area = Rect::new(box_area.x, box_area.y, w, h);
        self.button.render(state, box_area, painter);
        if self.checked {
            render_tick(painter, box_area);
        }
        let x = label.x + CHAR_WIDTH;
        painter.text(x, text_y(label, CHAR_HEIGHT), &self.label, Pen::BACKGROUND);
    }
}

impl<S, E: PointerEvent> DialogComponent<S, E> for Checkbox<S, E> {
    fn height(&self) -> i32 {
        return FIELD_HEIGHT;
    }
}

impl<S, E: PointerEvent> DialogInput<S, E> for Checkbox<S, E> {
    type Value = bool;

    fn value(&self) -> bool {
        return self.checked;
    }

    fn set_value(&mut self, checked: bool) {
        self.checked = checked;
    }
}

/// Steps through its options on each click, like Intuition's cycle gadget, with its label in
/// front: a left click goes to the next one, a right click back. Its value is the chosen
/// option's index.
pub struct Cycle<S, E> {
    label: String,
    options: Vec<String>,
    chosen: usize,
    button: Button<S, E>,
}

impl<S, E> Cycle<S, E> {
    pub fn new(label: &str, options: &[&str]) -> Self {
        let options: Vec<String> = options.iter().map(|option| option.to_string()).collect();
        let first = options.first().cloned().unwrap_or_default();
        return Self {
            label: label.to_owned(),
            options,
            chosen: 0,
            button: Button::new(first),
        };
    }

    pub fn chosen(mut self, index: usize) -> Self {
        self.choose(index);
        return self;
    }

    fn choose(&mut self, index: usize) {
        let Some(option) = self.options.get(index) else {
            return;
        };
        self.chosen = index;
        self.button.set_text(option.clone());
    }

    fn label_width(&self) -> i32 {
        if self.label.is_empty() {
            return 0;
        }
        return text_width(&self.label) + CHAR_WIDTH;
    }

    // the widest option, so it doesn't change size as it cycles
    fn button_width(&self) -> i32 {
        let widest = self.options.iter().map(|option| text_width(option)).max();
        return widest.unwrap_or(0) + 2 * BUTTON_LABEL_PADDING;
    }
}

impl<S, E: PointerEvent> Gadget<S, E> for Cycle<S, E> {
    fn handle_event(&mut self, state: &mut S, event: &E) -> bool {
        if !self.button.handle_event(state, event) {
            return false;
        }
        let count = self.options.len();
        match self.button.clicked() {
            Some(click) if count > 0 && click.button == MouseButton::Right => {
                self.choose((self.chosen + count - 1) % count)
            }
            Some(_) if count > 0 => self.choose((self.chosen + 1) % count),
            _ => {}
        }
        return true;
    }

    fn render(&mut self, state: &S, area: Rect, painter: &mut Painter) {
        let (label, rest) = area.split_left(self.label_width());
        painter.text(
            label.x,
            text_y(label, CHAR_HEIGHT),
            &self.label,
            Pen::BACKGROUND,
        );
        let button = Rect::new(rest.x, rest.y, self.button_width(), rest.h);
        self.button.render(state, button, painter);
    }
}

impl<S, E: PointerEvent> DialogComponent<S, E> for Cycle<S, E> {
    fn height(&self) -> i32 {
        return FIELD_HEIGHT;
    }
}

impl<S, E: PointerEvent> DialogInput<S, E> for Cycle<S, E> {
    type Value = usize;

    fn value(&self) -> usize {
        return self.chosen;
    }

    fn set_value(&mut self, index: usize) {
        self.choose(index);
    }
}

/// Shows the chosen option, and pressing it opens a list of all of them to pick from, like
/// ReAction's chooser. A click outside the list closes it as it was. Its value is the chosen
/// option's index.
pub struct PopupList<S, E> {
    label: String,
    options: Vec<String>,
    chosen: usize,
    button: Button<S, E>,
    open: bool,
    // from the last render, for clicks
    list_area: Rect,
}

impl<S, E> PopupList<S, E> {
    pub fn new(label: &str, options: &[&str]) -> Self {
        let options: Vec<String> = options.iter().map(|option| option.to_string()).collect();
        let first = options.first().cloned().unwrap_or_default();
        return Self {
            label: label.to_owned(),
            options,
            chosen: 0,
            // lined up with the options in the list
            button: Button::new(first)
                .align(crate::painter::Align::Left)
                .padding_xy(BUTTON_LABEL_PADDING, 1),
            open: false,
            list_area: Rect::default(),
        };
    }

    pub fn chosen(mut self, index: usize) -> Self {
        self.choose(index);
        return self;
    }

    fn choose(&mut self, index: usize) {
        let Some(option) = self.options.get(index) else {
            return;
        };
        self.chosen = index;
        self.button.set_text(option.clone());
    }

    fn label_width(&self) -> i32 {
        if self.label.is_empty() {
            return 0;
        }
        return text_width(&self.label) + CHAR_WIDTH;
    }

    fn button_width(&self) -> i32 {
        let widest = self.options.iter().map(|option| text_width(option)).max();
        return widest.unwrap_or(0) + 2 * BUTTON_LABEL_PADDING + POPUP_ARROW_SPACE;
    }

    // a press in the open list picks the option under it, anywhere else just closes it
    fn handle_open(&mut self, event: MouseEvent) {
        let MouseEvent::Down { pos, .. } = event else {
            return;
        };
        if self.list_area.contains(pos) {
            let row = (pos[1] - self.list_area.y - 1) / POPUP_ROW_HEIGHT;
            self.choose(row.max(0) as usize);
        }
        self.open = false;
    }
}

impl<S, E: PointerEvent> Gadget<S, E> for PopupList<S, E> {
    fn handle_event(&mut self, state: &mut S, event: &E) -> bool {
        if self.open {
            if let Some(mouse) = event.mouse() {
                self.handle_open(mouse);
            }
            return true;
        }
        if !self.button.handle_event(state, event) {
            return false;
        }
        if self.button.clicked().is_some() && !self.options.is_empty() {
            self.open = true;
        }
        return true;
    }

    fn render(&mut self, state: &S, area: Rect, painter: &mut Painter) {
        let (label, rest) = area.split_left(self.label_width());
        painter.text(
            label.x,
            text_y(label, CHAR_HEIGHT),
            &self.label,
            Pen::BACKGROUND,
        );
        let button = Rect::new(rest.x, rest.y, self.button_width(), rest.h);
        self.button.render(state, button, painter);
        render_popup_arrow(painter, button);
    }
}

impl<S, E: PointerEvent> DialogComponent<S, E> for PopupList<S, E> {
    fn height(&self) -> i32 {
        return FIELD_HEIGHT;
    }

    // under the button, as wide as it, the chosen option filled in
    fn render_overlay(&mut self, _state: &S, painter: &mut Painter) {
        if !self.open {
            return;
        }
        let button = self.button.area();
        let height = self.options.len() as i32 * POPUP_ROW_HEIGHT + 2;
        self.list_area = Rect::new(button.x, button.bottom(), button.w, height);
        painter.fill(self.list_area, Pen::FILL);
        let inside = painter.frame(self.list_area, 1);
        for (index, option) in self.options.iter().enumerate() {
            let row = Rect::new(
                inside.x,
                inside.y + index as i32 * POPUP_ROW_HEIGHT,
                inside.w,
                POPUP_ROW_HEIGHT,
            );
            let text = if index == self.chosen {
                painter.fill(row, Pen::SHADOW);
                Pen::SHINE
            } else {
                Pen::BACKGROUND
            };
            let x = row.x + BUTTON_LABEL_PADDING;
            painter.text(x, text_y(row, CHAR_HEIGHT), option, text);
        }
    }

    fn captures_events(&self) -> bool {
        return self.open;
    }
}

impl<S, E: PointerEvent> DialogInput<S, E> for PopupList<S, E> {
    type Value = usize;

    fn value(&self) -> usize {
        return self.chosen;
    }

    fn set_value(&mut self, index: usize) {
        self.choose(index);
    }
}

// a small arrow pointing down, at the button's right
fn render_popup_arrow(painter: &mut Painter, button: Rect) {
    let x = button.right() - POPUP_ARROW_SPACE;
    let y = button.y + (button.h - 3) / 2;
    for row in 0..3 {
        let width = POPUP_ARROW_WIDTH - 2 * row;
        painter.fill(Rect::new(x + row, y + row, width, 1), Pen::BACKGROUND);
    }
}

// a short stroke down to the bottom, then a long one up to the top right, 2 wide for tall pixels
fn render_tick(painter: &mut Painter, area: Rect) {
    const STROKES: [[i32; 2]; 7] = [[7, 4], [9, 5], [11, 6], [13, 5], [15, 4], [17, 3], [19, 2]];
    for [x, y] in STROKES {
        painter.fill(Rect::new(area.x + x, area.y + y, 2, 2), Pen::BACKGROUND);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        event::{MouseButton, MouseEvent},
        palette::{Color, IntuitionColorPalette},
        target::Framebuffer,
    };

    const SCREEN: Rect = Rect::new(0, 0, 640, 256);

    #[derive(Clone, Copy)]
    enum TestEvent {
        Mouse(MouseEvent),
        Typed(TextInput),
    }

    impl PointerEvent for TestEvent {
        fn mouse(&self) -> Option<MouseEvent> {
            return match self {
                TestEvent::Mouse(mouse) => Some(*mouse),
                TestEvent::Typed(_) => None,
            };
        }
    }

    impl TextEvent for TestEvent {
        fn text_input(&self) -> Option<TextInput> {
            return match self {
                TestEvent::Typed(input) => Some(*input),
                TestEvent::Mouse(_) => None,
            };
        }
    }

    // what the callbacks saw
    #[derive(Default)]
    struct Log {
        button: Option<usize>,
        text: Option<String>,
        checked: Option<bool>,
        submits: usize,
        cancelled: bool,
    }

    type TestDialog = Dialog<Log, TestEvent>;
    type TestWindow = Window<Log, TestEvent>;

    fn logged(dialog: TestDialog) -> TestDialog {
        return dialog
            .on_submit(|log: &mut Log, result| {
                log.button = Some(result.button());
                log.submits += 1;
            })
            .on_cancel(|log: &mut Log| log.cancelled = true);
    }

    fn render(dialog: &mut TestDialog, log: &Log) {
        let palette =
            IntuitionColorPalette::new([Color(0x000), Color(0xFFF), Color(0x666), Color(0xAAA)]);
        let mut screen = Framebuffer::new(SCREEN.w, SCREEN.h);
        dialog.render(log, SCREEN, &mut Painter::new(&mut screen, &palette));
    }

    fn click(dialog: &mut TestDialog, log: &mut Log, pos: [i32; 2]) {
        let press = TestEvent::Mouse(MouseEvent::Down {
            pos,
            button: MouseButton::Left,
        });
        assert!(dialog.handle_event(log, &press));
    }

    fn click_button(dialog: &mut TestDialog, log: &mut Log, index: usize) {
        let area = dialog.buttons[index].button.area();
        click(dialog, log, [area.x + area.w / 2, area.y + area.h / 2]);
    }

    fn key(dialog: &mut TestDialog, log: &mut Log, key: EditKey) {
        let input = TextInput::Key {
            key,
            shift: false,
            ctrl: false,
        };
        dialog.handle_event(log, &TestEvent::Typed(input));
    }

    fn type_text(dialog: &mut TestDialog, log: &mut Log, text: &str) {
        for c in text.chars() {
            dialog.handle_event(log, &TestEvent::Typed(TextInput::Char(c)));
        }
    }

    #[test]
    fn ok_submits_and_cancel_cancels() {
        let mut log = Log::default();
        let mut dialog = logged(Dialog::confirm("Sure?", "Really?"));
        render(&mut dialog, &log);
        click_button(&mut dialog, &mut log, 0);
        assert!(dialog.is_closed());
        assert_eq!(log.button, Some(0));

        let mut log = Log::default();
        let mut dialog = logged(Dialog::confirm("Sure?", "Really?"));
        render(&mut dialog, &log);
        click_button(&mut dialog, &mut log, 1);
        assert!(log.cancelled && log.button.is_none());
    }

    #[test]
    fn middle_button_submits_with_its_index() {
        let mut log = Log::default();
        let dialog = Dialog::new("Save?")
            .submit_button("Save")
            .submit_button("Don't save")
            .cancel_button("Cancel");
        let mut dialog = logged(dialog);
        render(&mut dialog, &log);
        click_button(&mut dialog, &mut log, 1);
        assert_eq!(log.button, Some(1));
    }

    #[test]
    fn only_button_submits() {
        let mut log = Log::default();
        let mut dialog = logged(Dialog::message("Done", "All saved."));
        render(&mut dialog, &log);
        click_button(&mut dialog, &mut log, 0);
        assert!(log.button.is_some() && !log.cancelled);
    }

    #[test]
    fn enter_submits_escape_cancels() {
        let mut log = Log::default();
        let mut dialog = logged(Dialog::confirm("Sure?", "Really?"));
        key(&mut dialog, &mut log, EditKey::Enter);
        assert!(log.button.is_some());

        let mut log = Log::default();
        let mut dialog = logged(Dialog::confirm("Sure?", "Really?"));
        key(&mut dialog, &mut log, EditKey::Escape);
        assert!(log.cancelled);
    }

    #[test]
    fn close_gadget_cancels() {
        let mut log = Log::default();
        let mut dialog = logged(Dialog::confirm("Sure?", "Really?"));
        render(&mut dialog, &log);
        let window = dialog.window_rect(SCREEN);
        click(&mut dialog, &mut log, [window.x + 8, window.y + 5]);
        assert!(log.cancelled);
    }

    #[test]
    fn fields_come_back_typed() {
        let mut log = Log::default();
        let mut dialog = Dialog::new("Instrument").submit_button("OK");
        let name = dialog.add_field(TextField::new("Name:").text("bass").focused());
        let looped = dialog.add_field(Checkbox::new("Loop"));
        let mut dialog = dialog.on_submit(move |log: &mut Log, result| {
            log.text = Some(result.get(name).clone());
            log.checked = Some(*result.get(looped));
        });
        render(&mut dialog, &log);
        type_text(&mut dialog, &mut log, " 2");
        let content = TestWindow::content_area(dialog.window_rect(SCREEN));
        let checkbox_y = content.y + PADDING[1] + FIELD_HEIGHT + COMPONENT_GAP + 5;
        click(
            &mut dialog,
            &mut log,
            [content.x + PADDING[0] + 5, checkbox_y],
        );
        key(&mut dialog, &mut log, EditKey::Enter);
        assert_eq!(log.text.as_deref(), Some("bass 2"));
        assert_eq!(log.checked, Some(true));
    }

    #[test]
    fn text_input_gives_its_field() {
        let mut log = Log::default();
        let (dialog, name) = Dialog::text_input("Rename", "Name:", "old");
        let mut dialog = dialog.on_submit(move |log: &mut Log, result| {
            log.text = Some(result.get(name).clone());
        });
        type_text(&mut dialog, &mut log, "er");
        key(&mut dialog, &mut log, EditKey::Enter);
        assert_eq!(log.text.as_deref(), Some("older"));
    }

    #[test]
    fn refused_submit_stays_open_with_the_message() {
        let mut log = Log::default();
        let (dialog, name) = Dialog::text_input("Rename", "Name:", "");
        let mut dialog = dialog.on_submit_checked(move |log: &mut Log, result| {
            if result.get(name).is_empty() {
                return Err("Give it a name".to_owned());
            }
            log.submits += 1;
            return Ok(());
        });
        let height_before = dialog.window_rect(SCREEN).h;
        key(&mut dialog, &mut log, EditKey::Enter);
        assert!(!dialog.is_closed());
        assert_eq!(dialog.error.as_deref(), Some("Give it a name"));
        assert_eq!(
            dialog.window_rect(SCREEN).h,
            height_before + COMPONENT_GAP + CHAR_HEIGHT
        );
        type_text(&mut dialog, &mut log, "kick");
        key(&mut dialog, &mut log, EditKey::Enter);
        assert!(dialog.is_closed());
        assert_eq!(log.submits, 1);
    }

    #[test]
    fn takes_every_event_until_closed() {
        let mut log = Log::default();
        let mut dialog = logged(Dialog::confirm("Sure?", "Really?"));
        render(&mut dialog, &log);
        click(&mut dialog, &mut log, [1, 1]);
        assert!(!dialog.is_closed());
        key(&mut dialog, &mut log, EditKey::Escape);
        let press = TestEvent::Mouse(MouseEvent::Down {
            pos: [1, 1],
            button: MouseButton::Left,
        });
        assert!(!dialog.handle_event(&mut log, &press));
    }

    #[test]
    fn not_cancellable() {
        let mut log = Log::default();
        let dialog = Dialog::new("Format")
            .submit_button("MMD0")
            .submit_button("MMD1")
            .cancellable(false);
        let mut dialog = logged(dialog);
        render(&mut dialog, &log);
        key(&mut dialog, &mut log, EditKey::Escape);
        let window = dialog.window_rect(SCREEN);
        click(&mut dialog, &mut log, [window.x + 8, window.y + 5]);
        assert!(!dialog.is_closed() && !log.cancelled);
        click_button(&mut dialog, &mut log, 1);
        assert_eq!(log.button, Some(1));
        assert!(!log.cancelled);
    }

    #[test]
    fn buttons_do_what_they_were_made_as() {
        let mut log = Log::default();
        let dialog = Dialog::new("Order")
            .cancel_button("Cancel")
            .submit_button("OK");
        let mut dialog = logged(dialog);
        render(&mut dialog, &log);
        click_button(&mut dialog, &mut log, 1);
        assert_eq!(log.button, Some(1));

        let mut log = Log::default();
        let dialog = Dialog::new("Order")
            .cancel_button("Cancel")
            .submit_button("OK");
        let mut dialog = logged(dialog);
        render(&mut dialog, &log);
        click_button(&mut dialog, &mut log, 0);
        assert!(log.cancelled && log.button.is_none());
    }

    #[test]
    fn enter_presses_the_first_submit_button() {
        let mut log = Log::default();
        let dialog = Dialog::new("Order")
            .cancel_button("Cancel")
            .submit_button("OK");
        let mut dialog = logged(dialog);
        key(&mut dialog, &mut log, EditKey::Enter);
        assert_eq!(log.button, Some(1));

        let mut log = Log::default();
        let mut dialog = logged(Dialog::new("Nothing to submit").cancel_button("Close"));
        key(&mut dialog, &mut log, EditKey::Enter);
        assert!(!dialog.is_closed() && log.button.is_none());
    }

    #[test]
    fn cycle_steps_through_and_wraps() {
        let mut log = Log::default();
        let mut dialog = Dialog::new("Channel").submit_button("OK");
        let channel =
            dialog.add_field(Cycle::new("Channel:", &["Left", "Right", "Mixed"]).chosen(2));
        let chosen = std::rc::Rc::new(std::cell::Cell::new(None));
        let seen = chosen.clone();
        let mut dialog =
            dialog.on_submit(move |_: &mut Log, result| seen.set(Some(*result.get(channel))));
        render(&mut dialog, &log);
        let content = TestWindow::content_area(dialog.window_rect(SCREEN));
        let button_x = content.x + PADDING[0] + text_width("Channel:") + CHAR_WIDTH + 5;
        click(
            &mut dialog,
            &mut log,
            [button_x, content.y + PADDING[1] + 5],
        );
        key(&mut dialog, &mut log, EditKey::Enter);
        assert_eq!(chosen.get(), Some(0));
    }

    fn right_click(dialog: &mut TestDialog, log: &mut Log, pos: [i32; 2]) {
        let press = TestEvent::Mouse(MouseEvent::Down {
            pos,
            button: MouseButton::Right,
        });
        assert!(dialog.handle_event(log, &press));
    }

    // the first component's row, past its label
    fn first_row(dialog: &TestDialog, label: &str) -> [i32; 2] {
        let content = TestWindow::content_area(dialog.window_rect(SCREEN));
        let x = content.x + PADDING[0] + text_width(label) + CHAR_WIDTH + 5;
        return [x, content.y + PADDING[1] + 5];
    }

    #[test]
    fn cycle_right_click_goes_back() {
        let mut log = Log::default();
        let mut dialog = Dialog::new("Channel").submit_button("OK");
        let channel = dialog.add_field(Cycle::new("Channel:", &["Left", "Right", "Mixed"]));
        let mut dialog = dialog.on_submit(move |log: &mut Log, result| {
            log.button = Some(*result.get(channel));
        });
        render(&mut dialog, &log);
        let at = first_row(&dialog, "Channel:");
        right_click(&mut dialog, &mut log, at);
        key(&mut dialog, &mut log, EditKey::Enter);
        assert_eq!(log.button, Some(2));
    }

    fn popup_dialog() -> (TestDialog, Field<usize>) {
        let mut dialog = Dialog::new("Format").submit_button("OK");
        let format = dialog.add_field(PopupList::new("Format:", &["MMD0", "MMD1", "MMD2"]));
        return (dialog, format);
    }

    #[test]
    fn popup_list_picks_an_option() {
        let mut log = Log::default();
        let (dialog, format) = popup_dialog();
        let mut dialog = dialog.on_submit(move |log: &mut Log, result| {
            log.button = Some(*result.get(format));
        });
        render(&mut dialog, &log);
        let at = first_row(&dialog, "Format:");
        click(&mut dialog, &mut log, at);
        render(&mut dialog, &log);
        // the list opens under the button, a row per option
        let list = Rect::new(0, at[1] - 5 + FIELD_HEIGHT, 0, 0);
        let third = [at[0], list.y + 1 + 2 * POPUP_ROW_HEIGHT + 3];
        click(&mut dialog, &mut log, third);
        key(&mut dialog, &mut log, EditKey::Enter);
        assert_eq!(log.button, Some(2));
    }

    #[test]
    fn popup_list_takes_events_while_open() {
        let mut log = Log::default();
        let (dialog, _) = popup_dialog();
        let mut dialog = logged(dialog);
        render(&mut dialog, &log);
        let at = first_row(&dialog, "Format:");
        click(&mut dialog, &mut log, at);
        key(&mut dialog, &mut log, EditKey::Enter);
        assert!(!dialog.is_closed() && log.button.is_none());
        // closes on a click outside it, and then Enter submits again
        click(&mut dialog, &mut log, [1, 1]);
        key(&mut dialog, &mut log, EditKey::Enter);
        assert_eq!(log.button, Some(0));
    }

    #[test]
    fn label_follows_another_field() {
        let mut log = Log::default();
        let mut dialog = Dialog::new("Channel").submit_button("OK");
        let preview = dialog.add_field(Label::new(""));
        let channel = dialog.add_field(Cycle::new("Channel:", &["Left", "Right", "Mixed"]));
        let mut dialog = dialog.on_change(move |_: &mut Log, fields| {
            let names = ["Left", "Right", "Mixed"];
            fields.set(preview, format!("Using {}", names[fields.get(channel)]));
        });
        dialog.update(&mut log);
        let shown = |dialog: &mut TestDialog| dialog.result(0).get(preview).clone();
        assert_eq!(shown(&mut dialog), "Using Left");
        render(&mut dialog, &log);
        let content = TestWindow::content_area(dialog.window_rect(SCREEN));
        let cycle_y = content.y + PADDING[1] + CHAR_HEIGHT + COMPONENT_GAP + 5;
        let x = content.x + PADDING[0] + text_width("Channel:") + CHAR_WIDTH + 5;
        click(&mut dialog, &mut log, [x, cycle_y]);
        assert_eq!(shown(&mut dialog), "Using Right");
    }

    #[test]
    fn fields_can_be_set() {
        let mut log = Log::default();
        let mut dialog: TestDialog = Dialog::new("Sync").submit_button("OK");
        let name = dialog.add_field(TextField::new("Name:"));
        let looped = dialog.add_field(Checkbox::new("Loop"));
        let mut dialog = dialog.on_change(move |_: &mut Log, fields| {
            fields.set(name, "kick".to_owned());
            fields.set(looped, true);
        });
        dialog.update(&mut log);
        let result = dialog.result(0);
        assert_eq!(result.get(name), "kick");
        assert!(*result.get(looped));
    }

    #[test]
    fn submits_once() {
        let mut log = Log::default();
        let mut dialog = logged(Dialog::confirm("Sure?", "Really?"));
        key(&mut dialog, &mut log, EditKey::Enter);
        key(&mut dialog, &mut log, EditKey::Enter);
        assert_eq!(log.submits, 1);
    }

    #[test]
    fn centred_and_sized_to_fit() {
        let dialog: TestDialog = Dialog::confirm("Sure?", "Line one\nLine two");
        let window = dialog.window_rect(SCREEN);
        let inside = 2 * PADDING[1] + 2 * CHAR_HEIGHT + BUTTONS_GAP + BUTTON_HEIGHT;
        let expected_height = TestWindow::height_for(inside);
        assert_eq!(window.h, expected_height);
        assert_eq!(window.w, DEFAULT_WIDTH);
        assert_eq!(window.x, (640 - DEFAULT_WIDTH) / 2);
        assert_eq!(window.y, (256 - expected_height) / 2);
    }
}
