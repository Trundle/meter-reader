#![no_main]
use libfuzzer_sys::fuzz_target;

extern crate meterreader_models;
use meterreader_models::MeterSampleValue;

fuzz_target!(|data: &[u8]| {
    let _ = MeterSampleValue::from_response(data);
});
