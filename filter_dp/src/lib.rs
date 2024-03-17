use serde::Deserialize;
use serde_json::{json, Value};
use chrono::{Utc, TimeZone};
use std::{collections::HashMap, slice, os::raw::c_char, str};
use rv::{dist::Laplace, traits::Rv};
use std::{fs::File, io::Read, path::Path};
use rand::thread_rng;
use toml::from_str;

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
    let tag_str: String = str::from_utf8( unsafe { slice::from_raw_parts(tag as *const u8, tag_len as usize) } ).expect("Invalid UTF-8 in tag").to_string();
    let records: Value = serde_json::from_slice( unsafe { slice::from_raw_parts(record as *const u8, record_len as usize) } ).expect("Invalid JSON in record");
    
    // Apply noise to the records
    let mut noisy_records: Value = apply_noise(&tag_str, records);

    // Process time value. Not used by filter but is required to be passed back
    let time: String = Utc.timestamp_opt(time_sec as i64, time_nsec)
    .single()
    .expect("Invalid timestamp")
    .format("%Y-%m-%dT%H:%M:%S.%9f %z")
    .to_string();

    if let Value::Object(ref mut map) = noisy_records {
        map.insert("time".to_string(), Value::String(time));
        map.insert("tag".to_string(), Value::String(tag_str));
    }

  noisy_records.to_string()
  .as_ptr()
}

#[derive(Deserialize)]
struct NoiseSetting {
    b: f64,
    mu: f64,
}

fn apply_noise(filename: &String, mut message: Value) -> Value {
    let filepath: String = format!("{}.toml", filename);

    // Check if there is a configuration file for the given tag
    if Path::new(&filepath).exists() {
        let mut contents = String::new();
        // Attempt to open and read the file into a string
        match File::open(&filepath).and_then(|mut file| file.read_to_string(&mut contents)) {
            Ok(_) => {
                // Attempt to parse the TOML contents into a TomlValue
                if let Ok(decoded) = from_str::<HashMap<String, NoiseSetting>>(&contents) {
                    // Iterate through the message object if it's a JSON object
                    if let Value::Object(obj) = &mut message {
                        for (record_key, record_value) in obj.iter_mut() {
                            // Check if the record_key's value is numeric
                            if let Some(x) = record_value.as_f64() {
                                // Check if the record_key has a valid entry in the TOML data
                                if let Some(setting) = decoded.get(record_key) {
                                    // If the setting's b and mu are valid, calculate noise
                                    let laplace: Laplace = Laplace::new(setting.mu, setting.b).expect("Invalid Laplace parameters");
                                    let mut rng = thread_rng();
                                    let noise: f64 = laplace.draw(&mut rng);
                                    *record_value = json!(x + noise); // Update the value with noise applied
                                }
                            }
                        }
                    }
                }
            },
            Err(_) => {} // If there's an error opening or reading the file, do nothing
        }
    }
    message
}
