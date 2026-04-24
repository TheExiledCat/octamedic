pub enum InputEvent {
    MouseMove {
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
    },
    MouseDown {
        x: f32,
        y: f32,
        button: usize,
    },
    MouseUp {
        x: f32,
        y: f32,
        button: usize,
    },
}
