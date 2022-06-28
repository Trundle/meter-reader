#![no_main]
use libfuzzer_sys::fuzz_target;

extern crate meterreader_models;
use meterreader_models::MeterValue;

fuzz_target!(|data: &[u8]| {
    let _ = MeterValue::from_data(data);
});
