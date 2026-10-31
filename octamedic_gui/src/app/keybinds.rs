// nothing loads a keybinds file yet
#![allow(dead_code)]

use std::{
    collections::HashMap,
    sync::{LazyLock, RwLock},
};

use crate::framework::keymap::Key;

/// Keys that replace an action's defaults, by action id, e.g. `main.quit`.
static OVERRIDES: LazyLock<RwLock<HashMap<String, Vec<Key>>>> = LazyLock::new(Default::default);

pub fn with_overrides<T>(f: impl FnOnce(&HashMap<String, Vec<Key>>) -> T) -> T {
    let overrides = OVERRIDES.read().unwrap_or_else(|e| e.into_inner());
    return f(&overrides);
}

/// `None` puts the action's default keys back.
pub fn set(id: &str, keys: Option<Vec<Key>>) {
    let mut overrides = OVERRIDES.write().unwrap_or_else(|e| e.into_inner());
    match keys {
        Some(keys) => overrides.insert(id.to_owned(), keys),
        None => overrides.remove(id),
    };
}
