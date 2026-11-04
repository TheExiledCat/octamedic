use crate::{
    event::PointerEvent,
    gadget::Gadget,
    gadgets::{
        button::{Button, Triangle},
        dialog::{BUTTON_LABEL_PADDING, DialogComponent, DialogInput, Dropdown, FIELD_HEIGHT},
    },
    painter::{CHAR_WIDTH, Painter, text_width},
    rect::Rect,
    shape::Direction,
};

const ROW_GAP: i32 = 2;
const ARROW_BUTTON_WIDTH: i32 = 16;
const BUTTON_GAP: i32 = CHAR_WIDTH / 2;

/// One row of a [`ReorderableList`]: which option it shows and whether it's switched on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListItem {
    pub choice: usize,
    pub enabled: bool,
}

impl ListItem {
    pub fn new(choice: usize) -> Self {
        return Self {
            choice,
            enabled: true,
        };
    }
}

/// What was just done to a [`ReorderableList`], so whatever it shows can follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListEvent {
    /// A new row at the end, showing the first option.
    Add,
    Remove(usize),
    /// A row moved one place up or down.
    Move {
        from: usize,
        to: usize,
    },
    Choose {
        item: usize,
        choice: usize,
    },
    Enable {
        item: usize,
        enabled: bool,
    },
    /// The row's edit button, to open its settings.
    Edit(usize),
}

type OnEvent<S> = Box<dyn FnMut(&mut S, ListEvent)>;

/// Rows that can be put in order with up and down arrows, each picking one of the same options
/// from a dropdown, with a button to switch it on and off. Edit, remove and add buttons are
/// there when asked for, like an effects chain. Its value is the rows.
pub struct ReorderableList<S, E> {
    options: Vec<String>,
    rows: Vec<Row<S, E>>,
    can_add: bool,
    can_remove: bool,
    editable: bool,
    add: Button<S, E>,
    on_event: Option<OnEvent<S>>,
}

impl<S, E> ReorderableList<S, E> {
    pub fn new(options: &[&str]) -> Self {
        return Self {
            options: options.iter().map(|option| option.to_string()).collect(),
            rows: Vec::new(),
            can_add: false,
            can_remove: false,
            editable: false,
            add: Button::new("Add"),
            on_event: None,
        };
    }

    pub fn items(mut self, items: &[ListItem]) -> Self {
        self.set_items(items);
        return self;
    }

    /// Shows an Add button under the rows.
    pub fn can_add(mut self, can_add: bool) -> Self {
        self.can_add = can_add;
        return self;
    }

    /// Gives each row a Del button.
    pub fn can_remove(mut self, can_remove: bool) -> Self {
        self.can_remove = can_remove;
        return self;
    }

    /// Gives each row an Edit button, which sends [`ListEvent::Edit`].
    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        return self;
    }

    /// Runs on every change, as it's made.
    pub fn on_event(mut self, handler: impl FnMut(&mut S, ListEvent) + 'static) -> Self {
        self.on_event = Some(Box::new(handler));
        return self;
    }

    fn set_items(&mut self, items: &[ListItem]) {
        let options: Vec<&str> = self.options.iter().map(String::as_str).collect();
        self.rows = items.iter().map(|item| Row::new(&options, *item)).collect();
    }

    fn lines(&self) -> i32 {
        return self.rows.len() as i32 + self.can_add as i32;
    }

    fn line(area: Rect, index: usize) -> Rect {
        let y = area.y + index as i32 * (FIELD_HEIGHT + ROW_GAP);
        return Rect::new(area.x, y, area.w, FIELD_HEIGHT);
    }

    fn emit(&mut self, state: &mut S, event: ListEvent) {
        if let Some(handler) = &mut self.on_event {
            handler(state, event);
        }
    }

    fn perform(&mut self, state: &mut S, index: usize, action: RowAction) {
        let event = match action {
            RowAction::Up if index > 0 => {
                self.rows.swap(index, index - 1);
                ListEvent::Move {
                    from: index,
                    to: index - 1,
                }
            }
            RowAction::Down if index + 1 < self.rows.len() => {
                self.rows.swap(index, index + 1);
                ListEvent::Move {
                    from: index,
                    to: index + 1,
                }
            }
            RowAction::Choose(choice) => ListEvent::Choose {
                item: index,
                choice,
            },
            RowAction::Enable(enabled) => ListEvent::Enable {
                item: index,
                enabled,
            },
            RowAction::Edit => ListEvent::Edit(index),
            RowAction::Remove => {
                self.rows.remove(index);
                ListEvent::Remove(index)
            }
            RowAction::Up | RowAction::Down | RowAction::Took => return,
        };
        self.emit(state, event);
    }
}

impl<S, E: PointerEvent> Gadget<S, E> for ReorderableList<S, E> {
    fn handle_event(&mut self, state: &mut S, event: &E) -> bool {
        let open = self
            .rows
            .iter()
            .position(|row| row.choice.captures_events());
        let rows = open.map_or(0..self.rows.len(), |index| index..index + 1);
        for index in rows {
            if let Some(action) = self.rows[index].handle(state, event) {
                self.perform(state, index, action);
                return true;
            }
        }
        if open.is_some() {
            return true;
        }
        if self.can_add && self.add.handle_event(state, event) {
            if self.add.clicked().is_some() {
                let options: Vec<&str> = self.options.iter().map(String::as_str).collect();
                self.rows.push(Row::new(&options, ListItem::new(0)));
                self.emit(state, ListEvent::Add);
            }
            return true;
        }
        return false;
    }

    fn render(&mut self, state: &S, area: Rect, painter: &mut Painter) {
        let last = self.rows.len().saturating_sub(1);
        for (index, row) in self.rows.iter_mut().enumerate() {
            row.up.set_enabled(index > 0);
            row.down.set_enabled(index < last);
            row.render(
                state,
                Self::line(area, index),
                self.editable,
                self.can_remove,
                painter,
            );
        }
        if self.can_add {
            let line = Self::line(area, self.rows.len());
            let add = Rect::new(line.x, line.y, label_width("Add"), line.h);
            self.add.render(state, add, painter);
        }
    }
}

impl<S, E: PointerEvent> DialogComponent<S, E> for ReorderableList<S, E> {
    fn height(&self) -> i32 {
        let lines = self.lines();
        return lines * FIELD_HEIGHT + (lines - 1).max(0) * ROW_GAP;
    }

    fn render_overlay(&mut self, state: &S, painter: &mut Painter) {
        for row in &mut self.rows {
            row.choice.render_overlay(state, painter);
        }
    }

    fn captures_events(&self) -> bool {
        return self.rows.iter().any(|row| row.choice.captures_events());
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.add.set_enabled(enabled);
        for row in &mut self.rows {
            row.set_enabled(enabled);
        }
    }
}

impl<S, E: PointerEvent> DialogInput<S, E> for ReorderableList<S, E> {
    type Value = Vec<ListItem>;

    fn value(&self) -> Vec<ListItem> {
        return self.rows.iter().map(Row::item).collect();
    }

    fn set_value(&mut self, items: Vec<ListItem>) {
        self.set_items(&items);
    }
}

enum RowAction {
    Up,
    Down,
    Choose(usize),
    Enable(bool),
    Edit,
    Remove,
    // taken without anything changing, like the dropdown opening
    Took,
}

struct Row<S, E> {
    up: Button<S, E>,
    down: Button<S, E>,
    choice: Dropdown<S, E>,
    enabled: Button<S, E>,
    edit: Button<S, E>,
    remove: Button<S, E>,
}

impl<S, E> Row<S, E> {
    fn new(options: &[&str], item: ListItem) -> Self {
        let mut enabled = Button::new("On");
        enabled.set_toggled(item.enabled);
        return Self {
            up: Button::new(Triangle::arrow(Direction::Up)),
            down: Button::new(Triangle::arrow(Direction::Down)),
            choice: Dropdown::new("", options).chosen(item.choice),
            enabled,
            edit: Button::new("Edit"),
            remove: Button::new("Del"),
        };
    }
}

impl<S, E: PointerEvent> Row<S, E> {
    fn item(&self) -> ListItem {
        return ListItem {
            choice: self.choice.value(),
            enabled: self.enabled.is_toggled(),
        };
    }

    fn set_enabled(&mut self, enabled: bool) {
        for button in [
            &mut self.up,
            &mut self.down,
            &mut self.enabled,
            &mut self.edit,
            &mut self.remove,
        ] {
            button.set_enabled(enabled);
        }
        DialogComponent::set_enabled(&mut self.choice, enabled);
    }

    fn handle(&mut self, state: &mut S, event: &E) -> Option<RowAction> {
        let chosen = self.choice.value();
        if self.choice.handle_event(state, event) {
            let now = self.choice.value();
            return Some(if now == chosen {
                RowAction::Took
            } else {
                RowAction::Choose(now)
            });
        }
        let buttons = [
            (&mut self.up, RowAction::Up),
            (&mut self.down, RowAction::Down),
            (&mut self.edit, RowAction::Edit),
            (&mut self.remove, RowAction::Remove),
        ];
        for (button, action) in buttons {
            if button.handle_event(state, event) {
                let clicked = button.clicked().is_some();
                return Some(if clicked { action } else { RowAction::Took });
            }
        }
        if self.enabled.handle_event(state, event) {
            if self.enabled.clicked().is_none() {
                return Some(RowAction::Took);
            }
            self.enabled.toggle();
            return Some(RowAction::Enable(self.enabled.is_toggled()));
        }
        return None;
    }

    // the arrows, the dropdown, then on, edit and del
    fn render(
        &mut self,
        state: &S,
        line: Rect,
        editable: bool,
        removable: bool,
        painter: &mut Painter,
    ) {
        let mut x = line.x;
        let mut next = |width: i32, gap: i32| {
            let area = Rect::new(x, line.y, width, line.h);
            x += width + gap;
            area
        };
        self.up.render(state, next(ARROW_BUTTON_WIDTH, 0), painter);
        self.down
            .render(state, next(ARROW_BUTTON_WIDTH, BUTTON_GAP), painter);
        let choice = next(self.choice.button_width(), BUTTON_GAP);
        self.choice.render(state, choice, painter);
        self.enabled
            .render(state, next(label_width("On"), BUTTON_GAP), painter);
        if editable {
            self.edit
                .render(state, next(label_width("Edit"), BUTTON_GAP), painter);
        }
        if removable {
            self.remove
                .render(state, next(label_width("Del"), BUTTON_GAP), painter);
        }
    }
}

fn label_width(label: &str) -> i32 {
    return text_width(label) + 2 * BUTTON_LABEL_PADDING;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        event::{MouseButton, MouseEvent},
        gadgets::dialog::DROPDOWN_ROW_HEIGHT,
        palette::{Color, IntuitionColorPalette},
        rect::Point,
        target::Framebuffer,
    };

    const AREA: Rect = Rect::new(0, 0, 320, 100);
    const OPTIONS: [&str; 3] = ["Scanlines", "Chroma", "Wobble"];

    type Events = Vec<ListEvent>;
    type TestList = ReorderableList<Events, MouseEvent>;

    fn list(items: &[ListItem]) -> TestList {
        return ReorderableList::new(&OPTIONS)
            .items(items)
            .can_add(true)
            .can_remove(true)
            .editable(true)
            .on_event(|events: &mut Events, event| events.push(event));
    }

    fn palette() -> IntuitionColorPalette {
        return IntuitionColorPalette::new([
            Color(0x000),
            Color(0xFFF),
            Color(0x666),
            Color(0xAAA),
        ]);
    }

    fn render(list: &mut TestList, events: &Events) {
        let mut screen = Framebuffer::new(AREA.w, AREA.h);
        let palette = palette();
        let mut painter = Painter::new(&mut screen, &palette);
        list.render(events, AREA, &mut painter);
        list.render_overlay(events, &mut painter);
    }

    fn click(list: &mut TestList, events: &mut Events, area: Rect) {
        let pos = Point::new(area.x + area.w / 2, area.y + area.h / 2);
        let press = MouseEvent::Down {
            pos,
            button: MouseButton::Left,
        };
        assert!(list.handle_event(events, &press));
        render(list, events);
    }

    fn three() -> [ListItem; 3] {
        return [ListItem::new(0), ListItem::new(1), ListItem::new(2)];
    }

    #[test]
    fn arrows_move_rows() {
        let mut events = Events::new();
        let mut list = list(&three());
        render(&mut list, &events);
        let down = list.rows[0].down.area();
        click(&mut list, &mut events, down);
        let up = list.rows[2].up.area();
        click(&mut list, &mut events, up);
        let choices: Vec<usize> = list.value().iter().map(|item| item.choice).collect();
        assert_eq!(choices, [1, 2, 0]);
        assert_eq!(
            events,
            [
                ListEvent::Move { from: 0, to: 1 },
                ListEvent::Move { from: 2, to: 1 }
            ]
        );
    }

    #[test]
    fn the_ends_dont_move_further() {
        let events = Events::new();
        let mut list = list(&three());
        render(&mut list, &events);
        assert!(!list.rows[0].up.is_enabled());
        assert!(!list.rows[2].down.is_enabled());
        assert!(list.rows[1].up.is_enabled());
    }

    #[test]
    fn add_remove_enable_and_edit() {
        let mut events = Events::new();
        let mut list = list(&[ListItem::new(2)]);
        render(&mut list, &events);
        let add = list.add.area();
        click(&mut list, &mut events, add);
        let enabled = list.rows[1].enabled.area();
        click(&mut list, &mut events, enabled);
        let edit = list.rows[1].edit.area();
        click(&mut list, &mut events, edit);
        let remove = list.rows[0].remove.area();
        click(&mut list, &mut events, remove);
        assert_eq!(
            events,
            [
                ListEvent::Add,
                ListEvent::Enable {
                    item: 1,
                    enabled: false
                },
                ListEvent::Edit(1),
                ListEvent::Remove(0),
            ]
        );
        assert_eq!(
            list.value(),
            [ListItem {
                choice: 0,
                enabled: false
            }]
        );
    }

    #[test]
    fn choosing_from_the_dropdown() {
        let mut events = Events::new();
        let mut list = list(&[ListItem::new(0)]);
        render(&mut list, &events);
        let button = list.rows[0].choice_button();
        click(&mut list, &mut events, button);
        assert!(list.captures_events());
        // the third option's row in the list that opened under the button
        let y = button.bottom() + 1 + 2 * DROPDOWN_ROW_HEIGHT;
        let third = Rect::new(button.x, y, button.w, DROPDOWN_ROW_HEIGHT);
        click(&mut list, &mut events, third);
        assert_eq!(events, [ListEvent::Choose { item: 0, choice: 2 }]);
    }

    #[test]
    fn height_fits_the_rows_and_add() {
        let list = list(&three());
        assert_eq!(list.height(), 4 * FIELD_HEIGHT + 3 * ROW_GAP);
        let without_add = ReorderableList::<Events, MouseEvent>::new(&OPTIONS);
        assert_eq!(without_add.height(), 0);
    }

    impl<S, E> Row<S, E> {
        fn choice_button(&self) -> Rect {
            let x = self.up.area().x + 2 * ARROW_BUTTON_WIDTH + BUTTON_GAP;
            let width = self.choice.button_width();
            return Rect::new(x, self.up.area().y, width, FIELD_HEIGHT);
        }
    }
}
