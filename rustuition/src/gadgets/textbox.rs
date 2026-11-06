use std::{
    marker::PhantomData,
    rc::Rc,
    time::{Duration, Instant},
};

use crate::{
    event::{EditKey, MouseButton, MouseEvent, PointerEvent, TextEvent, TextInput},
    font::Font,
    gadget::Gadget,
    painter::{Painter, Pen, inside_frame},
    rect::Rect,
};

const DOUBLE_CLICK: Duration = Duration::from_millis(500);

/// What a text box calls with its new text when typing changes it.
pub trait TextHandler<S>: Fn(&mut S, &str) + 'static {}

impl<S, F: Fn(&mut S, &str) + 'static> TextHandler<S> for F {}

/// A text field, Intuition's string gadget. Click it to type in it, drag across it to select.
// clones share the handler. `S` is the app's state, `E` the events it's sent
pub struct Textbox<S, E = MouseEvent> {
    text: String,
    // in chars. the cursor sits on the char it's before
    cursor: usize,
    // the other end of the selection, if there is one
    anchor: Option<usize>,
    focused: bool,
    dragging: bool,
    last_click: Option<(Instant, usize)>,
    // first char shown, when the text is wider than the box
    scroll: usize,
    max_length: Option<usize>,
    select_on_focus: bool,
    font: Font,
    border: i32,
    padding: i32,
    fill: Pen,
    text_pen: Pen,
    cursor_pen: Pen,
    selection_pen: Pen,
    on_update: Option<Rc<dyn TextHandler<S>>>,
    enabled: bool,
    // from the last render
    area: Rect,
    events: PhantomData<fn(&E)>,
}

impl<S, E> Textbox<S, E> {
    /// An empty text box in Topaz, light text on the background pen.
    pub fn new() -> Self {
        return Self {
            text: String::new(),
            cursor: 0,
            anchor: None,
            focused: false,
            dragging: false,
            last_click: None,
            scroll: 0,
            max_length: None,
            select_on_focus: false,
            font: Font::TOPAZ,
            border: 1,
            padding: 2,
            fill: Pen::BACKGROUND,
            text_pen: Pen::SHINE,
            cursor_pen: Pen::FILL,
            selection_pen: Pen::SHADOW,
            on_update: None,
            enabled: true,
            area: Rect::default(),
            events: PhantomData,
        };
    }

    /// Sets what's called when typing changes the text.
    pub fn on_update(mut self, handler: impl Fn(&mut S, &str) + 'static) -> Self {
        self.on_update = Some(Rc::new(handler));
        return self;
    }

    /// Stops typing past `length` chars.
    pub fn max_length(mut self, length: usize) -> Self {
        self.max_length = Some(length);
        return self;
    }

    /// Selects the whole text when a click focuses it, so typing replaces it. For number boxes.
    pub fn select_on_focus(mut self) -> Self {
        self.select_on_focus = true;
        return self;
    }

    /// Sets the font.
    pub fn font(mut self, font: Font) -> Self {
        self.font = font;
        return self;
    }

    /// Sets the pens for the box, the text, the cursor and the selection.
    pub fn pens(mut self, fill: Pen, text: Pen, cursor: Pen, selection: Pen) -> Self {
        self.fill = fill;
        self.text_pen = text;
        self.cursor_pen = cursor;
        self.selection_pen = selection;
        return self;
    }

    /// Sets the space between the frame and the text, across.
    pub fn padding(mut self, padding: i32) -> Self {
        self.padding = padding;
        return self;
    }

    /// The text in it.
    pub fn text(&self) -> &str {
        return &self.text;
    }

    /// Replaces the text without calling the handler, keeping the cursor where it fits.
    pub fn set_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        if text == self.text {
            return;
        }
        self.text = text;
        self.cursor = self.cursor.min(self.len());
        self.anchor = None;
    }

    /// Whether typing goes into it.
    pub fn cursor_to_end(&mut self) {
        self.cursor = self.len();
        self.anchor = None;
    }

    /// A disabled text box is ghosted, can't be focused and doesn't take typing.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.set_focused(false);
        }
    }

    pub fn is_enabled(&self) -> bool {
        return self.enabled;
    }

    pub fn is_focused(&self) -> bool {
        return self.focused;
    }

    /// Starts or stops typing going into it.
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        self.dragging &= focused;
        if !focused {
            self.anchor = None;
        }
    }

    /// The selected chars, as a range of char indexes.
    pub fn selection(&self) -> Option<(usize, usize)> {
        let anchor = self.anchor?;
        return (anchor != self.cursor)
            .then_some((anchor.min(self.cursor), anchor.max(self.cursor)));
    }

    fn len(&self) -> usize {
        return self.text.chars().count();
    }

    fn byte(&self, index: usize) -> usize {
        return self
            .text
            .char_indices()
            .nth(index)
            .map_or(self.text.len(), |(byte, _)| byte);
    }

    fn move_cursor(&mut self, to: usize, select: bool) {
        if select {
            self.anchor.get_or_insert(self.cursor);
        } else {
            self.anchor = None;
        }
        self.cursor = to.min(self.len());
    }

    // true if anything was selected
    fn delete_selection(&mut self) -> bool {
        let Some((start, end)) = self.selection() else {
            return false;
        };
        let range = self.byte(start)..self.byte(end);
        self.text.replace_range(range, "");
        self.cursor = start;
        self.anchor = None;
        return true;
    }

    fn delete(&mut self, from: usize, to: usize) {
        let (start, end) = (from.min(to), from.max(to).min(self.len()));
        let range = self.byte(start)..self.byte(end);
        self.text.replace_range(range, "");
        self.cursor = start;
    }

    fn insert(&mut self, c: char) -> bool {
        let replaced = self.delete_selection();
        self.anchor = None;
        if self.max_length.is_some_and(|max| self.len() >= max) {
            return replaced;
        }
        let byte = self.byte(self.cursor);
        self.text.insert(byte, c);
        self.cursor += 1;
        return true;
    }

    fn chars(&self) -> Vec<char> {
        return self.text.chars().collect();
    }

    // past the spaces then the word, either way
    fn word_from(&self, index: usize, forward: bool) -> usize {
        let chars = self.chars();
        let mut index = index.min(chars.len());
        let at = |index: usize| {
            if forward {
                chars.get(index).copied()
            } else {
                index.checked_sub(1).map(|before| chars[before])
            }
        };
        let step = |index: &mut usize| {
            if forward {
                *index += 1;
            } else {
                *index -= 1;
            }
        };
        while at(index).is_some_and(char::is_whitespace) {
            step(&mut index);
        }
        while at(index).is_some_and(|c| !c.is_whitespace()) {
            step(&mut index);
        }
        return index;
    }

    fn select_word(&mut self, index: usize) {
        let chars = self.chars();
        if chars.get(index).is_none_or(|c| c.is_whitespace()) {
            return;
        }
        let is_word = |i: usize| chars.get(i).is_some_and(|c| !c.is_whitespace());
        let mut start = index;
        while start > 0 && is_word(start - 1) {
            start -= 1;
        }
        let mut end = index;
        while is_word(end) {
            end += 1;
        }
        self.anchor = Some(start);
        self.cursor = end;
    }

    // true if the text changed
    fn edit(&mut self, key: EditKey, shift: bool, ctrl: bool) -> bool {
        let len = self.len();
        match key {
            EditKey::Left => {
                let to = match (self.selection(), shift || ctrl) {
                    (Some((start, _)), false) => start,
                    _ if ctrl => self.word_from(self.cursor, false),
                    _ => self.cursor.saturating_sub(1),
                };
                self.move_cursor(to, shift);
            }
            EditKey::Right => {
                let to = match (self.selection(), shift || ctrl) {
                    (Some((_, end)), false) => end,
                    _ if ctrl => self.word_from(self.cursor, true),
                    _ => self.cursor + 1,
                };
                self.move_cursor(to, shift);
            }
            EditKey::Home => self.move_cursor(0, shift),
            EditKey::End => self.move_cursor(len, shift),
            EditKey::SelectAll => {
                self.anchor = Some(0);
                self.cursor = len;
            }
            EditKey::Backspace => {
                if self.delete_selection() {
                    return true;
                }
                let from = if ctrl {
                    self.word_from(self.cursor, false)
                } else {
                    self.cursor.saturating_sub(1)
                };
                let changed = from < self.cursor;
                self.delete(from, self.cursor);
                return changed;
            }
            EditKey::Delete => {
                if self.delete_selection() {
                    return true;
                }
                let to = if ctrl {
                    self.word_from(self.cursor, true)
                } else {
                    self.cursor + 1
                };
                let changed = self.cursor < len;
                self.delete(self.cursor, to);
                return changed;
            }
            EditKey::Enter | EditKey::Escape => self.set_focused(false),
        }
        return false;
    }

    fn inner(&self, area: Rect) -> Rect {
        return inside_frame(area, self.border).inset(self.padding, 0);
    }

    // the char a pixel is on, or past the end
    fn index_at(&self, x: i32) -> usize {
        let mut left = self.inner(self.area).x;
        for (index, c) in self.text.chars().enumerate().skip(self.scroll) {
            left += self.font.advance(c);
            if x < left {
                return index;
            }
        }
        return self.len();
    }

    fn click(&mut self, x: i32) {
        let index = self.index_at(x);
        let now = Instant::now();
        let double = self.last_click.is_some_and(|(at, clicked)| {
            clicked == index && now.duration_since(at) <= DOUBLE_CLICK
        });
        if self.select_on_focus && !self.focused {
            self.focused = true;
            self.anchor = Some(0);
            self.cursor = self.len();
            self.last_click = None;
            return;
        }
        self.focused = true;
        if double {
            self.last_click = None;
            self.select_word(index);
            return;
        }
        self.last_click = Some((now, index));
        self.dragging = true;
        self.cursor = index;
        self.anchor = Some(index);
    }

    fn type_text(&mut self, state: &mut S, input: TextInput) -> bool {
        let changed = match input {
            TextInput::Char(c) if !c.is_control() => self.insert(c),
            TextInput::Char(_) => false,
            TextInput::Key { key, shift, ctrl } => self.edit(key, shift, ctrl),
        };
        if let Some(handler) = self.on_update.as_ref().filter(|_| changed) {
            handler(state, &self.text);
        }
        return true;
    }

    // keeps the cursor inside the box
    fn scroll_to_cursor(&mut self, width: i32) {
        self.scroll = self.scroll.min(self.cursor);
        let chars = self.chars();
        // the cursor after the last char gets no room, so only text wider than the box scrolls
        let cursor_end = |scroll: usize| -> i32 {
            let shown = chars.iter().skip(scroll).take(self.cursor + 1 - scroll);
            shown.map(|&c| self.font.advance(c)).sum::<i32>()
        };
        while self.scroll < self.cursor && cursor_end(self.scroll) > width {
            self.scroll += 1;
        }
    }
}

impl<S, E> Default for Textbox<S, E> {
    fn default() -> Self {
        return Self::new();
    }
}

// derive would want S: Clone
impl<S, E> Clone for Textbox<S, E> {
    fn clone(&self) -> Self {
        return Self {
            text: self.text.clone(),
            on_update: self.on_update.clone(),
            ..*self
        };
    }
}

impl<S, E: PointerEvent + TextEvent> Gadget<S, E> for Textbox<S, E> {
    fn handle_event(&mut self, state: &mut S, event: &E) -> bool {
        if !self.enabled {
            return false;
        }
        if let Some(input) = event.text_input() {
            return self.focused && self.type_text(state, input);
        }
        match event.mouse() {
            Some(MouseEvent::Down {
                pos,
                button: MouseButton::Left,
            }) if self.area.contains(pos) => {
                self.click(pos.x);
                return true;
            }
            Some(MouseEvent::Down { .. }) => {
                self.set_focused(false);
                return false;
            }
            Some(MouseEvent::Move { pos }) if self.dragging => {
                self.cursor = self.index_at(pos.x);
                return false;
            }
            Some(MouseEvent::Up {
                button: MouseButton::Left,
                ..
            }) if self.dragging => {
                self.dragging = false;
                if self.selection().is_none() {
                    self.anchor = None;
                }
                return true;
            }
            _ => return false,
        }
    }

    fn render(&mut self, _state: &S, area: Rect, painter: &mut Painter) {
        self.area = area;
        painter.fill(area, self.fill);
        painter.frame(area, self.border);
        let inner = self.inner(area);
        // scrolled for the cursor while typing, so a short text could stay half hidden after
        if self.focused {
            self.scroll_to_cursor(inner.w);
        } else {
            self.scroll = 0;
        }
        // a row lower when it can't be centred exactly, a font's bottom row is room for descenders
        let y = inner.y + ((inner.h - self.font.height() + 1) / 2).max(0);
        let selection = self.selection().filter(|_| self.focused);
        let cursor = (self.focused && selection.is_none()).then_some(self.cursor);
        painter.clipped(inner, |painter| {
            let mut x = inner.x;
            let shown = self.text.chars().chain([' ']).enumerate().skip(self.scroll);
            for (index, c) in shown {
                let width = self.font.advance(c);
                let cell = Rect::new(x, inner.y, width, inner.h);
                let selected = selection.is_some_and(|(start, end)| (start..end).contains(&index));
                let pen = if cursor == Some(index) {
                    painter.fill(cell, self.cursor_pen);
                    self.fill
                } else {
                    if selected {
                        painter.fill(cell, self.selection_pen);
                    }
                    self.text_pen
                };
                painter.text_in_font(x, y, &c.to_string(), pen, self.font);
                x += width;
                if x >= inner.right() {
                    break;
                }
            }
        });
        if !self.enabled {
            // black dots wouldn't show on a black box
            let dots = if self.fill == Pen::BACKGROUND {
                Pen::FILL
            } else {
                Pen::BACKGROUND
            };
            painter.ghost(area, dots);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use super::*;
    use crate::{
        palette::{Color, IntuitionColorPalette},
        rect::Point,
        target::Framebuffer,
    };

    const AREA: Rect = Rect::new(0, 0, 100, 10);

    fn textbox(text: &str) -> Textbox<()> {
        let mut textbox = Textbox::new();
        textbox.set_text(text);
        textbox.area = AREA;
        return textbox;
    }

    fn key(key: EditKey) -> TextInput {
        return TextInput::Key {
            key,
            shift: false,
            ctrl: false,
        };
    }

    fn shift(key: EditKey) -> TextInput {
        return TextInput::Key {
            key,
            shift: true,
            ctrl: false,
        };
    }

    fn ctrl(key: EditKey) -> TextInput {
        return TextInput::Key {
            key,
            shift: false,
            ctrl: true,
        };
    }

    fn typed(textbox: &mut Textbox<()>, inputs: &[TextInput]) {
        for &input in inputs {
            textbox.type_text(&mut (), input);
        }
    }

    // topaz is 8 wide, the text starts 4 in: the frame's 2 and the padding's 2
    fn char_x(index: i32) -> i32 {
        return 4 + index * 8 + 1;
    }

    #[test]
    fn disabled_ignores_clicks_and_typing() {
        let mut textbox = textbox("hello");
        textbox.set_focused(true);
        textbox.set_enabled(false);
        assert!(!textbox.is_focused());
        let down = MouseEvent::Down {
            pos: Point::new(char_x(2), 5),
            button: MouseButton::Left,
        };
        assert!(!textbox.handle_event(&mut (), &down));
        assert!(!textbox.is_focused());
    }

    #[test]
    fn click_focuses_and_places_cursor() {
        let mut textbox = textbox("hello");
        let down = MouseEvent::Down {
            pos: Point::new(char_x(2), 5),
            button: MouseButton::Left,
        };
        assert!(textbox.handle_event(&mut (), &down));
        assert!(textbox.is_focused());
        assert_eq!(textbox.cursor, 2);
        let past_end = MouseEvent::Down {
            pos: Point::new(90, 5),
            button: MouseButton::Left,
        };
        textbox.handle_event(&mut (), &past_end);
        assert_eq!(textbox.cursor, 5);
    }

    #[test]
    fn select_on_focus_selects_everything_and_typing_replaces_it() {
        let mut textbox = textbox("64").select_on_focus();
        let down = MouseEvent::Down {
            pos: Point::new(char_x(0), 5),
            button: MouseButton::Left,
        };
        textbox.handle_event(&mut (), &down);
        assert_eq!(textbox.selection(), Some((0, 2)));
        typed(&mut textbox, &[TextInput::Char('9')]);
        assert_eq!(textbox.text(), "9");
        // once it's focused, a click places the cursor as usual
        textbox.handle_event(&mut (), &down);
        assert_eq!((textbox.cursor, textbox.selection()), (0, None));
    }

    #[test]
    fn click_elsewhere_unfocuses() {
        let mut textbox = textbox("hello");
        textbox.set_focused(true);
        let away = MouseEvent::Down {
            pos: Point::new(200, 5),
            button: MouseButton::Left,
        };
        assert!(!textbox.handle_event(&mut (), &away));
        assert!(!textbox.is_focused());
    }

    #[test]
    fn drag_selects() {
        let mut textbox = textbox("hello world");
        let down = MouseEvent::Down {
            pos: Point::new(char_x(1), 5),
            button: MouseButton::Left,
        };
        textbox.handle_event(&mut (), &down);
        textbox.handle_event(
            &mut (),
            &MouseEvent::Move {
                pos: Point::new(char_x(4), 5),
            },
        );
        textbox.handle_event(
            &mut (),
            &MouseEvent::Up {
                pos: Point::new(char_x(4), 5),
                button: MouseButton::Left,
            },
        );
        assert_eq!(textbox.selection(), Some((1, 4)));
        typed(&mut textbox, &[TextInput::Char('X')]);
        assert_eq!(textbox.text(), "hXo world");
    }

    #[test]
    fn typing_while_held_keeps_every_char() {
        let mut textbox = textbox("");
        let down = MouseEvent::Down {
            pos: Point::new(char_x(0), 5),
            button: MouseButton::Left,
        };
        textbox.handle_event(&mut (), &down);
        typed(&mut textbox, &[TextInput::Char('a'), TextInput::Char('b')]);
        assert_eq!(textbox.text(), "ab");
    }

    #[test]
    fn typing_needs_focus() {
        let mut textbox: Textbox<(), FakeEvent> = Textbox::new();
        textbox.set_text("ab");
        assert!(!textbox.handle_event(&mut (), &FakeEvent(TextInput::Char('c'))));
        assert_eq!(textbox.text(), "ab");
    }

    #[test]
    fn editing_keys() {
        let mut textbox = textbox("one two");
        textbox.set_focused(true);
        textbox.cursor = 7;
        typed(
            &mut textbox,
            &[key(EditKey::Backspace), TextInput::Char('!')],
        );
        assert_eq!(textbox.text(), "one tw!");
        typed(&mut textbox, &[key(EditKey::Home), key(EditKey::Delete)]);
        assert_eq!(textbox.text(), "ne tw!");
        typed(&mut textbox, &[key(EditKey::End), ctrl(EditKey::Backspace)]);
        assert_eq!(textbox.text(), "ne ");
        typed(&mut textbox, &[ctrl(EditKey::Left), ctrl(EditKey::Delete)]);
        assert_eq!(textbox.text(), " ");
    }

    #[test]
    fn shift_and_select_all() {
        let mut textbox = textbox("abcdef");
        textbox.set_focused(true);
        textbox.cursor = 2;
        typed(
            &mut textbox,
            &[shift(EditKey::Right), shift(EditKey::Right)],
        );
        assert_eq!(textbox.selection(), Some((2, 4)));
        typed(&mut textbox, &[key(EditKey::Left)]);
        assert_eq!((textbox.cursor, textbox.selection()), (2, None));
        typed(&mut textbox, &[shift(EditKey::End)]);
        assert_eq!(textbox.selection(), Some((2, 6)));
        typed(
            &mut textbox,
            &[key(EditKey::SelectAll), key(EditKey::Backspace)],
        );
        assert_eq!(textbox.text(), "");
    }

    #[test]
    fn double_click_selects_word() {
        let mut textbox = textbox("one two three");
        let down = MouseEvent::Down {
            pos: Point::new(char_x(5), 5),
            button: MouseButton::Left,
        };
        let up = MouseEvent::Up {
            pos: Point::new(char_x(5), 5),
            button: MouseButton::Left,
        };
        for event in [down, up, down] {
            textbox.handle_event(&mut (), &event);
        }
        assert_eq!(textbox.selection(), Some((4, 7)));
    }

    #[test]
    fn handler_gets_typed_text_only() {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let told = seen.clone();
        let mut textbox: Textbox<()> =
            Textbox::new().on_update(move |_, text| told.borrow_mut().push(text.to_owned()));
        textbox.set_text("a");
        textbox.set_focused(true);
        textbox.cursor = 1;
        typed(
            &mut textbox,
            &[
                TextInput::Char('b'),
                key(EditKey::Left),
                key(EditKey::Enter),
            ],
        );
        assert_eq!(*seen.borrow(), ["ab"]);
        assert!(!textbox.is_focused());
    }

    #[test]
    fn max_length() {
        let mut textbox = textbox("abc").max_length(3);
        textbox.set_focused(true);
        textbox.cursor = 3;
        typed(&mut textbox, &[TextInput::Char('d')]);
        assert_eq!(textbox.text(), "abc");
        typed(&mut textbox, &[shift(EditKey::Left), TextInput::Char('d')]);
        assert_eq!(textbox.text(), "abd");
    }

    #[test]
    fn multibyte_text() {
        let mut textbox = textbox("héllo");
        textbox.set_focused(true);
        textbox.cursor = 2;
        typed(
            &mut textbox,
            &[key(EditKey::Backspace), TextInput::Char('ë')],
        );
        assert_eq!(textbox.text(), "hëllo");
    }

    #[test]
    fn scrolls_to_cursor() {
        let mut textbox = textbox("a fairly long song name");
        textbox.set_focused(true);
        textbox.cursor = textbox.len();
        textbox.scroll_to_cursor(40);
        assert_eq!(textbox.scroll, textbox.len() - 5);

        textbox.cursor = 0;
        textbox.scroll_to_cursor(40);
        assert_eq!(textbox.scroll, 0);
    }

    #[test]
    fn a_full_box_doesnt_scroll_for_the_cursor_after_it() {
        let mut full = textbox("abcde");
        full.set_focused(true);
        full.cursor = full.len();
        full.scroll_to_cursor(40);
        assert_eq!(full.scroll, 0);
    }

    #[test]
    fn shows_the_start_once_unfocused() {
        let mut textbox = textbox("a fairly long song name");
        textbox.set_focused(true);
        textbox.cursor = textbox.len();
        let palette =
            IntuitionColorPalette::new([Color(0x000), Color(0xFFF), Color(0x666), Color(0xAAA)]);
        let mut screen = Framebuffer::new(AREA.w, AREA.h);
        let narrow = Rect::new(0, 0, 40, 10);
        textbox.render(&(), narrow, &mut Painter::new(&mut screen, &palette));
        assert!(textbox.scroll > 0);
        textbox.set_focused(false);
        textbox.render(&(), narrow, &mut Painter::new(&mut screen, &palette));
        assert_eq!(textbox.scroll, 0);
    }

    struct FakeEvent(TextInput);

    impl PointerEvent for FakeEvent {
        fn mouse(&self) -> Option<MouseEvent> {
            return None;
        }
    }

    impl TextEvent for FakeEvent {
        fn text_input(&self) -> Option<TextInput> {
            return Some(self.0);
        }
    }
}
