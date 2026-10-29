use std::collections::HashMap;

use ggez::graphics::{Canvas, Rect};
use taffy::{AvailableSpace, Display, NodeId, Style, TaffyTree, TraversePartialTree};

use crate::{
    app::state::AppState,
    framework::{
        input::MouseButton,
        widget::{DrawState, Widget},
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiEvent {
    Clicked { node: NodeId, button: MouseButton },
}

pub struct Ui {
    tree: TaffyTree<Box<dyn Widget>>,
    root: NodeId,
    hovered: Option<NodeId>,
    pressed: Option<(NodeId, MouseButton)>,
    unhandled_events: Vec<UiEvent>,
    hidden_displays: HashMap<NodeId, Display>,
}

// fills a node while its widget is detached
struct Placeholder;

impl Widget for Placeholder {
    fn draw(&self, _canvas: &mut Canvas, _draw: &DrawState, _state: &AppState) {}
}

impl Ui {
    pub fn new(root: impl Widget) -> Self {
        return Self::new_styled(root, |_| {});
    }

    pub fn new_styled(root: impl Widget, adjust_style: impl FnOnce(&mut Style)) -> Self {
        let mut style = root.style();
        adjust_style(&mut style);
        let mut tree = TaffyTree::new();
        let root_node = tree
            .new_leaf_with_context(style, Box::new(Placeholder) as Box<dyn Widget>)
            .expect("creating a root node can't fail");
        let mut ui = Self {
            tree,
            root: root_node,
            hovered: None,
            pressed: None,
            unhandled_events: Vec::new(),
            hidden_displays: HashMap::new(),
        };
        ui.mount_widget(root_node, root);
        return ui;
    }

    pub fn root(&self) -> NodeId {
        return self.root;
    }

    pub fn add(&mut self, parent: NodeId, widget: impl Widget) -> NodeId {
        return self.add_styled(parent, widget, |_| {});
    }

    pub fn add_styled(
        &mut self,
        parent: NodeId,
        widget: impl Widget,
        adjust_style: impl FnOnce(&mut Style),
    ) -> NodeId {
        let mut style = widget.style();
        adjust_style(&mut style);
        let node = self
            .tree
            .new_leaf_with_context(style, Box::new(Placeholder) as Box<dyn Widget>)
            .expect("creating a leaf node can't fail");
        self.tree
            .add_child(parent, node)
            .expect("parent should be a node in this ui");
        self.mount_widget(node, widget);
        return node;
    }

    fn mount_widget(&mut self, node: NodeId, mut widget: impl Widget) {
        widget.mount(self, node);
        if let Some(slot) = self.tree.get_node_context_mut(node) {
            *slot = Box::new(widget);
        }
    }

    pub fn remove(&mut self, node: NodeId) {
        for index in (0..self.tree.child_count(node)).rev() {
            if let Ok(child) = self.tree.child_at_index(node, index) {
                self.remove(child);
            }
        }
        if self.hovered == Some(node) {
            self.hovered = None;
        }
        if self.pressed.is_some_and(|(pressed, _)| pressed == node) {
            self.pressed = None;
        }
        // taffy's `remove` leaves the node's context behind, which would leak the widget and
        // let a stale id still find it.
        let _ = self.tree.set_node_context(node, None);
        let _ = self.tree.remove(node);
        self.hidden_displays.remove(&node);
    }

    pub fn set_hidden(&mut self, node: NodeId, hidden: bool) {
        let Ok(style) = self.tree.style(node) else {
            return;
        };
        let mut style = style.clone();
        match (hidden, self.hidden_displays.contains_key(&node)) {
            (true, false) => {
                self.hidden_displays.insert(node, style.display);
                style.display = Display::None;
            }
            (false, true) => style.display = self.hidden_displays.remove(&node).unwrap_or_default(),
            _ => return,
        }
        let _ = self.tree.set_style(node, style);
    }

    pub fn is_hidden(&self, node: NodeId) -> bool {
        return self.hidden_displays.contains_key(&node);
    }

    pub fn invalidate_layout(&mut self, node: NodeId) {
        let _ = self.tree.mark_dirty(node);
    }

    pub fn get<W: Widget>(&self, node: NodeId) -> Option<&W> {
        // `as_ref` first: calling `as_any` on the box itself would give the box, not the widget.
        return self
            .tree
            .get_node_context(node)?
            .as_ref()
            .as_any()
            .downcast_ref();
    }

    pub fn get_mut<W: Widget>(&mut self, node: NodeId) -> Option<&mut W> {
        let _ = self.tree.mark_dirty(node);
        return self
            .tree
            .get_node_context_mut(node)?
            .as_mut()
            .as_any_mut()
            .downcast_mut();
    }

    pub fn find_ancestor<W: Widget>(&self, node: NodeId) -> Option<NodeId> {
        let mut current = self.tree.parent(node);
        while let Some(ancestor) = current {
            if self.get::<W>(ancestor).is_some() {
                return Some(ancestor);
            }
            current = self.tree.parent(ancestor);
        }
        return None;
    }

    pub fn ancestor_mut<W: Widget>(&mut self, node: NodeId) -> Option<&mut W> {
        let ancestor = self.find_ancestor::<W>(node)?;
        return self.get_mut(ancestor);
    }

    fn with_widget_detached<R>(
        &mut self,
        node: NodeId,
        f: impl FnOnce(&mut dyn Widget, &mut Ui) -> R,
    ) -> Option<R> {
        let slot = self.tree.get_node_context_mut(node)?;
        let mut widget = std::mem::replace(slot, Box::new(Placeholder));
        let result = f(widget.as_mut(), self);
        if let Some(slot) = self.tree.get_node_context_mut(node) {
            *slot = widget;
        }
        return Some(result);
    }

    pub fn update(&mut self, state: &mut AppState) {
        let mut nodes = vec![self.root];
        let mut index = 0;
        while let Some(&node) = nodes.get(index) {
            for child in 0..self.tree.child_count(node) {
                if let Ok(child) = self.tree.child_at_index(node, child) {
                    nodes.push(child);
                }
            }
            index += 1;
        }
        for node in nodes {
            self.with_widget_detached(node, |widget, ui| widget.update(ui, state, node));
        }
    }

    pub fn bubble_event(&mut self, node: NodeId, event: UiEvent, state: &mut AppState) {
        let mut path = vec![];
        let mut current = Some(node);
        while let Some(node) = current {
            path.push(node);
            current = self.tree.parent(node);
        }
        for node in path {
            let handled = self
                .with_widget_detached(node, |widget, ui| widget.on_event(ui, state, node, &event));
            if handled == Some(true) {
                return;
            }
        }
        self.unhandled_events.push(event);
    }

    pub fn set_style(&mut self, node: NodeId, style: Style) {
        let _ = self.tree.set_style(node, style);
    }

    pub fn is_hovered(&self, node: NodeId) -> bool {
        return self.hovered == Some(node);
    }

    pub fn is_pressed(&self, node: NodeId) -> bool {
        return self.is_hovered(node) && self.pressed.is_some_and(|(pressed, _)| pressed == node);
    }

    pub fn compute_layout(
        &mut self,
        size: taffy::Size<f32>,
        mut measure: impl FnMut(
            &dyn Widget,
            taffy::Size<Option<f32>>,
            taffy::Size<AvailableSpace>,
        ) -> taffy::Size<f32>,
    ) {
        let available = taffy::Size {
            width: AvailableSpace::Definite(size.width),
            height: AvailableSpace::Definite(size.height),
        };
        let _ = self.tree.compute_layout_with_measure(
            self.root,
            available,
            |known, available, _, widget, _| match widget {
                Some(widget) => measure(widget.as_ref(), known, available),
                None => taffy::Size::ZERO,
            },
        );
    }

    pub fn screen_rect(&self, node: NodeId) -> Option<Rect> {
        let size = self.tree.layout(node).ok()?.size;
        let (mut x, mut y) = (0.0, 0.0);
        let mut current = Some(node);
        while let Some(node) = current {
            let location = self.tree.layout(node).ok()?.location;
            x += location.x;
            y += location.y;
            current = self.tree.parent(node);
        }
        return Some(Rect::new(x, y, size.width, size.height));
    }

    pub fn node_at(&self, point: [f32; 2]) -> Option<NodeId> {
        let mut node = self.root;
        let mut rect = self.child_screen_rect(node, (0.0, 0.0))?;
        if !rect.contains(point) {
            return None;
        }
        'descend: loop {
            // last child first, since it's drawn on top
            for index in (0..self.tree.child_count(node)).rev() {
                let Ok(child) = self.tree.child_at_index(node, index) else {
                    continue;
                };
                if self.is_hidden(child) {
                    continue;
                }
                let Some(child_rect) = self.child_screen_rect(child, (rect.x, rect.y)) else {
                    continue;
                };
                if child_rect.contains(point) {
                    node = child;
                    rect = child_rect;
                    continue 'descend;
                }
            }
            return Some(node);
        }
    }

    fn input_target_at(&self, point: [f32; 2]) -> Option<NodeId> {
        let mut current = self.node_at(point);
        while let Some(node) = current {
            if self
                .tree
                .get_node_context(node)
                .is_some_and(|widget| widget.is_interactive())
            {
                return Some(node);
            }
            current = self.tree.parent(node);
        }
        return None;
    }

    pub fn mouse_moved(&mut self, point: [f32; 2]) {
        self.hovered = self.input_target_at(point);
    }

    pub fn mouse_pressed(&mut self, point: [f32; 2], button: MouseButton) {
        self.mouse_moved(point);
        if self.pressed.is_none() {
            self.pressed = self.hovered.map(|node| (node, button));
        }
    }

    pub fn mouse_released(&mut self, point: [f32; 2], button: MouseButton, state: &mut AppState) {
        self.mouse_moved(point);
        let Some((node, pressed_button)) = self.pressed else {
            return;
        };
        if pressed_button != button {
            return;
        }
        self.pressed = None;
        if self.hovered == Some(node) {
            self.bubble_event(node, UiEvent::Clicked { node, button }, state);
        }
    }

    pub fn take_unhandled_events(&mut self) -> Vec<UiEvent> {
        return std::mem::take(&mut self.unhandled_events);
    }

    pub fn draw(&self, canvas: &mut Canvas, state: &AppState) {
        self.draw_node(canvas, self.root, (0.0, 0.0), state);
    }

    fn draw_node(&self, canvas: &mut Canvas, node: NodeId, origin: (f32, f32), state: &AppState) {
        if self.is_hidden(node) {
            return;
        }
        let Ok(layout) = self.tree.layout(node) else {
            return;
        };
        let Some(rect) = self.child_screen_rect(node, origin) else {
            return;
        };
        let inset =
            |side: fn(&taffy::Rect<f32>) -> f32| side(&layout.padding) + side(&layout.border);
        let content_rect = Rect::new(
            rect.x + inset(|r| r.left),
            rect.y + inset(|r| r.top),
            rect.w - inset(|r| r.left) - inset(|r| r.right),
            rect.h - inset(|r| r.top) - inset(|r| r.bottom),
        );
        let draw = DrawState {
            rect,
            content_rect,
            hovered: self.is_hovered(node),
            pressed: self.is_pressed(node),
        };
        let widget = self.tree.get_node_context(node);
        if let Some(widget) = widget {
            widget.draw(canvas, &draw, state);
        }
        for index in 0..self.tree.child_count(node) {
            if let Ok(child) = self.tree.child_at_index(node, index) {
                self.draw_node(canvas, child, (rect.x, rect.y), state);
            }
        }
        if let Some(widget) = widget {
            widget.draw_after_children(canvas, &draw, state);
        }
    }

    fn child_screen_rect(&self, node: NodeId, origin: (f32, f32)) -> Option<Rect> {
        let layout = self.tree.layout(node).ok()?;
        return Some(Rect::new(
            origin.0 + layout.location.x,
            origin.1 + layout.location.y,
            layout.size.width,
            layout.size.height,
        ));
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};
    use taffy::{
        Position, Size,
        prelude::{length, percent},
    };

    use super::*;
    use crate::framework::widgets::{button::Button, panel::Panel};

    fn state() -> AppState {
        return AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
    }

    /// An interactive widget with no looks, standing in for a button.
    struct Clickable;

    impl Widget for Clickable {
        fn draw(&self, _canvas: &mut Canvas, _draw: &DrawState, _state: &AppState) {}

        fn is_interactive(&self) -> bool {
            return true;
        }
    }

    fn sized(x: f32, y: f32, width: f32, height: f32) -> impl FnOnce(&mut Style) {
        return move |style| {
            *style = Style {
                position: Position::Absolute,
                inset: taffy::Rect {
                    left: length(x),
                    top: length(y),
                    right: taffy::prelude::auto(),
                    bottom: taffy::prelude::auto(),
                },
                size: Size {
                    width: length(width),
                    height: length(height),
                },
                ..Default::default()
            }
        };
    }

    fn ui() -> Ui {
        return Ui::new_styled(Panel::new(), |style| {
            style.size = Size {
                width: percent(1.0_f32),
                height: percent(1.0_f32),
            };
        });
    }

    fn layout(ui: &mut Ui) {
        ui.compute_layout(
            Size {
                width: 800.0,
                height: 600.0,
            },
            |widget, known, available| {
                let _ = (widget, available);
                Size {
                    width: known.width.unwrap_or(0.0),
                    height: known.height.unwrap_or(0.0),
                }
            },
        );
    }

    fn click(ui: &mut Ui, point: [f32; 2]) {
        ui.mouse_pressed(point, MouseButton::Left);
        ui.mouse_released(point, MouseButton::Left, &mut state());
    }

    #[test]
    fn releasing_over_the_pressed_widget_clicks_it() {
        let mut ui = ui();
        let button = ui.add_styled(ui.root(), Clickable, sized(10.0, 10.0, 50.0, 20.0));
        layout(&mut ui);
        click(&mut ui, [20.0, 20.0]);
        assert_eq!(
            ui.take_unhandled_events(),
            vec![UiEvent::Clicked {
                node: button,
                button: MouseButton::Left,
            }]
        );
        assert!(ui.take_unhandled_events().is_empty());
    }

    #[test]
    fn releasing_off_the_pressed_widget_does_not_click() {
        let mut ui = ui();
        ui.add_styled(ui.root(), Clickable, sized(10.0, 10.0, 50.0, 20.0));
        layout(&mut ui);
        ui.mouse_pressed([20.0, 20.0], MouseButton::Left);
        ui.mouse_released([200.0, 200.0], MouseButton::Left, &mut state());
        assert!(ui.take_unhandled_events().is_empty());
    }

    #[test]
    fn pressed_only_shows_while_the_cursor_is_over_the_widget() {
        let mut ui = ui();
        let button = ui.add_styled(ui.root(), Clickable, sized(10.0, 10.0, 50.0, 20.0));
        layout(&mut ui);
        ui.mouse_pressed([20.0, 20.0], MouseButton::Left);
        assert!(ui.is_pressed(button));
        ui.mouse_moved([200.0, 200.0]);
        assert!(!ui.is_pressed(button));
        ui.mouse_moved([20.0, 20.0]);
        assert!(ui.is_pressed(button));
    }

    #[test]
    fn clicking_a_child_that_takes_no_input_clicks_its_interactive_parent() {
        let mut ui = ui();
        let button = ui.add_styled(ui.root(), Clickable, sized(10.0, 10.0, 100.0, 40.0));
        ui.add_styled(button, Panel::new(), sized(5.0, 5.0, 20.0, 10.0));
        layout(&mut ui);
        click(&mut ui, [20.0, 20.0]);
        assert_eq!(
            ui.take_unhandled_events(),
            vec![UiEvent::Clicked {
                node: button,
                button: MouseButton::Left,
            }]
        );
    }

    #[test]
    fn the_last_added_sibling_is_on_top() {
        let mut ui = ui();
        ui.add_styled(ui.root(), Clickable, sized(0.0, 0.0, 100.0, 100.0));
        let top = ui.add_styled(ui.root(), Clickable, sized(50.0, 50.0, 100.0, 100.0));
        layout(&mut ui);
        assert_eq!(ui.node_at([75.0, 75.0]), Some(top));
    }

    #[test]
    fn nested_rects_are_in_screen_coordinates() {
        let mut ui = ui();
        let outer = ui.add_styled(ui.root(), Panel::new(), sized(100.0, 50.0, 200.0, 200.0));
        let inner = ui.add_styled(outer, Clickable, sized(10.0, 20.0, 30.0, 40.0));
        layout(&mut ui);
        assert_eq!(
            ui.screen_rect(inner),
            Some(Rect::new(110.0, 70.0, 30.0, 40.0))
        );
        assert_eq!(ui.node_at([115.0, 75.0]), Some(inner));
    }

    #[test]
    fn hover_follows_the_cursor() {
        let mut ui = ui();
        let button = ui.add_styled(ui.root(), Clickable, sized(10.0, 10.0, 50.0, 20.0));
        layout(&mut ui);
        ui.mouse_moved([20.0, 20.0]);
        assert!(ui.is_hovered(button));
        ui.mouse_moved([200.0, 200.0]);
        assert!(!ui.is_hovered(button));
    }

    #[test]
    fn widgets_are_reached_by_node_and_type() {
        let mut ui = ui();
        let node = ui.add(ui.root(), Clickable);
        assert!(ui.get::<Clickable>(node).is_some());
        assert!(ui.get_mut::<Clickable>(node).is_some());
        assert!(ui.get::<Panel>(node).is_none());
    }

    #[test]
    fn removing_a_pressed_widget_does_not_click_it_later() {
        let mut ui = ui();
        let panel = ui.add_styled(ui.root(), Panel::new(), sized(0.0, 0.0, 100.0, 100.0));
        let button = ui.add_styled(panel, Clickable, sized(10.0, 10.0, 50.0, 20.0));
        layout(&mut ui);
        ui.mouse_pressed([20.0, 20.0], MouseButton::Left);
        ui.remove(panel);
        let replacement = ui.add_styled(ui.root(), Clickable, sized(10.0, 10.0, 50.0, 20.0));
        layout(&mut ui);
        ui.mouse_released([20.0, 20.0], MouseButton::Left, &mut state());
        assert!(ui.take_unhandled_events().is_empty());
        assert!(ui.get::<Clickable>(button).is_none());
        assert!(ui.is_hovered(replacement));
    }

    /// A tab bar: one button per view, and clicking a button shows only its view.
    struct Tabs {
        tabs: Vec<(NodeId, NodeId)>,
    }

    impl Widget for Tabs {
        fn draw(&self, _canvas: &mut Canvas, _draw: &DrawState, _state: &AppState) {}

        fn mount(&mut self, ui: &mut Ui, node: NodeId) {
            for index in 0..2 {
                let x = index as f32 * 100.0;
                let button = ui.add_styled(node, Clickable, sized(x, 0.0, 100.0, 20.0));
                let view = ui.add_styled(node, Clickable, sized(0.0, 20.0, 400.0, 380.0));
                ui.set_hidden(view, index != 0);
                self.tabs.push((button, view));
            }
        }

        fn on_event(
            &mut self,
            ui: &mut Ui,
            _state: &mut AppState,
            _node: NodeId,
            event: &UiEvent,
        ) -> bool {
            let UiEvent::Clicked { node: clicked, .. } = *event;
            if !self.tabs.iter().any(|&(button, _)| button == clicked) {
                return false;
            }
            for &(button, view) in &self.tabs {
                ui.set_hidden(view, button != clicked);
            }
            return true;
        }
    }

    #[test]
    fn a_parent_widget_handles_its_childrens_clicks() {
        let mut ui = ui();
        let root = ui.root();
        let tabs = ui.add_styled(root, Tabs { tabs: vec![] }, sized(0.0, 0.0, 400.0, 400.0));
        let [(_, first_view), (_, second_view)] = ui.get::<Tabs>(tabs).unwrap().tabs[..] else {
            panic!("two tabs");
        };
        layout(&mut ui);
        assert_eq!(ui.node_at([50.0, 100.0]), Some(first_view));
        click(&mut ui, [150.0, 10.0]);
        layout(&mut ui);
        assert!(ui.is_hidden(first_view));
        assert!(!ui.is_hidden(second_view));
        assert_eq!(ui.node_at([50.0, 100.0]), Some(second_view));
        // the tab bar handled the click, so the app doesn't see it.
        assert!(ui.take_unhandled_events().is_empty());
    }

    #[test]
    fn clicks_no_widget_handles_reach_the_app() {
        let mut ui = ui();
        let root = ui.root();
        ui.add_styled(root, Tabs { tabs: vec![] }, sized(0.0, 0.0, 400.0, 400.0));
        layout(&mut ui);
        // the visible view is interactive but not a tab button.
        click(&mut ui, [50.0, 100.0]);
        assert_eq!(ui.take_unhandled_events().len(), 1);
    }

    #[test]
    fn showing_a_hidden_node_restores_its_display() {
        let mut ui = ui();
        let node = ui.add_styled(ui.root(), Panel::new(), |style| {
            *style = Style {
                display: Display::Grid,
                ..Default::default()
            }
        });
        ui.set_hidden(node, true);
        ui.set_hidden(node, true);
        ui.set_hidden(node, false);
        assert_eq!(ui.tree.style(node).unwrap().display, Display::Grid);
    }

    /// Counts its updates in the label-like `Counter` child it manages.
    struct Ticker {
        counter: Option<NodeId>,
    }

    struct Counter(u32);

    impl Widget for Counter {
        fn draw(&self, _canvas: &mut Canvas, _draw: &DrawState, _state: &AppState) {}
    }

    impl Widget for Ticker {
        fn draw(&self, _canvas: &mut Canvas, _draw: &DrawState, _state: &AppState) {}

        fn mount(&mut self, ui: &mut Ui, node: NodeId) {
            self.counter = Some(ui.add(node, Counter(0)));
        }

        fn update(&mut self, ui: &mut Ui, _state: &mut AppState, _node: NodeId) {
            if let Some(counter) = self.counter.and_then(|node| ui.get_mut::<Counter>(node)) {
                counter.0 += 1;
            }
        }
    }

    #[test]
    fn update_lets_widgets_change_their_children() {
        let mut ui = ui();
        let root = ui.root();
        let ticker = ui.add(root, Ticker { counter: None });
        let counter = ui.get::<Ticker>(ticker).unwrap().counter;
        ui.update(&mut state());
        ui.update(&mut state());
        assert_eq!(ui.get::<Counter>(counter.unwrap()).unwrap().0, 2);
    }

    /// Removes itself when clicked.
    struct CloseButton;

    impl Widget for CloseButton {
        fn draw(&self, _canvas: &mut Canvas, _draw: &DrawState, _state: &AppState) {}

        fn is_interactive(&self) -> bool {
            return true;
        }

        fn on_event(
            &mut self,
            ui: &mut Ui,
            _state: &mut AppState,
            node: NodeId,
            _event: &UiEvent,
        ) -> bool {
            ui.remove(node);
            return true;
        }
    }

    #[test]
    fn a_widget_can_remove_itself_while_handling_an_event() {
        let mut ui = ui();
        let button = ui.add_styled(ui.root(), CloseButton, sized(10.0, 10.0, 50.0, 20.0));
        layout(&mut ui);
        click(&mut ui, [20.0, 20.0]);
        assert!(ui.get::<CloseButton>(button).is_none());
        assert!(ui.take_unhandled_events().is_empty());
    }

    #[test]
    fn clicking_the_label_inside_a_button_clicks_the_button() {
        let mut ui = ui();
        let root = ui.root();
        let button = ui.add_styled(
            root,
            Button::new("Play", 12.0),
            sized(10.0, 10.0, 100.0, 40.0),
        );
        let label = ui.get::<Button>(button).unwrap().label_node().unwrap();
        assert_eq!(ui.tree.parent(label), Some(button));
        layout(&mut ui);
        click(&mut ui, [12.0, 12.0]);
        assert_eq!(
            ui.take_unhandled_events(),
            vec![UiEvent::Clicked {
                node: button,
                button: MouseButton::Left,
            }]
        );
    }

    #[test]
    fn a_buttons_click_handler_runs_and_stops_the_click_bubbling() {
        let mut ui = ui();
        let clicks = Rc::new(Cell::new(0));
        let counted = clicks.clone();
        let button = Button::new("Play", 12.0).on_click(move |_, _, _, _| {
            counted.set(counted.get() + 1);
            true
        });
        ui.add_styled(ui.root(), button, sized(10.0, 10.0, 100.0, 40.0));
        layout(&mut ui);
        click(&mut ui, [20.0, 20.0]);
        assert_eq!(clicks.get(), 1);
        assert!(ui.take_unhandled_events().is_empty());
    }

    /// Owns a button whose click handler reaches back to it through the ui.
    struct Owner {
        clicks: u32,
    }

    impl Widget for Owner {
        fn mount(&mut self, ui: &mut Ui, node: NodeId) {
            let button = Button::new("+1", 12.0).on_click(|ui, _, node, _| {
                if let Some(owner) = ui.ancestor_mut::<Owner>(node) {
                    owner.clicks += 1;
                }
                true
            });
            ui.add_styled(node, button, sized(10.0, 10.0, 100.0, 40.0));
        }

        fn draw(&self, _canvas: &mut Canvas, _draw: &DrawState, _state: &AppState) {}
    }

    #[test]
    fn a_click_handler_can_change_the_widget_that_owns_the_button() {
        let mut ui = ui();
        let owner = ui.add_styled(
            ui.root(),
            Owner { clicks: 0 },
            sized(0.0, 0.0, 400.0, 400.0),
        );
        layout(&mut ui);
        click(&mut ui, [20.0, 20.0]);
        click(&mut ui, [20.0, 20.0]);
        assert_eq!(ui.get::<Owner>(owner).unwrap().clicks, 2);
    }

    #[test]
    fn find_ancestor_skips_the_node_itself_and_finds_the_nearest_match() {
        let mut ui = ui();
        let outer = ui.add(ui.root(), Owner { clicks: 0 });
        let inner = ui.add(outer, Owner { clicks: 0 });
        let leaf = ui.add(inner, Panel::new());
        assert_eq!(ui.find_ancestor::<Owner>(leaf), Some(inner));
        assert_eq!(ui.find_ancestor::<Owner>(inner), Some(outer));
        assert_eq!(ui.find_ancestor::<Owner>(outer), None);
    }

    #[test]
    fn a_click_handler_returning_false_lets_the_click_bubble() {
        let mut ui = ui();
        let button = Button::new("Play", 12.0).on_click(|_, _, _, _| false);
        let node = ui.add_styled(ui.root(), button, sized(10.0, 10.0, 100.0, 40.0));
        layout(&mut ui);
        click(&mut ui, [20.0, 20.0]);
        assert_eq!(
            ui.take_unhandled_events(),
            vec![UiEvent::Clicked {
                node,
                button: MouseButton::Left,
            }]
        );
    }
}
