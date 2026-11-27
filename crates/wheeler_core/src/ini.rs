//! Minimal INI abstraction so configuration parsing stays game independent
//! (and testable). The plugin adapts libskyrim's INI reader to [`IniSource`].

use std::collections::HashMap;

/// Read-only `section` / `key` lookup.
pub trait IniSource {
    fn value(&self, section: &str, key: &str) -> Option<&str>;
}

/// Reads a value with the same leniency as SimpleIni / `std::sto*` in the C++.
pub trait IniExt: IniSource {
    /// `GetFloatValue`
    fn f32_or(&self, section: &str, key: &str, default: f32) -> f32 {
        self.value(section, key)
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(default)
    }

    /// `GetUInt32Value` (decimal; `0x` hexadecimal accepted as well).
    fn u32_or(&self, section: &str, key: &str, default: u32) -> u32 {
        self.value(section, key)
            .and_then(parse_u32)
            .unwrap_or(default)
    }

    /// `GetBoolValue` (SimpleIni: true/yes/on/1, false/no/off/0).
    fn bool_or(&self, section: &str, key: &str, default: bool) -> bool {
        match self
            .value(section, key)
            .map(|v| v.trim().to_ascii_lowercase())
        {
            Some(v) if matches!(v.as_str(), "true" | "yes" | "on" | "1" | "t" | "y") => true,
            Some(v) if matches!(v.as_str(), "false" | "no" | "off" | "0" | "f" | "n") => false,
            _ => default,
        }
    }
}

impl<T: IniSource + ?Sized> IniExt for T {}

fn parse_u32(v: &str) -> Option<u32> {
    let v = v.trim();
    match v.strip_prefix("0x").or_else(|| v.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => v.parse().ok(),
    }
}

/// In-memory INI, mainly for tests and tools.
#[derive(Debug, Default, Clone)]
pub struct MemoryIni {
    values: HashMap<(String, String), String>,
}

impl MemoryIni {
    /// Parses `[section]` / `key = value` text (`;` and `#` comments).
    pub fn parse(text: &str) -> Self {
        let mut values = HashMap::new();
        let mut section = String::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                section = name.trim().to_owned();
            } else if let Some((key, value)) = line.split_once('=') {
                values.insert(
                    (section.clone(), key.trim().to_owned()),
                    value.trim().to_owned(),
                );
            }
        }
        Self { values }
    }
}

impl IniSource for MemoryIni {
    fn value(&self, section: &str, key: &str) -> Option<&str> {
        self.values
            .get(&(section.to_owned(), key.to_owned()))
            .map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_typed_values() {
        let ini = MemoryIni::parse("[A]\nf = 1.5\nu = 0x10\nd = 42\nb = yes\n; c = 1\n");
        assert_eq!(ini.f32_or("A", "f", 0.0), 1.5);
        assert_eq!(ini.u32_or("A", "u", 0), 16);
        assert_eq!(ini.u32_or("A", "d", 0), 42);
        assert!(ini.bool_or("A", "b", false));
        assert_eq!(ini.u32_or("A", "c", 7), 7);
    }
}
