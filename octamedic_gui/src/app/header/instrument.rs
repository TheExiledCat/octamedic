use octamed::mmd::module::INSTRUMENT_NAME_LENGTH;
use octamedic_core::data::instrument::{
    InstrumentKind, MAX_FINETUNE, MAX_INSTRUMENT_VOLUME, MIN_FINETUNE, OctamedicInstrument,
};
use octamedic_util::base32;
use rustuition::{
    gadget::Gadget,
    gadgets::{
        button::{ContentArea, DEFAULT_REPEAT_DELAY, DEFAULT_REPEAT_INTERVAL, IncDec, Triangle},
        slider::SliderDirection,
    },
    painter::{Align, Painter, Pen},
    rect::Rect,
    shape::Direction::{Left, Right},
};

use crate::{
    app::{fonts, state::AppState},
    framework::{
        event::Event,
        widgets::{
            Button, Slider, Textbox,
            symbols::{draw_rows, picture},
        },
    },
};

// from the panel's top left
const NUMBER: Rect = Rect::new(2, 0, 20, 10);
const NAME: Rect = Rect::new(22, 0, 340, 10);
const LENGTH: Rect = Rect::new(394, 0, 52, 10);
const TRANSPORT: [Rect; 5] = [
    Rect::new(2, 10, 31, 11),
    Rect::new(33, 10, 31, 11),
    Rect::new(64, 10, 31, 11),
    Rect::new(95, 10, 31, 11),
    Rect::new(126, 10, 21, 11),
];
const FLUSH: Rect = Rect::new(2, 21, 44, 11);
const LOAD: Rect = Rect::new(46, 21, 102, 11);
// a frame with a label, a sunken number box, then - and +
const STEPPED_GROUPS: [(&str, Rect, [i32; 2], Rect); 2] = [
    (
        "TRANSPOSE",
        Rect::new(147, 10, 85, 22),
        [154, 12],
        Rect::new(153, 18, 36, 11),
    ),
    (
        "FINETUNE",
        Rect::new(232, 10, 76, 22),
        [241, 12],
        Rect::new(238, 18, 28, 11),
    ),
];
const STEP_BUTTON_WIDTH: i32 = 18;
const SUNKEN_BOXES: [Rect; 2] = [Rect::new(334, 10, 28, 11), Rect::new(334, 21, 28, 11)];
const REPEAT_BOXES: [Rect; 2] = [Rect::new(394, 10, 52, 11), Rect::new(394, 21, 52, 11)];
const LABELS: [(&str, [i32; 2]); 7] = [
    ("LEN:", [368, 2]),
    ("HLD:", [311, 13]),
    ("DEC:", [311, 24]),
    ("R/B:", [368, 13]),
    ("R/L:", [368, 24]),
    ("TYPE:", [2, 36]),
    ("VOL:", [193, 36]),
];
const TYPES: [(&str, Rect); 3] = [
    ("SAMPLE", Rect::new(37, 32, 50, 13)),
    ("SYNTH", Rect::new(87, 32, 48, 13)),
    ("HYBRID", Rect::new(135, 32, 48, 13)),
];
const VOLUME: Rect = Rect::new(224, 32, 50, 13);
const VOLUME_SLIDER: Rect = Rect::new(274, 32, 138, 13);
const VOLUME_STEPS: [Rect; 2] = [Rect::new(412, 32, 18, 13), Rect::new(430, 32, 18, 13)];

const TRANSPORT_ARROW: [i32; 2] = [8, 7];
const BAR_GAP: i32 = 2;
const STEP_SIGN: [i32; 3] = [8, 7, 1];
// play to the end of the range and back, going by the picture
const RANGE_PICTURE: &[&str] = &[
    "##.....##.....##",
    "####...##...##.#",
    "######.##.##...#",
    "##########.....#",
    "######.##.##...#",
    "####...##...##.#",
    "##.....##.....##",
];
// LOAD's arrows, and where they and the label go inside the button's frame
const LOAD_ARROW: &[&str] = &[
    "......##......",
    "....######....",
    "..##########..",
    "##############",
    "....######....",
    "....######....",
];
const LOAD_ARROWS_X: [i32; 2] = [6, 72];
const LOAD_TEXT: [i32; 2] = [31, 2];
const LOAD_ARROWS_Y: i32 = 1;

pub struct Instrument {
    number: Button,
    name: Textbox,
    length: Button,
    transport: [Button; 5],
    flush: Button,
    load: Button,
    // transpose, then finetune
    stepped: [Stepper; 2],
    // hold and decay, shown but not edited yet
    sunken: [Textbox; 2],
    // the loop's start and length
    repeats: [Textbox; 2],
    types: [Button; 3],
    volume: Button,
    volume_slider: Slider,
    volume_steps: [Button; 2],
    // the level last set on the slider, so a drag shows up as a change
    slider_level: u16,
}

impl Instrument {
    pub fn new() -> Self {
        let [w, h] = TRANSPORT_ARROW;
        let transport = [
            Button::new(Triangle::new(Left, w, h).with_bar().bar_gap(BAR_GAP)),
            Button::new(Triangle::new(Left, w, h)),
            Button::new(Triangle::new(Right, w, h)),
            Button::new(Triangle::new(Right, w, h).with_bar().bar_gap(BAR_GAP)),
            Button::new(picture(RANGE_PICTURE)),
        ];
        let name = Textbox::new()
            .max_length(INSTRUMENT_NAME_LENGTH - 1)
            .on_update(|state: &mut AppState, name| {
                state.edit_active_instrument(|instrument| instrument.name = name.to_owned());
            });
        let steps = MAX_INSTRUMENT_VOLUME as u16 + 1;
        return Self {
            number: Button::new("01").font(fonts::INTERFACE),
            name,
            length: Button::new("-").align(Align::Right),
            transport: transport.map(unhooked),
            flush: label_button("FLUSH"),
            load: unhooked(Button::new(load)),
            stepped: [
                Stepper::new(i8::MIN as i32, i8::MAX as i32),
                Stepper::new(MIN_FINETUNE as i32, MAX_FINETUNE as i32),
            ],
            sunken: std::array::from_fn(|_| Textbox::new()),
            repeats: std::array::from_fn(|_| Textbox::new()),
            types: TYPES.map(|(label, _)| label_button(label)),
            volume: Button::new("00/00").font(fonts::INTERFACE),
            volume_slider: Slider::new(SliderDirection::Horizontal, false, steps),
            volume_steps: [Left, Right]
                .map(|direction| repeating(Button::new(Triangle::new(direction, w, h)))),
            slider_level: 0,
        };
    }

    fn update_volume(&mut self, state: &mut AppState, volume: u8) {
        for step in &mut self.volume_steps {
            step.update(state);
        }
        let [down, up] = &mut self.volume_steps;
        let mut wanted = None;
        if down.clicked().is_some() {
            wanted = Some(volume.saturating_sub(1));
        }
        if up.clicked().is_some() {
            wanted = Some(volume.saturating_add(1).min(MAX_INSTRUMENT_VOLUME));
        }
        if self.volume_slider.value() != self.slider_level {
            wanted = Some(self.volume_slider.value() as u8);
        }
        if let Some(wanted) = wanted.filter(|&wanted| wanted != volume) {
            state.edit_active_instrument(|instrument| instrument.volume = wanted);
        }
        let volume = active(state).map_or(0, |instrument| instrument.volume);
        self.volume_slider.set_value(volume as u16);
        self.slider_level = self.volume_slider.value();
        self.volume.set_text(format!("{volume:02}/{volume:02X}"));
    }
}

// a number typed in its box or stepped with - and +, from `min` to `max`
struct Stepper {
    textbox: Textbox,
    steps: [Button; 2],
    min: i32,
    max: i32,
}

impl Stepper {
    fn new(min: i32, max: i32) -> Self {
        let [w, h, thickness] = STEP_SIGN;
        return Self {
            textbox: Textbox::new(),
            steps: [false, true]
                .map(|plus| repeating(Button::new(IncDec::new(plus, w, h, thickness)))),
            min,
            max,
        };
    }

    // the new value, if it's changed from `current`
    fn update(&mut self, state: &mut AppState, current: i32) -> Option<i32> {
        let mut wanted = None;
        for (step, delta) in self.steps.iter_mut().zip([-1, 1]) {
            step.update(state);
            if step.clicked().is_some() {
                self.textbox.set_focused(false);
                wanted = Some(current + delta);
            }
        }
        if self.textbox.is_focused() {
            let typed = self.textbox.text().trim().parse::<i32>().ok();
            wanted = wanted.or(typed.filter(|value| (self.min..=self.max).contains(value)));
        } else {
            self.textbox.set_text(current.to_string());
        }
        return wanted
            .map(|value| value.clamp(self.min, self.max))
            .filter(|&value| value != current);
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        return self.textbox.handle_event(state, event)
            || self
                .steps
                .iter_mut()
                .any(|step| step.handle_event(state, event));
    }
}

fn active(state: &AppState) -> Option<&OctamedicInstrument> {
    return state.song().instrument(state.active_instrument);
}

impl Gadget<AppState, Event> for Instrument {
    fn update(&mut self, state: &mut AppState) {
        if let Some(clear) = state.name_edit_request.take() {
            if clear {
                self.name.set_text("");
                state.edit_active_instrument(|instrument| instrument.name.clear());
            }
            self.name.set_focused(true);
            self.name.cursor_to_end();
        }
        let id = state.active_instrument;
        self.number.set_text(base32::format(id.0 as u32, 2, '0'));
        let [transpose, finetune] = &mut self.stepped;
        let current = active(state).map_or(0, |instrument| instrument.transpose as i32);
        if let Some(value) = transpose.update(state, current) {
            state.edit_active_instrument(|instrument| instrument.transpose = value as i8);
        }
        let current = active(state).map_or(0, |instrument| instrument.finetune as i32);
        if let Some(value) = finetune.update(state, current) {
            state.edit_active_instrument(|instrument| instrument.finetune = value as i8);
        }
        let volume = active(state).map_or(0, |instrument| instrument.volume);
        self.update_volume(state, volume);

        let instrument = active(state);
        if !self.name.is_focused() {
            let name = instrument.map_or("", |instrument| instrument.name.as_str());
            self.name.set_text(name);
        }
        let sample = instrument.and_then(OctamedicInstrument::sample);
        let length = sample.map_or("-".to_owned(), |sample| sample.data.len().to_string());
        self.length.set_text(length);
        let [hold, decay] = &mut self.sunken;
        hold.set_text(
            instrument
                .map_or(0, |instrument| instrument.hold)
                .to_string(),
        );
        decay.set_text(
            instrument
                .map_or(0, |instrument| instrument.decay)
                .to_string(),
        );
        let (start, end) = sample.and_then(|sample| sample.repeat).unwrap_or((0, 0));
        let [repeat, repeat_length] = &mut self.repeats;
        repeat.set_text(start.to_string());
        repeat_length.set_text((end - start).to_string());
        let kind = instrument.map(|instrument| &instrument.kind);
        let toggled = [
            matches!(kind, Some(InstrumentKind::Sample(_))),
            matches!(kind, Some(InstrumentKind::Synth(_))),
            matches!(kind, Some(InstrumentKind::Hybrid(..))),
        ];
        for (button, toggled) in self.types.iter_mut().zip(toggled) {
            button.set_toggled(toggled);
        }
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        return self.name.handle_event(state, event)
            || self
                .stepped
                .iter_mut()
                .any(|stepper| stepper.handle_event(state, event))
            || self.volume_slider.handle_event(state, event)
            || self
                .volume_steps
                .iter_mut()
                .any(|step| step.handle_event(state, event));
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        self.number.render(state, NUMBER.within(area), painter);
        self.name.render(state, NAME.within(area), painter);
        self.length.render(state, LENGTH.within(area), painter);
        for (button, rect) in self.transport.iter_mut().zip(TRANSPORT) {
            button.render(state, rect.within(area), painter);
        }
        self.flush.render(state, FLUSH.within(area), painter);
        self.load.render(state, LOAD.within(area), painter);
        for (
            Stepper {
                textbox: number,
                steps,
                ..
            },
            (label, group, [x, y], rect),
        ) in self.stepped.iter_mut().zip(STEPPED_GROUPS)
        {
            painter.frame(group.within(area), 1);
            painter.text_in_font(area.x + x, area.y + y, label, Pen::BACKGROUND, fonts::LABEL);
            render_sunken(number, state, rect.within(area), painter);
            for (index, step) in steps.iter_mut().enumerate() {
                let x = rect.right() + index as i32 * STEP_BUTTON_WIDTH;
                let step_rect = Rect::new(x, rect.y, STEP_BUTTON_WIDTH, rect.h);
                step.render(state, step_rect.within(area), painter);
            }
        }
        for (number, rect) in self.sunken.iter_mut().zip(SUNKEN_BOXES) {
            render_sunken(number, state, rect.within(area), painter);
        }
        for (number, rect) in self.repeats.iter_mut().zip(REPEAT_BOXES) {
            number.render(state, rect.within(area), painter);
        }
        for (label, [x, y]) in LABELS {
            painter.text_in_font(area.x + x, area.y + y, label, Pen::BACKGROUND, fonts::LABEL);
        }
        for (button, (_, rect)) in self.types.iter_mut().zip(TYPES) {
            button.render(state, rect.within(area), painter);
        }
        self.volume.render(state, VOLUME.within(area), painter);
        self.volume_slider
            .render(state, VOLUME_SLIDER.within(area), painter);
        for (button, rect) in self.volume_steps.iter_mut().zip(VOLUME_STEPS) {
            button.render(state, rect.within(area), painter);
        }
    }
}

fn unhooked(button: Button) -> Button {
    return button.enabled(false);
}

// held down, it keeps stepping
fn repeating(button: Button) -> Button {
    return button.repeat(DEFAULT_REPEAT_DELAY, DEFAULT_REPEAT_INTERVAL);
}

fn label_button(label: &'static str) -> Button {
    return unhooked(Button::new(label).font(fonts::LABEL));
}

// a text box draws a raised frame, these are sunk in
fn render_sunken(textbox: &mut Textbox, state: &AppState, rect: Rect, painter: &mut Painter) {
    textbox.render(state, rect, painter);
    painter.frame_inverted(rect, 1);
}

// LOAD between two arrows pointing up
fn load(area: ContentArea, pen: Pen, painter: &mut Painter) {
    let inside = area.inside;
    for x in LOAD_ARROWS_X {
        draw_rows(
            LOAD_ARROW,
            inside.x + x,
            inside.y + LOAD_ARROWS_Y,
            pen,
            painter,
        );
    }
    let [x, y] = LOAD_TEXT;
    painter.text_in_font(inside.x + x, inside.y + y, "LOAD", pen, fonts::LABEL);
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use octamedic_core::{
        data::{
            instrument::{InstrumentId, OctamedicSample, SampleOctaves},
            project::OctamedicProject,
            song::SongId,
        },
        engine::handle::EngineHandle,
    };
    use rustuition::target::Framebuffer;

    use super::*;
    use crate::framework::event::MouseButton;

    const AREA: Rect = Rect::new(0, 0, 448, 45);

    fn panel() -> (Instrument, AppState) {
        let mut instrument = OctamedicInstrument::new("bass");
        instrument.kind = InstrumentKind::Sample(Arc::new(OctamedicSample {
            data: vec![0; 100],
            repeat: Some((20, 60)),
            octaves: SampleOctaves::One,
        }));
        let mut project = OctamedicProject::new();
        let song = project.get_song_mut(SongId(0)).unwrap();
        song.set_instrument(InstrumentId(1), Some(instrument));
        let mut state = AppState::new(project, EngineHandle::disconnected());
        let mut panel = Instrument::new();
        update(&mut panel, &mut state);
        return (panel, state);
    }

    fn update(panel: &mut Instrument, state: &mut AppState) {
        panel.update(state);
        let mut screen = Framebuffer::new(640, 256);
        panel.render(state, AREA, &mut Painter::new(&mut screen, &state.palette));
    }

    fn click(panel: &mut Instrument, state: &mut AppState, area: Rect) {
        let pos = [area.x + area.w / 2, area.y + area.h / 2];
        let down = Event::MouseDown {
            pos,
            button: MouseButton::Left,
        };
        let up = Event::MouseUp {
            pos,
            button: MouseButton::Left,
        };
        panel.handle_event(state, &down);
        panel.handle_event(state, &up);
        update(panel, state);
    }

    fn active(state: &AppState) -> OctamedicInstrument {
        return state
            .song()
            .instrument(state.active_instrument)
            .unwrap()
            .clone();
    }

    #[test]
    fn shows_the_active_instrument() {
        let (panel, _) = panel();
        assert_eq!(panel.name.text(), "bass");
        let [repeat, length] = &panel.repeats;
        assert_eq!((repeat.text(), length.text()), ("20", "40"));
        assert!(panel.types[0].is_toggled() && !panel.types[1].is_toggled());
        assert_eq!(panel.stepped[0].textbox.text(), "0");
    }

    #[test]
    fn clicking_a_box_changes_nothing() {
        let (mut panel, mut state) = panel();
        click(&mut panel, &mut state, NAME);
        click(&mut panel, &mut state, STEPPED_GROUPS[0].3);
        assert!(state.is_saved());
    }

    #[test]
    fn amiga_i_clears_and_edits_the_name() {
        let (mut panel, mut state) = panel();
        state.name_edit_request = Some(true);
        update(&mut panel, &mut state);
        assert!(panel.name.is_focused());
        assert_eq!(active(&state).name, "");
        panel.handle_event(&mut state, &Event::text('x'));
        update(&mut panel, &mut state);
        assert_eq!(active(&state).name, "x");
    }

    #[test]
    fn typing_a_name() {
        let (mut panel, mut state) = panel();
        click(&mut panel, &mut state, NAME);
        panel.handle_event(&mut state, &Event::text('!'));
        update(&mut panel, &mut state);
        assert_eq!(active(&state).name, "bass!");
        assert!(!state.is_saved());
    }

    #[test]
    fn stepping_transpose_and_finetune() {
        let (mut panel, mut state) = panel();
        let plus = panel.stepped[0].steps[1].area();
        click(&mut panel, &mut state, plus);
        assert_eq!(active(&state).transpose, 1);
        let minus = panel.stepped[1].steps[0].area();
        for _ in 0..12 {
            click(&mut panel, &mut state, minus);
        }
        assert_eq!(active(&state).finetune, MIN_FINETUNE);
        assert_eq!(panel.stepped[1].textbox.text(), "-8");
    }

    #[test]
    fn typing_a_transpose() {
        let (mut panel, mut state) = panel();
        let number = STEPPED_GROUPS[0].3;
        click(&mut panel, &mut state, number);
        panel.stepped[0].textbox.set_text("");
        for character in "-12".chars() {
            panel.handle_event(&mut state, &Event::text(character));
            update(&mut panel, &mut state);
        }
        assert_eq!(active(&state).transpose, -12);
    }

    #[test]
    fn volume_buttons() {
        let (mut panel, mut state) = panel();
        let down = panel.volume_steps[0].area();
        click(&mut panel, &mut state, down);
        assert_eq!(active(&state).volume, MAX_INSTRUMENT_VOLUME - 1);
        assert_eq!(panel.volume_slider.value(), 63);
        let up = panel.volume_steps[1].area();
        click(&mut panel, &mut state, up);
        click(&mut panel, &mut state, up);
        assert_eq!(active(&state).volume, MAX_INSTRUMENT_VOLUME);
    }
}
