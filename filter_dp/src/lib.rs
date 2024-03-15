use serde_json::{json, Value};
use chrono::{Utc, TimeZone, LocalResult, prelude::DateTime};
use std::slice;
use std::os::raw::c_char;
use std::fs;
use std::io::{Read, Write};

#[no_mangle]
pub extern "C" fn filter_dp(
    tag: *const c_char,
    tag_len: u32,
    time_sec: u32,
    time_nsec: u32,
    record: *const c_char,
    record_len: u32,
) -> *const u8 {
    
    // Process tag and record
    let tag_str: &str = std::str::from_utf8( unsafe { std::slice::from_raw_parts(tag as *const u8, tag_len as usize) } ).expect("Invalid UTF-8 in tag");
    let message: Value = serde_json::from_slice( unsafe { slice::from_raw_parts(record as *const u8, record_len as usize) } ).expect("Invalid JSON in record");
    
    // Process time value. Not used by filter but is required to be passed back
    let dt_result: LocalResult<DateTime<Utc>> = Utc.timestamp_opt(time_sec as i64, time_nsec);
    let dt: DateTime<Utc> = match dt_result {
        chrono::LocalResult::Single(dt) => dt,
        _ => panic!("Invalid timestamp"), // Handle the None case or ambiguity
    };
    let time: String = dt.format("%Y-%m-%dT%H:%M:%S.%9f %z").to_string();

    let message: Value = json!({
        "time": time,
        "tag": tag_str,
    });


  let buf: String = message.to_string();
  buf.as_ptr()
}