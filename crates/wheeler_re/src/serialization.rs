//! Port of `Serialization/SerializationEntry.cpp`: stores the wheels in the
//! SKSE co-save as one JSON record, compatible with the C++ plugin.

use libskyrim::skse::{SerializationInterface, get_serialization_interface};
use libskyrim::{skse_error, skse_message};
use wheeler_core::save::{
    JSON_RECORD_TYPE, SERIALIZATION_ID, SERIALIZER_VERSION, WheelerData, json_from_record,
};

use crate::items::factory;
use crate::wheeler;

/// C++ `SerializationEntry::BindSerializationCallbacks`
pub fn install() {
    let Some(intfc) = (unsafe { get_serialization_interface().as_ref() }) else {
        skse_error!("Serialization interface unavailable; wheels will not be saved");
        return;
    };
    intfc.set_unique_id(SERIALIZATION_ID);
    intfc.set_save_callback(Some(save));
    intfc.set_load_callback(Some(load));
    intfc.set_revert_callback(Some(revert));
}

/// C++ `SerializationEntry::Save`
unsafe extern "system" fn save(intfc: *mut SerializationInterface) {
    let Some(intfc) = (unsafe { intfc.as_ref() }) else {
        return;
    };
    let payload = wheeler::save_data().to_record();
    let Ok(len) = u32::try_from(payload.len()) else {
        skse_error!("Wheel data too large to save ({} bytes)", payload.len());
        return;
    };
    if !intfc.write_record(
        JSON_RECORD_TYPE,
        SERIALIZER_VERSION,
        payload.as_ptr().cast(),
        len,
    ) {
        skse_error!("Failed to write the wheel record");
        return;
    }
    skse_message!("Serialized wheels ({len} bytes)");
}

/// C++ `SerializationEntry::Load`
unsafe extern "system" fn load(intfc: *mut SerializationInterface) {
    let Some(intfc) = (unsafe { intfc.as_ref() }) else {
        return;
    };
    let (mut ty, mut version, mut length) = (0, 0, 0);
    while intfc.get_next_record_info(&mut ty, &mut version, &mut length) {
        if ty != JSON_RECORD_TYPE {
            continue; // unread data is skipped by SKSE
        }
        if version != SERIALIZER_VERSION {
            skse_error!("Load: unsupported record version {version}");
            continue;
        }
        let mut payload = vec![0u8; length as usize];
        let read = intfc.read_record_data(payload.as_mut_ptr().cast(), length);
        payload.truncate(read as usize);

        let Some(json) = json_from_record(&payload) else {
            skse_error!("Load: truncated wheel record");
            continue;
        };
        match WheelerData::from_json(json) {
            Ok(data) => {
                let resolve = |old: u32| {
                    let mut new = 0;
                    intfc.resolve_form_id(old, &mut new).then_some(new)
                };
                wheeler::load_data(&data, &mut |item| factory::from_save_data(item, resolve));
                skse_message!("Loaded {} wheels", data.wheels.len());
            }
            Err(e) => skse_error!("Failed to parse wheel json: {e}"),
        }
    }
}

/// C++ `SerializationEntry::Revert`
unsafe extern "system" fn revert(_intfc: *mut SerializationInterface) {
    wheeler::clear();
}
