// most of this is for the keybinds file and help, which don't exist yet
#![allow(dead_code)]

use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::{self, Display},
};

use ggez::input::keyboard::{KeyCode, KeyMods};

use crate::{
    app::{keybinds, state::AppState},
    framework::event::Event,
};

pub const CONTROL_SYMBOL: &str = "^";
pub const ALT_SYMBOL: &str = "A-";
pub const SHIFT_SYMBOL: &str = "S-";

const LETTERS: [KeyCode; 26] = [
    KeyCode::A,
    KeyCode::B,
    KeyCode::C,
    KeyCode::D,
    KeyCode::E,
    KeyCode::F,
    KeyCode::G,
    KeyCode::H,
    KeyCode::I,
    KeyCode::J,
    KeyCode::K,
    KeyCode::L,
    KeyCode::M,
    KeyCode::N,
    KeyCode::O,
    KeyCode::P,
    KeyCode::Q,
    KeyCode::R,
    KeyCode::S,
    KeyCode::T,
    KeyCode::U,
    KeyCode::V,
    KeyCode::W,
    KeyCode::X,
    KeyCode::Y,
    KeyCode::Z,
];

const DIGITS: [KeyCode; 10] = [
    KeyCode::Key0,
    KeyCode::Key1,
    KeyCode::Key2,
    KeyCode::Key3,
    KeyCode::Key4,
    KeyCode::Key5,
    KeyCode::Key6,
    KeyCode::Key7,
    KeyCode::Key8,
    KeyCode::Key9,
];

const FUNCTION_KEYS: [KeyCode; 12] = [
    KeyCode::F1,
    KeyCode::F2,
    KeyCode::F3,
    KeyCode::F4,
    KeyCode::F5,
    KeyCode::F6,
    KeyCode::F7,
    KeyCode::F8,
    KeyCode::F9,
    KeyCode::F10,
    KeyCode::F11,
    KeyCode::F12,
];

/// Keys written as the character on them, on a US layout.
const SYMBOLS: [(KeyCode, char); 11] = [
    (KeyCode::Comma, ','),
    (KeyCode::Period, '.'),
    (KeyCode::Minus, '-'),
    (KeyCode::Equals, '='),
    (KeyCode::Slash, '/'),
    (KeyCode::Semicolon, ';'),
    (KeyCode::Apostrophe, '\''),
    (KeyCode::LBracket, '['),
    (KeyCode::RBracket, ']'),
    (KeyCode::Backslash, '\\'),
    (KeyCode::Grave, '`'),
];

/// Keys written with a name.
const NAMED: [(KeyCode, &str); 15] = [
    (KeyCode::Space, "Spc"),
    (KeyCode::Up, "Up"),
    (KeyCode::Down, "Dn"),
    (KeyCode::Left, "Lt"),
    (KeyCode::Right, "Rt"),
    (KeyCode::PageUp, "PgUp"),
    (KeyCode::PageDown, "PgDn"),
    (KeyCode::Insert, "Ins"),
    (KeyCode::Delete, "Del"),
    (KeyCode::Back, "Bksp"),
    (KeyCode::Return, "Enter"),
    (KeyCode::Escape, "Esc"),
    (KeyCode::Tab, "Tab"),
    (KeyCode::Home, "Home"),
    (KeyCode::End, "End"),
];

/// A key plus its modifiers, e.g. Ctrl+S. Keys are physical keys, so Shift+1 is `S-1`, not `!`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Key {
    code: KeyCode,
    mods: KeyMods,
}

impl Key {
    /// Only Shift, Ctrl and Alt count.
    pub fn new(code: KeyCode, mods: KeyMods) -> Self {
        let mods = mods & (KeyMods::SHIFT | KeyMods::CTRL | KeyMods::ALT);
        return Self { code, mods };
    }

    pub fn plain(code: KeyCode) -> Self {
        return Self::new(code, KeyMods::NONE);
    }

    pub fn shift(code: KeyCode) -> Self {
        return Self::new(code, KeyMods::SHIFT);
    }

    pub fn ctrl(code: KeyCode) -> Self {
        return Self::new(code, KeyMods::CTRL);
    }

    pub fn alt(code: KeyCode) -> Self {
        return Self::new(code, KeyMods::ALT);
    }

    pub fn matches(&self, code: KeyCode, mods: KeyMods) -> bool {
        return *self == Key::new(code, mods);
    }

    /// The number of an unmodified function key, e.g. 9 for F9.
    fn function_number(&self) -> Option<usize> {
        if !self.mods.is_empty() {
            return None;
        }
        return FUNCTION_KEYS
            .iter()
            .position(|&code| code == self.code)
            .map(|index| index + 1);
    }

    /// The key as written in a keybinds file, e.g. `z`, `Z` (Shift+Z), `^S`, `A-O` or `S-Tab`.
    pub fn to_text(self) -> String {
        let letter = letter(self.code);
        if self.mods == KeyMods::SHIFT {
            if let Some(letter) = letter {
                return letter.to_ascii_uppercase().to_string();
            }
        }
        let mut text = String::new();
        if self.mods.contains(KeyMods::CTRL) {
            text.push_str(CONTROL_SYMBOL);
        }
        if self.mods.contains(KeyMods::ALT) {
            text.push_str(ALT_SYMBOL);
        }
        if self.mods.contains(KeyMods::SHIFT) {
            text.push_str(SHIFT_SYMBOL);
        }
        match letter {
            Some(letter) if !self.mods.is_empty() => text.push(letter.to_ascii_uppercase()),
            _ => text.push_str(&code_text(self.code)),
        }
        return text;
    }

    /// Reads `to_text`'s format. A prefix needs a key after it, so `^` alone isn't a key.
    pub fn parse(text: &str) -> Option<Self> {
        let mut mods = KeyMods::NONE;
        let mut rest = text;
        loop {
            let (modifier, after) = if let Some(after) = rest.strip_prefix(CONTROL_SYMBOL) {
                (KeyMods::CTRL, after)
            } else if let Some(after) = rest.strip_prefix(ALT_SYMBOL) {
                (KeyMods::ALT, after)
            } else if let Some(after) = rest.strip_prefix(SHIFT_SYMBOL) {
                (KeyMods::SHIFT, after)
            } else {
                break;
            };
            if after.is_empty() {
                break;
            }
            mods |= modifier;
            rest = after;
        }
        let mut chars = rest.chars();
        if let (Some(c), None) = (chars.next(), chars.next()) {
            if c.is_ascii_uppercase() && mods.is_empty() {
                mods = KeyMods::SHIFT;
            }
        }
        return Some(Self::new(parse_code(rest)?, mods));
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return write!(f, "{}", self.to_text());
    }
}

fn letter(code: KeyCode) -> Option<char> {
    let index = LETTERS.iter().position(|&letter| letter == code)?;
    return Some((b'a' + index as u8) as char);
}

fn code_text(code: KeyCode) -> String {
    if let Some(letter) = letter(code) {
        return letter.to_string();
    }
    if let Some(digit) = DIGITS.iter().position(|&digit| digit == code) {
        return digit.to_string();
    }
    if let Some(number) = FUNCTION_KEYS.iter().position(|&key| key == code) {
        return format!("F{}", number + 1);
    }
    if let Some(&(_, symbol)) = SYMBOLS.iter().find(|(key, _)| *key == code) {
        return symbol.to_string();
    }
    if let Some(&(_, name)) = NAMED.iter().find(|(key, _)| *key == code) {
        return name.to_string();
    }
    return format!("{code:?}");
}

fn parse_code(text: &str) -> Option<KeyCode> {
    if let Some(&(code, _)) = NAMED.iter().find(|(_, name)| *name == text) {
        return Some(code);
    }
    let mut chars = text.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_lowercase() {
            return Some(LETTERS[(c as u8 - b'a') as usize]);
        }
        if let Some(digit) = c.to_digit(10) {
            return Some(DIGITS[digit as usize]);
        }
        return SYMBOLS
            .iter()
            .find(|(_, symbol)| *symbol == c)
            .map(|&(code, _)| code);
    }
    let number: usize = text.strip_prefix('F')?.parse().ok()?;
    return FUNCTION_KEYS.get(number.checked_sub(1)?).copied();
}

pub struct HelpEntry {
    pub key: String,
    pub description: &'static str,
    /// Set when the entry can be rebound.
    pub binding: Option<BindingInfo>,
}

#[derive(Clone)]
pub struct BindingInfo {
    /// e.g. `main.quit`
    pub id: String,
    pub defaults: Vec<Key>,
}

pub trait Action: Copy + PartialEq + fmt::Debug {
    /// The variant name in snake_case. Override it for variants with values.
    fn id(self) -> String {
        return snake_case(&format!("{self:?}"));
    }

    /// Whether holding the key does it again at the key repeat rate. Toggles shouldn't, or
    /// holding one flickers it.
    fn repeats(self) -> bool {
        return true;
    }
}

pub fn snake_case(name: &str) -> String {
    let mut text = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                text.push('_');
            }
            text.push(c.to_ascii_lowercase());
        } else {
            text.push(c);
        }
    }
    return text;
}

pub struct HelpSection {
    pub title: &'static str,
    pub entries: Vec<HelpEntry>,
}

impl HelpSection {
    pub fn new(title: &'static str) -> Self {
        return Self {
            title,
            entries: Vec::new(),
        };
    }

    pub fn entries(mut self, entries: Vec<HelpEntry>) -> Self {
        self.entries.extend(entries);
        return self;
    }
}

#[derive(Clone, Copy)]
pub struct Binding<A> {
    pub key: Key,
    pub action: A,
    pub label: &'static str,
}

/// Maps keys to a widget's actions. The same table drives event handling (`lookup`) and what
/// gets displayed (`key_for`, `help_entries`), so the two can never drift apart. Overrides set in
/// `keybinds` replace the keys passed to `bind`.
pub struct Keymap<A> {
    /// Prefix of the action ids, e.g. `main`.
    scope: &'static str,
    bindings: Vec<Binding<A>>,
}

impl<A: Action> Keymap<A> {
    pub fn new(scope: &'static str) -> Self {
        return Self {
            scope,
            bindings: Vec::new(),
        };
    }

    pub fn bind(mut self, key: Key, action: A, label: &'static str) -> Self {
        self.bindings.push(Binding { key, action, label });
        return self;
    }

    pub fn id(&self, action: A) -> String {
        return format!("{}.{}", self.scope, action.id());
    }

    /// The bindings with the overrides applied.
    pub fn bindings(&self) -> Cow<'_, [Binding<A>]> {
        return keybinds::with_overrides(|overrides| self.bindings_with(overrides));
    }

    /// The defaults, e.g. to change a label.
    pub fn default_bindings_mut(&mut self) -> &mut [Binding<A>] {
        return &mut self.bindings;
    }

    /// The action bound to the pressed key, if any, and if it repeats when the key is held.
    pub fn lookup(&self, event: &Event) -> Option<A> {
        let Event::Key { key, mods, repeat } = *event else {
            return None;
        };
        return self
            .bindings()
            .iter()
            .find(|binding| binding.key.matches(key, mods))
            .map(|binding| binding.action)
            .filter(|action| !repeat || action.repeats());
    }

    /// The first key bound to `action`, for showing it next to the thing it controls.
    pub fn key_for(&self, action: A) -> Option<Key> {
        return self
            .bindings()
            .iter()
            .find(|binding| binding.action == action)
            .map(|binding| binding.key);
    }

    /// The key as shown on screen, or `--` when unbound.
    pub fn key_text(&self, action: A) -> String {
        return self
            .key_for(action)
            .map_or_else(|| NO_KEY.to_owned(), |key| key.to_string());
    }

    fn bindings_with(&self, overrides: &HashMap<String, Vec<Key>>) -> Cow<'_, [Binding<A>]> {
        if overrides.is_empty() {
            return Cow::Borrowed(self.bindings.as_slice());
        }
        let mut bindings = Vec::new();
        let mut overridden: Vec<A> = Vec::new();
        for binding in &self.bindings {
            let Some(keys) = overrides.get(&self.id(binding.action)) else {
                bindings.push(*binding);
                continue;
            };
            if !overridden.contains(&binding.action) {
                overridden.push(binding.action);
                bindings.extend(keys.iter().map(|&key| Binding { key, ..*binding }));
            }
        }
        return Cow::Owned(bindings);
    }

    /// One entry per action.
    pub fn help_entries(&self, describe: impl Fn(A) -> &'static str) -> Vec<HelpEntry> {
        let mut actions: Vec<A> = Vec::new();
        for binding in &self.bindings {
            if !actions.contains(&binding.action) {
                actions.push(binding.action);
            }
        }
        let bindings = self.bindings();
        return actions
            .into_iter()
            .map(|action| {
                let keys_of = |bindings: &[Binding<A>]| -> Vec<Key> {
                    return bindings
                        .iter()
                        .filter(|binding| binding.action == action)
                        .map(|binding| binding.key)
                        .collect();
                };
                let keys = keys_of(&bindings);
                return HelpEntry {
                    key: if keys.is_empty() {
                        UNBOUND.to_owned()
                    } else {
                        join_keys(&keys)
                    },
                    description: describe(action),
                    binding: Some(BindingInfo {
                        id: self.id(action),
                        defaults: keys_of(&self.bindings),
                    }),
                };
            })
            .collect();
    }
}

pub const UNBOUND: &str = "none";

/// Two dashes, since `-` is a key itself.
const NO_KEY: &str = "--";

/// Joins keys with "/", collapsing runs of three or more consecutive function keys into a
/// range, e.g. "Fn9-12" instead of "F9/F10/F11/F12".
pub fn join_keys(keys: &[Key]) -> String {
    let mut parts = Vec::new();
    let mut start = 0;
    while start < keys.len() {
        let mut end = start;
        while let (Some(a), Some(b)) = (
            keys[end].function_number(),
            keys.get(end + 1).and_then(Key::function_number),
        ) {
            if b != a + 1 {
                break;
            }
            end += 1;
        }
        if end - start >= 2 {
            let first = keys[start].function_number().unwrap();
            let last = keys[end].function_number().unwrap();
            parts.push(format!("Fn{first}-{last}"));
        } else {
            parts.extend(keys[start..=end].iter().map(Key::to_string));
        }
        start = end + 1;
    }
    return parts.join("/");
}

/// Implemented by widgets that react to keys through a `Keymap`. Provides the shared glue
/// (event -> lookup -> perform) so each widget only has to say what its actions do.
pub trait KeyActions {
    type Action: Action;

    /// The keymap that is currently active, e.g. a different one per mode.
    fn keymap(&self) -> &Keymap<Self::Action>;

    fn perform(&mut self, action: Self::Action, state: &mut AppState);

    fn describe(action: Self::Action) -> &'static str;

    fn help_section(&self, title: &'static str) -> HelpSection {
        return HelpSection::new(title).entries(self.keymap().help_entries(Self::describe));
    }

    /// Runs the action bound to `event`, returning whether it was handled.
    fn dispatch(&mut self, state: &mut AppState, event: &Event) -> bool {
        let Some(action) = self.keymap().lookup(event) else {
            return false;
        };
        self.perform(action, state);
        return true;
    }
}

#[cfg(test)]
impl Action for i32 {}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(key: KeyCode, mods: KeyMods) -> Event {
        return Event::Key {
            key,
            mods,
            repeat: false,
        };
    }

    #[test]
    fn keys_read_back_from_text() {
        let keys = [
            Key::ctrl(KeyCode::S),
            Key::alt(KeyCode::O),
            Key::shift(KeyCode::Z),
            Key::plain(KeyCode::Z),
            Key::plain(KeyCode::Space),
            Key::plain(KeyCode::Minus),
            Key::alt(KeyCode::Minus),
            Key::shift(KeyCode::Key1),
            Key::new(KeyCode::S, KeyMods::CTRL | KeyMods::SHIFT),
            Key::shift(KeyCode::Tab),
            Key::plain(KeyCode::Up),
            Key::plain(KeyCode::F12),
            Key::alt(KeyCode::Delete),
        ];
        for key in keys {
            assert_eq!(Key::parse(&key.to_text()), Some(key), "{}", key.to_text());
        }
        assert_eq!(Key::shift(KeyCode::Z).to_text(), "Z");
        assert_eq!(Key::plain(KeyCode::Z).to_text(), "z");
        assert_eq!(Key::ctrl(KeyCode::S).to_text(), "^S");
        assert_eq!(Key::shift(KeyCode::Key1).to_text(), "S-1");
        assert_eq!(Key::parse("^"), None);
        assert_eq!(Key::parse("Fx"), None);
        assert_eq!(Key::parse("F13"), None);
        assert_eq!(Key::parse(""), None);
    }

    #[test]
    fn the_windows_key_does_not_count() {
        let key = Key::ctrl(KeyCode::S);
        assert!(key.matches(KeyCode::S, KeyMods::CTRL | KeyMods::LOGO));
        assert!(!key.matches(KeyCode::S, KeyMods::NONE));
    }

    #[test]
    fn lookup_finds_the_bound_action() {
        let keymap = Keymap::new("test")
            .bind(Key::plain(KeyCode::Up), 0, "Row")
            .bind(Key::shift(KeyCode::Up), 1, "Row");
        assert_eq!(keymap.lookup(&press(KeyCode::Up, KeyMods::NONE)), Some(0));
        assert_eq!(keymap.lookup(&press(KeyCode::Up, KeyMods::SHIFT)), Some(1));
        assert_eq!(keymap.lookup(&press(KeyCode::Down, KeyMods::NONE)), None);
        assert_eq!(keymap.lookup(&Event::Text('x')), None);
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Toggle {
        Flip,
    }

    impl Action for Toggle {
        fn repeats(self) -> bool {
            return false;
        }
    }

    #[test]
    fn held_keys_repeat_only_actions_that_repeat() {
        let held = Event::Key {
            key: KeyCode::Up,
            mods: KeyMods::NONE,
            repeat: true,
        };
        let moves = Keymap::new("test").bind(Key::plain(KeyCode::Up), 0, "Row");
        assert_eq!(moves.lookup(&held), Some(0));
        let toggles = Keymap::new("test").bind(Key::plain(KeyCode::Up), Toggle::Flip, "Flip");
        assert_eq!(toggles.lookup(&held), None);
        assert_eq!(
            toggles.lookup(&press(KeyCode::Up, KeyMods::NONE)),
            Some(Toggle::Flip)
        );
    }

    #[test]
    fn overrides_replace_every_default_key_of_their_action() {
        let keymap = Keymap::new("test")
            .bind(Key::plain(KeyCode::Up), 0, "Row")
            .bind(Key::plain(KeyCode::K), 0, "Row")
            .bind(Key::plain(KeyCode::Down), 1, "Row");
        let overrides = HashMap::from([
            ("test.0".to_owned(), vec![Key::plain(KeyCode::W)]),
            ("test.1".to_owned(), vec![]),
        ]);
        let bindings = keymap.bindings_with(&overrides);
        let keys: Vec<(Key, i32)> = bindings.iter().map(|b| (b.key, b.action)).collect();
        assert_eq!(keys, vec![(Key::plain(KeyCode::W), 0)]);
        assert_eq!(keymap.bindings_with(&HashMap::new()).len(), 3);
    }

    #[test]
    fn action_ids_are_snake_case() {
        assert_eq!(snake_case("ToggleEdit"), "toggle_edit");
        assert_eq!(snake_case("Up"), "up");
    }

    #[test]
    fn help_merges_the_keys_of_an_action() {
        let keymap = (9..=12)
            .fold(Keymap::new("test"), |keymap, n| {
                keymap.bind(Key::plain(FUNCTION_KEYS[n - 1]), 0, "JMP")
            })
            .bind(Key::plain(KeyCode::Delete), 1, "Del")
            .bind(Key::plain(KeyCode::Back), 1, "Del");
        let entries = keymap.help_entries(|_| "");
        let keys: Vec<&str> = entries.iter().map(|entry| entry.key.as_str()).collect();
        assert_eq!(keys, vec!["Fn9-12", "Del/Bksp"]);
        assert_eq!(entries[0].binding.as_ref().unwrap().id, "test.0");
    }
}
