//! Save data of the wheels (C++ `SerializationEntry` + the
//! `SerializeIntoJsonObj` / `SerializeFromJsonObj` hierarchy).
//!
//! The format matches the C++ plugin so existing co-saves keep working:
//! one `'WJSN'` record (version 2) under the plugin ID `'WHLR'`, holding a
//! little-endian `u64` byte length followed by this JSON:
//!
//! ```json
//! {
//!   "activewheel": 0,
//!   "wheels": [
//!     { "entries": [
//!         { "selecteditem": 0,
//!           "items": [ { "type": "WheelItemWeapon", "formID": 1234, "uniqueID": 5 } ] }
//!     ] }
//!   ]
//! }
//! ```

use serde::{Deserialize, Serialize};

/// C++ `WHEELER_SERIALIZATION_ID` (`'WHLR'`).
pub const SERIALIZATION_ID: u32 = u32::from_be_bytes(*b"WHLR");
/// C++ `WHEELER_JSON_STRING_TYPE` (`'WJSN'`).
pub const JSON_RECORD_TYPE: u32 = u32::from_be_bytes(*b"WJSN");
/// C++ `SERIALIZER_VERSION`
pub const SERIALIZER_VERSION: u32 = 2;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WheelerData {
    #[serde(default)]
    pub wheels: Vec<WheelData>,
    #[serde(rename = "activewheel", default)]
    pub active_wheel: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WheelData {
    #[serde(default)]
    pub entries: Vec<EntryData>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryData {
    #[serde(default)]
    pub items: Vec<ItemData>,
    #[serde(rename = "selecteditem", default)]
    pub selected_item: i32,
}

/// One wheel item: `type` is the C++ class name (`WheelItemWeapon`, ...).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemData {
    #[serde(rename = "type")]
    pub item_type: String,
    #[serde(rename = "formID")]
    pub form_id: u32,
    /// Weapons and armors only.
    #[serde(rename = "uniqueID", default, skip_serializing_if = "Option::is_none")]
    pub unique_id: Option<u16>,
}

impl ItemData {
    pub fn new(item_type: &str, form_id: u32, unique_id: Option<u16>) -> Self {
        Self {
            item_type: item_type.to_owned(),
            form_id,
            unique_id,
        }
    }
}

impl WheelerData {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("wheel data is always serializable")
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// The record payload: `u64` length + JSON bytes (C++ `Serial::Write<std::string>`).
    pub fn to_record(&self) -> Vec<u8> {
        let json = self.to_json();
        let mut out = Vec::with_capacity(8 + json.len());
        out.extend_from_slice(&(json.len() as u64).to_le_bytes());
        out.extend_from_slice(json.as_bytes());
        out
    }
}

/// Splits a record payload into its JSON string (C++ `Serial::Read<std::string>`).
pub fn json_from_record(payload: &[u8]) -> Option<&str> {
    let (len, rest) = payload.split_first_chunk::<8>()?;
    let len = usize::try_from(u64::from_le_bytes(*len)).ok()?;
    std::str::from_utf8(rest.get(..len)?).ok()
}

/// Clamps a saved selection to `num_items` (C++ `std::clamp(sel, 0, n)`,
/// minus its off-by-one).
pub fn clamp_selected(selected: i32, num_items: usize) -> i32 {
    selected.clamp(0, (num_items as i32 - 1).max(0))
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    /// Shape written by the C++ plugin (nlohmann sorts keys).
    const CPP_JSON: &str = r#"{"activewheel":1,"wheels":[{"entries":[{"items":[{"formID":4660,"type":"WheelItemWeapon","uniqueID":5},{"formID":77,"type":"WheelItemSpell"}],"selecteditem":1},{"items":[],"selecteditem":0}]},{"entries":[]}]}"#;

    #[test]
    fn reads_cpp_json() {
        let data = WheelerData::from_json(CPP_JSON).unwrap();
        assert_eq!(data.active_wheel, 1);
        assert_eq!(data.wheels.len(), 2);
        let entry = &data.wheels[0].entries[0];
        assert_eq!(entry.selected_item, 1);
        assert_eq!(
            entry.items[0],
            ItemData::new("WheelItemWeapon", 4660, Some(5))
        );
        assert_eq!(entry.items[1], ItemData::new("WheelItemSpell", 77, None));
    }

    #[test]
    fn record_round_trip() {
        let data = WheelerData::from_json(CPP_JSON).unwrap();
        let record = data.to_record();
        let json = json_from_record(&record).unwrap();
        assert_eq!(WheelerData::from_json(json).unwrap(), data);
        assert_eq!(json_from_record(&record[..4]), None);
        assert_eq!(json_from_record(&record[..record.len() - 1]), None);
    }

    #[test]
    fn clamps_selection() {
        assert_eq!(clamp_selected(5, 2), 1);
        assert_eq!(clamp_selected(-1, 2), 0);
        assert_eq!(clamp_selected(3, 0), 0);
    }
}
