#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = minit_core::unit::parse_unit_toml(input).map(|unit| unit.validate());
    }
});
