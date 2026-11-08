use rustuition::{
    filters::{
        AdjustableFilter, ChromaticAberration, ColorAdjust, Curvature, Noise, Scanlines, VhsWobble,
        Vignette,
    },
    gadgets::{
        dialog::{Field, NumberField},
        reorderable_list::{ListEvent, ListItem, ReorderableList},
    },
    target::Framebuffer,
};

use crate::{
    app::{config::FilterConfig, state::AppState},
    framework::widgets::Dialog,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterKind {
    Scanlines,
    ChromaticAberration,
    VhsWobble,
    ColorAdjust,
    Curvature,
    Vignette,
    Noise,
}

impl FilterKind {
    pub const ALL: [FilterKind; 7] = [
        FilterKind::Scanlines,
        FilterKind::ChromaticAberration,
        FilterKind::VhsWobble,
        FilterKind::ColorAdjust,
        FilterKind::Curvature,
        FilterKind::Vignette,
        FilterKind::Noise,
    ];

    pub fn name(self) -> &'static str {
        return match self {
            FilterKind::Scanlines => "Scanlines",
            FilterKind::ChromaticAberration => "Chroma",
            FilterKind::VhsWobble => "VHS wobble",
            FilterKind::ColorAdjust => "Color",
            FilterKind::Curvature => "Curvature",
            FilterKind::Vignette => "Vignette",
            FilterKind::Noise => "Noise",
        };
    }

    // what it's called in config.toml
    fn key(self) -> &'static str {
        return match self {
            FilterKind::Scanlines => "scanlines",
            FilterKind::ChromaticAberration => "chroma",
            FilterKind::VhsWobble => "vhs_wobble",
            FilterKind::ColorAdjust => "color",
            FilterKind::Curvature => "curvature",
            FilterKind::Vignette => "vignette",
            FilterKind::Noise => "noise",
        };
    }

    fn create(self) -> Box<dyn AdjustableFilter> {
        return match self {
            FilterKind::Scanlines => Box::new(Scanlines::default()),
            FilterKind::ChromaticAberration => Box::new(ChromaticAberration::default()),
            FilterKind::VhsWobble => Box::new(VhsWobble::default()),
            FilterKind::ColorAdjust => Box::new(ColorAdjust::default()),
            FilterKind::Curvature => Box::new(Curvature::default()),
            FilterKind::Vignette => Box::new(Vignette::default()),
            FilterKind::Noise => Box::new(Noise::default()),
        };
    }
}

pub struct ScreenFilter {
    pub kind: FilterKind,
    pub enabled: bool,
    pub filter: Box<dyn AdjustableFilter>,
}

/// What the screen goes through before it's shown, in order.
#[derive(Default)]
pub struct ScreenFilters {
    filters: Vec<ScreenFilter>,
}

impl ScreenFilters {
    /// The filters `config.toml` lists. Kinds it doesn't know are left out, and settings it
    /// doesn't have keep their defaults.
    pub fn from_config(configs: &[FilterConfig]) -> Self {
        let mut filters = Self::default();
        for config in configs {
            let Some(kind) = FilterKind::ALL
                .into_iter()
                .find(|kind| kind.key() == config.kind)
            else {
                continue;
            };
            let mut filter = kind.create();
            for (index, param) in filter.params().iter().enumerate() {
                if let Some(&value) = config.settings.get(&setting_key(param.name)) {
                    filter.set_param(index, value);
                }
            }
            filters.filters.push(ScreenFilter {
                kind,
                enabled: config.enabled,
                filter,
            });
        }
        return filters;
    }

    pub fn to_config(&self) -> Vec<FilterConfig> {
        return self
            .filters
            .iter()
            .map(|filter| FilterConfig {
                kind: filter.kind.key().to_owned(),
                enabled: filter.enabled,
                settings: filter
                    .filter
                    .params()
                    .iter()
                    .enumerate()
                    .map(|(index, param)| (setting_key(param.name), filter.filter.param(index)))
                    .collect(),
            })
            .collect();
    }

    pub fn get(&self, index: usize) -> Option<&ScreenFilter> {
        return self.filters.get(index);
    }

    pub fn add(&mut self, kind: FilterKind) {
        self.filters.push(ScreenFilter {
            kind,
            enabled: true,
            filter: kind.create(),
        });
    }

    pub fn is_active(&self) -> bool {
        return self.filters.iter().any(|filter| filter.enabled);
    }

    /// Runs the filters over `frame` in order. The first one that bends the picture gets it
    /// `finer` times as big across and down, and so do the ones after it.
    pub fn apply(&self, mut frame: Framebuffer, finer: [i32; 2]) -> Framebuffer {
        let mut scaled = false;
        for filter in self.filters.iter().filter(|filter| filter.enabled) {
            if !scaled && filter.filter.wants_finer_pixels() {
                frame = frame.scaled(finer);
                scaled = true;
            }
            filter.filter.apply(&mut frame);
        }
        return frame;
    }

    pub fn advance_frame(&mut self) {
        for filter in &mut self.filters {
            filter.filter.advance_frame();
        }
    }

    pub fn set_param(&mut self, index: usize, param: usize, value: f32) {
        if let Some(filter) = self.filters.get_mut(index) {
            filter.filter.set_param(param, value);
        }
    }

    fn items(&self) -> Vec<ListItem> {
        return self
            .filters
            .iter()
            .map(|filter| ListItem {
                choice: FilterKind::ALL
                    .iter()
                    .position(|kind| *kind == filter.kind)
                    .unwrap_or(0),
                enabled: filter.enabled,
            })
            .collect();
    }

    fn apply_event(&mut self, event: ListEvent) {
        match event {
            ListEvent::Add => self.add(FilterKind::ALL[0]),
            ListEvent::Remove(index) if index < self.filters.len() => {
                self.filters.remove(index);
            }
            ListEvent::Move { from, to } if from.max(to) < self.filters.len() => {
                self.filters.swap(from, to)
            }
            ListEvent::Choose { item, choice } => {
                if let (Some(filter), Some(&kind)) =
                    (self.filters.get_mut(item), FilterKind::ALL.get(choice))
                {
                    filter.kind = kind;
                    filter.filter = kind.create();
                }
            }
            ListEvent::Enable { item, enabled } => {
                if let Some(filter) = self.filters.get_mut(item) {
                    filter.enabled = enabled;
                }
            }
            ListEvent::Remove(_) | ListEvent::Move { .. } | ListEvent::Edit(_) => {}
        }
    }
}

/// The filters in order, changed as they're edited, so there's only a Close button.
pub fn open_filters_dialog(state: &mut AppState) {
    let names = FilterKind::ALL.map(FilterKind::name);
    let list = ReorderableList::new(&names)
        .items(&state.screen_filters.items())
        .can_add(true)
        .can_remove(true)
        .editable(true)
        .can_toggle_all(true)
        .on_event(|state: &mut AppState, event| match event {
            ListEvent::Edit(index) => open_params_dialog(state, index),
            event => {
                state.screen_filters.apply_event(event);
                state.save_screen_filters();
            }
        });
    let mut dialog = Dialog::new("Screen filters").submit_button("Close");
    dialog.add_field(list);
    state.open_dialog(dialog);
}

fn open_params_dialog(state: &mut AppState, index: usize) {
    let Some(filter) = state.screen_filters.get(index) else {
        return;
    };
    let repeat_delay = state.video().clock().key_repeat_delay;
    let mut dialog = Dialog::new(filter.kind.name()).submit_button("OK");
    let fields: Vec<Field<f32>> = filter
        .filter
        .params()
        .iter()
        .enumerate()
        .map(|(param_index, param)| {
            let value = filter.filter.param(param_index);
            let field = NumberField::new(param.name, param.min, param.max, param.step)
                .value(value)
                .repeat_delay(repeat_delay);
            dialog.add_field(field)
        })
        .collect();
    let dialog = dialog.on_change(move |state, values| {
        for (param, field) in fields.iter().enumerate() {
            state
                .screen_filters
                .set_param(index, param, values.get(*field));
        }
        state.save_screen_filters();
    });
    state.open_dialog(dialog);
}

// "Wavelength" -> "wavelength", for config.toml
fn setting_key(name: &str) -> String {
    return name.to_ascii_lowercase().replace(' ', "_");
}

#[cfg(test)]
mod tests {
    use rustuition::target::{RenderTarget, Rgba};

    use super::*;

    #[test]
    fn follows_the_list() {
        let mut filters = ScreenFilters::default();
        filters.apply_event(ListEvent::Add);
        filters.apply_event(ListEvent::Add);
        filters.apply_event(ListEvent::Choose { item: 1, choice: 2 });
        filters.apply_event(ListEvent::Move { from: 1, to: 0 });
        filters.apply_event(ListEvent::Enable {
            item: 1,
            enabled: false,
        });
        let kinds: Vec<(FilterKind, bool)> = filters
            .filters
            .iter()
            .map(|filter| (filter.kind, filter.enabled))
            .collect();
        assert_eq!(
            kinds,
            [
                (FilterKind::VhsWobble, true),
                (FilterKind::Scanlines, false)
            ]
        );
        assert_eq!(
            filters.items(),
            [
                ListItem {
                    choice: 2,
                    enabled: true
                },
                ListItem {
                    choice: 0,
                    enabled: false
                }
            ]
        );
        filters.apply_event(ListEvent::Remove(0));
        assert!(!filters.is_active());
    }

    #[test]
    fn kept_in_the_config() {
        let mut filters = ScreenFilters::default();
        filters.add(FilterKind::Vignette);
        filters.add(FilterKind::Curvature);
        filters.set_param(0, 2, 0.75);
        filters.apply_event(ListEvent::Enable {
            item: 1,
            enabled: false,
        });
        let config = filters.to_config();
        assert_eq!(config[0].kind, "vignette");
        assert_eq!(config[0].settings["roundness"], 0.75);
        let loaded = ScreenFilters::from_config(&config);
        assert_eq!(loaded.items(), filters.items());
        assert_eq!(loaded.get(0).unwrap().filter.param(2), 0.75);
    }

    #[test]
    fn unknown_kinds_and_settings_are_left_out() {
        let config: crate::app::config::Config = toml::from_str(
            "[[filters]]\nkind = \"fisheye\"\n\n[[filters]]\nkind = \"noise\"\nsparkle = 3.0\n",
        )
        .unwrap();
        let filters = ScreenFilters::from_config(&config.filters);
        assert_eq!(filters.items(), [ListItem::new(6)]);
        assert_eq!(filters.get(0).unwrap().filter.param(0), 0.08);
    }

    #[test]
    fn only_enabled_filters_apply() {
        let mut filters = ScreenFilters::default();
        filters.add(FilterKind::Scanlines);
        filters.set_param(0, 0, 1.0);
        let grey = || {
            let mut frame = Framebuffer::new(1, 2);
            frame.fill(frame.bounds(), Rgba::rgb(200, 200, 200));
            frame
        };
        let frame = filters.apply(grey(), [2, 2]);
        assert_eq!(frame.pixel(0, 1), Some([0, 0, 0]));
        filters.apply_event(ListEvent::Enable {
            item: 0,
            enabled: false,
        });
        let frame = filters.apply(grey(), [2, 2]);
        assert_eq!(frame.pixel(0, 1), Some([200, 200, 200]));
    }

    #[test]
    fn enlarged_from_the_first_filter_that_bends_the_picture() {
        let mut filters = ScreenFilters::default();
        filters.add(FilterKind::Scanlines);
        let frame = filters.apply(Framebuffer::new(4, 4), [1, 2]);
        assert_eq!((frame.width(), frame.height()), (4, 4));
        filters.add(FilterKind::Curvature);
        filters.add(FilterKind::Noise);
        let frame = filters.apply(Framebuffer::new(4, 4), [1, 2]);
        assert_eq!((frame.width(), frame.height()), (4, 8));
    }
}
