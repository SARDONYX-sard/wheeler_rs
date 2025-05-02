use dashmap::DashMap;
use std::sync::LazyLock;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct KeyId(pub u32);

pub type FnMap = DashMap<KeyId, fn()>;

static KEY_FN_MAP_DOWN: LazyLock<FnMap> = LazyLock::new(FnMap::new);
static KEY_FN_MAP_UP: LazyLock<FnMap> = LazyLock::new(FnMap::new);
static KEY_FN_MAP_DOWN_GAMEPAD: LazyLock<FnMap> = LazyLock::new(FnMap::new);
static KEY_FN_MAP_UP_GAMEPAD: LazyLock<FnMap> = LazyLock::new(FnMap::new);

pub fn bind_input(key: KeyId, func: fn(), is_down: bool, is_gamepad: bool) {
    match (is_down, is_gamepad) {
        (true, true) => KEY_FN_MAP_DOWN_GAMEPAD.insert(key, func),
        (true, false) => KEY_FN_MAP_DOWN.insert(key, func),
        (false, true) => KEY_FN_MAP_UP_GAMEPAD.insert(key, func),
        (false, false) => KEY_FN_MAP_UP.insert(key, func),
    };
}

/// Is already registered key?
pub fn is_key_bound(key: KeyId) -> bool {
    KEY_FN_MAP_DOWN_GAMEPAD.contains_key(&key)
        | KEY_FN_MAP_DOWN.contains_key(&key)
        | KEY_FN_MAP_UP_GAMEPAD.contains_key(&key)
        | KEY_FN_MAP_UP.contains_key(&key)
}

pub fn dispatch(key: KeyId, is_down: bool, is_gamepad: bool) -> bool {
    let func = match (is_down, is_gamepad) {
        (true, true) => KEY_FN_MAP_DOWN_GAMEPAD.get(&key),
        (true, false) => KEY_FN_MAP_DOWN.get(&key),
        (false, true) => KEY_FN_MAP_UP_GAMEPAD.get(&key),
        (false, false) => KEY_FN_MAP_UP.get(&key),
    };

    if let Some(func) = func {
        func();
        return true;
    };
    false
}
