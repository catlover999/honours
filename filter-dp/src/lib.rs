use wasm_bindgen::prelude::*;
use std::ffi::{CString, c_char};
use std::slice;
use std::str;

#[wasm_bindgen]
extern {
    // Enable console logging for debugging
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[no_mangle]
pub extern "C" fn rust_filter(
    tag: *const c_char,
    tag_len: u32,
    time_sec: u32,
    time_nsec: u32,
    record: *const c_char,
    record_len: u32,
)-> *mut c_char {
    // Setup panic hook for better error messages in wasm
    console_error_panic_hook::set_once();

    
    unsafe {
        // Convert the record C string to Rust string
        let record_slice = slice::from_raw_parts(record as *const u8, record_len as usize);
        let record_str = match str::from_utf8(record_slice) {
            Ok(s) => s,
            Err(_) => {
                log("Failed to convert record to UTF-8 string");
                return std::ptr::null_mut();
            },
        };

        // TODO
        
        // Convert the noisy value back to a C string
        let c_str_processed = CString::new(record_str).unwrap();
        c_str_processed.into_raw()
    }
}
