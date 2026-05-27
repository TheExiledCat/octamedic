use ggez::mint::Point2;

pub enum InputEvent {
    MouseMove {
        pos: Point2<f32>,
        delta: Point2<f32>,
    },
    MouseDown {
        pos: Point2<f32>,
        button: MouseButton,
    },
    MouseUp {
        pos: Point2<f32>,
        button: MouseButton,
    },
}

#[derive(Eq, PartialEq)]

pub enum MouseButton {
    Left,
    Right,
}
