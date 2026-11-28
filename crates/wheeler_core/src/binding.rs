//! Key combinations: chords of up to [`MAX_CHORD_KEYS`] keys, pressed once
//! or tapped up to [`MAX_TAPS`] times, and the detector that turns key
//! presses into triggered actions.
//!
//! Key ids follow [`crate::input_id`] (keyboard, mouse and gamepad share one
//! id space), so one chord may mix devices.
//!
//! INI syntax: keys joined by `+` (decimal or `0x` hex), optionally followed
//! by `*<taps>`; `0` or empty means unbound. Plain numbers (the C++ format)
//! are single-key, single-tap combos.
//!
//! ```text
//! toggleWheel = 58          ; caps lock
//! addWheel = 0x2A+0x31      ; left shift + N
//! nextWheel = 281*2         ; double-tap right trigger
//! ```

use crate::input_id::KeyId;

pub const MAX_CHORD_KEYS: usize = 4;
pub const MAX_TAPS: u8 = 3;

/// A chord (set of keys held together) plus how many times it is tapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    /// Sorted, unique; only the first `len` are used.
    keys: [KeyId; MAX_CHORD_KEYS],
    len: u8,
    taps: u8,
}

impl Default for KeyCombo {
    fn default() -> Self {
        Self::NONE
    }
}

impl KeyCombo {
    /// Unbound.
    pub const NONE: Self = Self {
        keys: [0; MAX_CHORD_KEYS],
        len: 0,
        taps: 1,
    };

    /// One key, one tap (`0` = unbound, like the C++ INI).
    pub const fn single(key: KeyId) -> Self {
        if key == 0 {
            return Self::NONE;
        }
        Self {
            keys: [key, 0, 0, 0],
            len: 1,
            taps: 1,
        }
    }

    /// Two keys held together, one tap (for `const` defaults).
    pub const fn chord2(a: KeyId, b: KeyId) -> Self {
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        Self {
            keys: [lo, hi, 0, 0],
            len: 2,
            taps: 1,
        }
    }

    /// `None` for too many keys or an invalid tap count. Duplicates and `0`
    /// ids are dropped; no keys gives [`KeyCombo::NONE`].
    pub fn new(keys: &[KeyId], taps: u8) -> Option<Self> {
        if !(1..=MAX_TAPS).contains(&taps) {
            return None;
        }
        let mut sorted: Vec<KeyId> = keys.iter().copied().filter(|&k| k != 0).collect();
        sorted.sort_unstable();
        sorted.dedup();
        if sorted.len() > MAX_CHORD_KEYS {
            return None;
        }
        let mut combo = Self { taps, ..Self::NONE };
        combo.keys[..sorted.len()].copy_from_slice(&sorted);
        combo.len = sorted.len() as u8;
        Some(combo)
    }

    pub fn keys(&self) -> &[KeyId] {
        &self.keys[..self.len as usize]
    }

    pub fn taps(&self) -> u8 {
        self.taps
    }

    pub fn with_taps(self, taps: u8) -> Self {
        Self {
            taps: taps.clamp(1, MAX_TAPS),
            ..self
        }
    }

    pub fn is_none(&self) -> bool {
        self.len == 0
    }

    pub fn contains(&self, key: KeyId) -> bool {
        self.keys().contains(&key)
    }

    /// Parses the INI syntax (see the module docs).
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let (chord, taps) = match text.split_once('*') {
            Some((chord, taps)) => (chord, taps.trim().parse().ok()?),
            None => (text, 1),
        };
        let mut keys = Vec::new();
        for part in chord.split('+') {
            let part = part.trim();
            if part.is_empty() {
                if chord.trim().is_empty() {
                    break;
                }
                return None;
            }
            let key = match part.strip_prefix("0x").or_else(|| part.strip_prefix("0X")) {
                Some(hex) => KeyId::from_str_radix(hex, 16).ok()?,
                None => part.parse().ok()?,
            };
            keys.push(key);
        }
        Self::new(&keys, taps)
    }

    /// The INI form, decimal ids (`0` when unbound).
    pub fn to_ini_string(&self) -> String {
        if self.is_none() {
            return "0".to_owned();
        }
        let keys: Vec<String> = self.keys().iter().map(ToString::to_string).collect();
        let mut s = keys.join("+");
        if self.taps > 1 {
            s += &format!("*{}", self.taps);
        }
        s
    }
}

/// When a binding fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    /// When the chord completes (after the last tap).
    Press,
    /// When a key of the chord is released after the chord fired.
    Release,
}

#[derive(Debug, Clone, Copy)]
struct Binding<A> {
    combo: KeyCombo,
    edge: Edge,
    action: A,
}

/// The chord of an ongoing tap sequence.
#[derive(Debug, Clone, Copy)]
struct TapState {
    chord: KeyCombo,
    count: u8,
    /// Time of the last completion of the chord.
    last: f32,
    /// Fires `count` taps at this time unless another tap comes first.
    deadline: Option<f32>,
    /// The chord is still held (completed and not released yet).
    held: bool,
    /// The press of the current tap fired; release bindings may fire.
    press_fired: bool,
    /// Released while the press was still deferred.
    release_pending: bool,
}

/// Turns key presses into actions.
///
/// - A chord completes when its last key goes down while the others are
///   held. When several bound chords complete at once, only the largest ones
///   count (`Shift+E` wins over `E`).
/// - Taps: completing the same chord again within the tap window counts as
///   another tap. A press binding fires right away when no binding of the
///   same chord needs more taps; otherwise it waits until the window passes
///   ([`ComboDetector::tick`]).
/// - Release bindings fire when the chord is released, for the tap count of
///   the press that fired (after it, if the press was deferred).
#[derive(Debug, Clone)]
pub struct ComboDetector<A> {
    bindings: Vec<Binding<A>>,
    held: Vec<KeyId>,
    taps: Vec<TapState>,
    /// Seconds between taps.
    pub tap_window: f32,
}

impl<A: Copy> Default for ComboDetector<A> {
    fn default() -> Self {
        Self::new(0.25)
    }
}

impl<A: Copy> ComboDetector<A> {
    pub fn new(tap_window: f32) -> Self {
        Self {
            bindings: Vec::new(),
            held: Vec::new(),
            taps: Vec::new(),
            tap_window,
        }
    }

    /// Adds a binding; unbound combos are ignored.
    pub fn bind(&mut self, combo: KeyCombo, edge: Edge, action: A) {
        if !combo.is_none() {
            self.bindings.push(Binding {
                combo,
                edge,
                action,
            });
        }
    }

    /// Removes the bindings and forgets every key state.
    pub fn clear(&mut self) {
        self.bindings.clear();
        self.held.clear();
        self.taps.clear();
    }

    /// Forgets held keys and tap sequences (keys released while input went
    /// elsewhere would otherwise stay "held").
    pub fn reset_keys(&mut self) {
        self.held.clear();
        self.taps.clear();
    }

    /// Whether `key` is part of any binding.
    pub fn is_key_bound(&self, key: KeyId) -> bool {
        self.bindings.iter().any(|b| b.combo.contains(key))
    }

    fn chord_of(combo: &KeyCombo) -> KeyCombo {
        combo.with_taps(1)
    }

    fn max_taps(&self, chord: &KeyCombo) -> u8 {
        self.bindings
            .iter()
            .filter(|b| b.edge == Edge::Press && Self::chord_of(&b.combo) == *chord)
            .map(|b| b.combo.taps)
            .max()
            .unwrap_or(0)
    }

    fn fire(&self, chord: &KeyCombo, taps: u8, edge: Edge, out: &mut Vec<A>) {
        out.extend(
            self.bindings
                .iter()
                .filter(|b| b.edge == edge && b.combo == chord.with_taps(taps))
                .map(|b| b.action),
        );
    }

    fn fire_press(&mut self, i: usize, out: &mut Vec<A>) {
        let state = self.taps[i];
        self.fire(&state.chord, state.count, Edge::Press, out);
        let state = &mut self.taps[i];
        state.deadline = None;
        state.press_fired = true;
        if state.release_pending {
            state.release_pending = false;
            let (chord, count) = (state.chord, state.count);
            self.fire(&chord, count, Edge::Release, out);
        }
    }

    /// Feeds one key edge at time `now` (seconds); returns fired actions.
    pub fn on_key(&mut self, key: KeyId, down: bool, now: f32) -> Vec<A> {
        let mut out = self.tick(now);
        if down {
            if self.held.contains(&key) {
                return out; // repeat
            }
            self.held.push(key);
            self.on_press(key, now, &mut out);
        } else {
            self.held.retain(|&k| k != key);
            self.on_release(key, &mut out);
        }
        out
    }

    fn on_press(&mut self, key: KeyId, now: f32, out: &mut Vec<A>) {
        // Bound chords completed by this key.
        let mut completed: Vec<KeyCombo> = Vec::new();
        for b in &self.bindings {
            let chord = Self::chord_of(&b.combo);
            if chord.contains(key)
                && chord.keys().iter().all(|k| self.held.contains(k))
                && !completed.contains(&chord)
            {
                completed.push(chord);
            }
        }
        // Keep only the largest: drop chords that are subsets of another.
        let largest: Vec<KeyCombo> = completed
            .iter()
            .filter(|c| {
                !completed
                    .iter()
                    .any(|o| o.len > c.len && c.keys().iter().all(|k| o.contains(*k)))
            })
            .copied()
            .collect();

        for chord in largest {
            let window = self.tap_window;
            let i = match self.taps.iter().position(|t| t.chord == chord) {
                Some(i) => i,
                None => {
                    self.taps.push(TapState {
                        chord,
                        count: 0,
                        last: f32::NEG_INFINITY,
                        deadline: None,
                        held: false,
                        press_fired: false,
                        release_pending: false,
                    });
                    self.taps.len() - 1
                }
            };
            let state = &mut self.taps[i];
            if state.count == 0 || now - state.last > window {
                state.count = 0;
            }
            state.count += 1;
            state.last = now;
            state.held = true;
            state.press_fired = false;
            state.release_pending = false;

            let count = state.count;
            if count >= self.max_taps(&chord) {
                self.fire_press(i, out);
                self.taps[i].count = 0; // sequence complete
            } else {
                self.taps[i].deadline = Some(now + window);
            }
        }
    }

    fn on_release(&mut self, key: KeyId, out: &mut Vec<A>) {
        for i in 0..self.taps.len() {
            let state = self.taps[i];
            if !state.held || !state.chord.contains(key) {
                continue;
            }
            self.taps[i].held = false;
            if state.press_fired {
                // `count` was reset after a complete sequence; recover it.
                let count = if state.count == 0 {
                    self.max_taps(&state.chord)
                } else {
                    state.count
                };
                self.fire(&state.chord, count, Edge::Release, out);
            } else if state.deadline.is_some() {
                self.taps[i].release_pending = true;
            }
        }
    }

    /// Fires deferred presses whose tap window passed. Call every frame.
    pub fn tick(&mut self, now: f32) -> Vec<A> {
        let mut out = Vec::new();
        for i in 0..self.taps.len() {
            if self.taps[i].deadline.is_some_and(|d| now >= d) {
                self.fire_press(i, &mut out);
                self.taps[i].count = 0;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    const SHIFT: KeyId = 0x2A;
    const E: KeyId = 0x12;
    const Q: KeyId = 0x10;

    fn combo(keys: &[KeyId], taps: u8) -> KeyCombo {
        KeyCombo::new(keys, taps).unwrap()
    }

    #[test]
    fn parses_and_formats() {
        assert_eq!(KeyCombo::parse("58"), Some(KeyCombo::single(58)));
        assert_eq!(KeyCombo::parse("0"), Some(KeyCombo::NONE));
        assert_eq!(KeyCombo::parse(""), Some(KeyCombo::NONE));
        assert_eq!(KeyCombo::parse("0x2A + 0x12"), Some(combo(&[E, SHIFT], 1)));
        assert_eq!(KeyCombo::parse("281*2"), Some(combo(&[281], 2)));
        assert_eq!(KeyCombo::parse("1+2+3+4+5"), None);
        assert_eq!(KeyCombo::parse("1*4"), None);
        assert_eq!(KeyCombo::parse("1+"), None);
        assert_eq!(KeyCombo::parse("x"), None);
        assert_eq!(combo(&[SHIFT, E], 2).to_ini_string(), "18+42*2");
        assert_eq!(KeyCombo::NONE.to_ini_string(), "0");
        for s in ["18+42*2", "58", "0"] {
            assert_eq!(KeyCombo::parse(s).unwrap().to_ini_string(), s);
        }
    }

    #[test]
    fn single_key_fires_on_press_and_release() {
        let mut d = ComboDetector::new(0.25);
        d.bind(KeyCombo::single(E), Edge::Press, "down");
        d.bind(KeyCombo::single(E), Edge::Release, "up");
        assert_eq!(d.on_key(E, true, 0.0), vec!["down"]);
        assert_eq!(d.on_key(E, true, 0.1), Vec::<&str>::new()); // repeat
        assert_eq!(d.on_key(E, false, 0.2), vec!["up"]);
        assert!(d.is_key_bound(E));
        assert!(!d.is_key_bound(Q));
    }

    #[test]
    fn chord_in_any_order_and_most_specific_wins() {
        let mut d = ComboDetector::new(0.25);
        d.bind(KeyCombo::single(E), Edge::Press, "e");
        d.bind(combo(&[SHIFT, E], 1), Edge::Press, "shift+e");
        assert_eq!(d.on_key(SHIFT, true, 0.0), Vec::<&str>::new());
        assert_eq!(d.on_key(E, true, 0.1), vec!["shift+e"]);
        d.on_key(E, false, 0.2);
        d.on_key(SHIFT, false, 0.2);
        // E first, then Shift: E fires on its own, then the chord.
        assert_eq!(d.on_key(E, true, 1.0), vec!["e"]);
        assert_eq!(d.on_key(SHIFT, true, 1.1), vec!["shift+e"]);
    }

    #[test]
    fn single_tap_waits_only_when_double_tap_is_bound() {
        let mut d = ComboDetector::new(0.25);
        d.bind(KeyCombo::single(E), Edge::Press, "single");
        d.bind(combo(&[E], 2), Edge::Press, "double");

        // Double tap within the window.
        assert!(d.on_key(E, true, 0.0).is_empty());
        d.on_key(E, false, 0.05);
        assert_eq!(d.on_key(E, true, 0.15), vec!["double"]);
        d.on_key(E, false, 0.2);

        // A single tap fires once the window passes.
        assert!(d.on_key(E, true, 1.0).is_empty());
        d.on_key(E, false, 1.05);
        assert!(d.tick(1.2).is_empty());
        assert_eq!(d.tick(1.26), vec!["single"]);

        // Too slow for a double tap: two singles.
        assert!(d.on_key(E, true, 2.0).is_empty());
        d.on_key(E, false, 2.05);
        assert_eq!(d.on_key(E, true, 2.5), vec!["single"]); // window of the first tap passed
        assert_eq!(d.tick(2.8), vec!["single"]);
    }

    #[test]
    fn triple_tap_chord() {
        let mut d = ComboDetector::new(0.3);
        d.bind(combo(&[SHIFT, Q], 3), Edge::Press, "triple");
        d.on_key(SHIFT, true, 0.0);
        for (i, t) in [0.1, 0.3, 0.5].into_iter().enumerate() {
            let fired = d.on_key(Q, true, t);
            d.on_key(Q, false, t + 0.05);
            assert_eq!(fired.is_empty(), i < 2);
        }
        // Not enough taps: nothing fires (no single-tap binding).
        d.on_key(Q, true, 2.0);
        d.on_key(Q, false, 2.05);
        assert!(d.tick(3.0).is_empty());
    }

    #[test]
    fn release_after_deferred_press_fires_after_it() {
        let mut d = ComboDetector::new(0.25);
        d.bind(KeyCombo::single(E), Edge::Press, "open");
        d.bind(KeyCombo::single(E), Edge::Release, "close");
        d.bind(combo(&[E], 2), Edge::Press, "double");
        assert!(d.on_key(E, true, 0.0).is_empty());
        assert!(d.on_key(E, false, 0.1).is_empty());
        assert_eq!(d.tick(0.3), vec!["open", "close"]);
    }
}
