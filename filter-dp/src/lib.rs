use wasm_bindgen::prelude::*;
use libc::{c_char, size_t};
use std::ffi::{CStr, CString};
use opendp::error::Fallible;
use opendp::measurements::laplace::make_laplace;

#[wasm_bindgen]
extern {
    // Enable console logging for debugging
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[no_mangle]
pub extern "C" fn rust_filter(tag: *const c_char, tag_len: size_t, time_sec: u32, time_nsec: u32, record: *const c_char, record_len: size_t) -> *mut c_char {
    // Setup panic hook for better error messages in wasm
    console_error_panic_hook::set_once();
    
    unsafe {
        // Convert the record C string to Rust string
        let record_str = CStr::from_ptr(record).to_string_lossy();
        let record_value: f64 = match record_str.parse() {
            Ok(val) => val,
            Err(_) => {
                log("Failed to parse record to f64");
                return std::ptr::null_mut();
            },
        };

        // Apply Laplace noise
        let noisy_value = add_laplace_noise(record_value).unwrap_or_else(|_| {
            log("Failed to apply Laplace noise");
            0.0 // You might want to handle this case differently
        });
        
        // Convert the noisy value back to a C string
        let noisy_value_str = noisy_value.to_string();
        let c_str_noisy = CString::new(noisy_value_str).unwrap();
        c_str_noisy.into_raw()
    }
}

fn add_laplace_noise(value: f64) -> Fallible<f64> {
    let measurement = make_laplace(1.0)?; // Assuming a scale of 1.0 for simplicity
    measurement.invoke(value)
}
