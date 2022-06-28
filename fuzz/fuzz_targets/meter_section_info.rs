#![no_main]
use libfuzzer_sys::fuzz_target;

extern crate meterreader_models;
use meterreader_models::MeterSectionInfo;

fuzz_target!(|data: &[u8]| {
    let _ = MeterSectionInfo::from_response(data);
});
