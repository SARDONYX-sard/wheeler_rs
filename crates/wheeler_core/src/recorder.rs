//! "Press the keys to bind" recording for the settings window.

use crate::binding::{KeyCombo, MAX_CHORD_KEYS};
use crate::input_id::KeyId;

/// Escape alone cancels recording (it can't be recorded by itself).
pub const CANCEL_KEY: KeyId = 0x01;

/// Records the chord held at its largest while keys are pressed; finishes
/// when every key is released.
#[derive(Debug, Clone, Default)]
pub struct Recorder {
    held: Vec<KeyId>,
    peak: Vec<KeyId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordResult {
    /// Still recording.
    Pending,
    Done(KeyCombo),
    Cancelled,
}

impl Recorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Keys held right now (for live display).
    pub fn held(&self) -> &[KeyId] {
        &self.held
    }

    /// The chord recorded so far.
    pub fn peak(&self) -> &[KeyId] {
        &self.peak
    }

    /// Feeds one key edge; `taps` is applied to the result.
    pub fn on_key(&mut self, key: KeyId, down: bool, taps: u8) -> RecordResult {
        if down {
            if !self.held.contains(&key) {
                self.held.push(key);
            }
            if self.held.len() > self.peak.len() && self.held.len() <= MAX_CHORD_KEYS {
                self.peak = self.held.clone();
            }
            return RecordResult::Pending;
        }
        if !self.held.contains(&key) {
            return RecordResult::Pending; // released a key pressed before recording
        }
        self.held.retain(|&k| k != key);
        if !self.held.is_empty() {
            return RecordResult::Pending;
        }
        if self.peak == [CANCEL_KEY] {
            return RecordResult::Cancelled;
        }
        match KeyCombo::new(&self.peak, taps) {
            Some(combo) => RecordResult::Done(combo),
            None => RecordResult::Cancelled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_the_largest_chord() {
        let mut r = Recorder::new();
        assert_eq!(r.on_key(0x2A, true, 1), RecordResult::Pending);
        assert_eq!(r.on_key(0x12, true, 1), RecordResult::Pending);
        assert_eq!(r.on_key(0x12, false, 1), RecordResult::Pending);
        assert_eq!(
            r.on_key(0x2A, false, 2),
            RecordResult::Done(KeyCombo::new(&[0x2A, 0x12], 2).unwrap())
        );
    }

    #[test]
    fn escape_cancels_and_stale_releases_are_ignored() {
        let mut r = Recorder::new();
        assert_eq!(r.on_key(256, false, 1), RecordResult::Pending);
        r.on_key(CANCEL_KEY, true, 1);
        assert_eq!(r.on_key(CANCEL_KEY, false, 1), RecordResult::Cancelled);
    }

    #[test]
    fn caps_at_four_keys() {
        let mut r = Recorder::new();
        for k in 1..=5 {
            r.on_key(k + 0x10, true, 1);
        }
        assert_eq!(r.peak().len(), 4);
    }
}
