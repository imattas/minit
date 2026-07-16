#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let mut line = input.to_string();
        if !line.ends_with('\n') {
            line.push('\n');
        }
        let _ = minit_core::ipc::decode_request(&line);
        let _ = minit_core::ipc::decode_response(&line);
    }
});
