use chrono::{TimeZone, Utc};
use rand::{rngs::StdRng, thread_rng, RngCore, SeedableRng};
use rv::{
    dist::{Distribution, Distribution::*, Gaussian, Laplace},
    prelude::Rv,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    os::raw::c_char,
    slice, str,
};
use std::{fs::File, io::Read, path::Path};

//use toml::from_str;

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
    let tag: String =
        str::from_utf8(unsafe { slice::from_raw_parts(tag as *const u8, tag_len as usize) })
            .expect("Invalid UTF-8 in tag")
            .to_string();
    let record: Value = serde_json::from_slice(unsafe {
        slice::from_raw_parts(record as *const u8, record_len as usize)
    })
    .expect("Invalid JSON in record");

    // Apply noise to the records
    let mut noisy_records: Value = apply_noise_to_records(&tag, record);

    // Process time value. Not used by filter but is required to be passed back
    let time: String = Utc
        .timestamp_opt(time_sec as i64, time_nsec)
        .single()
        .expect("Invalid timestamp")
        .format("%Y-%m-%dT%H:%M:%S.%9f %z")
        .to_string();

    if let Value::Object(ref mut map) = noisy_records {
        map.insert("time".to_string(), Value::String(time));
        map.insert("tag".to_string(), Value::String(tag));
    }

    noisy_records.to_string().as_ptr()
}

#[derive(Deserialize)]
struct OptionalSettings {
    rng_seed: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Noise {
    Laplace {
        mu: f64,
        b: f64,
        #[serde(flatten)]
        optional: OptionalSettings,
    },
    Gaussian {
        mu: f64,
        sigma: f64,
        #[serde(flatten)]
        optional: OptionalSettings,
    },
}

fn add_noise_to_value(distribution: Distribution, value: f64, optional: &OptionalSettings) -> f64 {
    // We need noise to choose a value on the distribution. This can optionally be seeded
    let mut rng: Box<dyn RngCore> = match &optional.rng_seed {
        Some(seed) => {
            let mut hasher = DefaultHasher::new();
            seed.hash(&mut hasher);
            let seed_hash = hasher.finish();
            Box::new(StdRng::seed_from_u64(seed_hash))
        }
        None => Box::new(thread_rng()),
    };

    // Creates the noise from the distribution
    let noise: f64 = distribution.draw(&mut rng).into();
    // Returns the noisy value
    value + noise
}

fn apply_noise_to_records(filename: &String, mut records: Value) -> Value {
    let filepath: String = format!("{}.toml", filename);

    // Check if there is a configuration file for the given tag
    if Path::new(&filepath).exists() {
        let mut contents = String::new();
        // Attempt to open and read the file into a string
        match File::open(&filepath).and_then(|mut file| file.read_to_string(&mut contents)) {
            Ok(_) => {
                // Attempt to parse the TOML contents into a TomlValue
                if let Ok(decoded) = toml::from_str::<HashMap<String, Noise>>(&contents) {
                    // Iterate through the message object if it's a JSON object
                    if let Value::Object(obj) = &mut records {
                        for (record_key, record_value) in obj.iter_mut() {
                            // Check if the record_key has a valid entry in the TOML data
                            if let Some(setting) = decoded.get(record_key) {
                                // Match against the setting type
                                match setting {
                                    Noise::Laplace {
                                        mu,
                                        b,
                                        optional,
                                    } => {
                                        let laplace = Laplace::new(*mu, *b)
                                            .expect("Invalid Laplace parameters");
                                        *record_value = json!(add_noise_to_value(
                                            Laplace(laplace),
                                            record_value.as_f64().expect("Value is non-numeric"),
                                            optional
                                        ));
                                    }
                                    
                                    Noise::Gaussian {
                                        mu,
                                        sigma,
                                        optional,
                                    } => {
                                        let gaussian = Gaussian::new(*mu, *sigma)
                                            .expect("Invalid Gaussian parameters");
                                        *record_value = json!(add_noise_to_value(
                                            Gaussian(gaussian),
                                            record_value.as_f64().expect("Value is non-numeric"),
                                            optional
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => {} // If there's an error opening or reading the file, do nothing
        }
    }
    records
}
