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
use std::{fs::File, io::Read};

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
    let mut noisy_records: Value = add_noise_to_records(&tag, record);

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
        #[serde(default = "default_mu")]
        mu: f64,
        sensitivity: f64,
        epsilon: f64,
        #[serde(flatten)]
        optional: OptionalSettings,
    },
    Gaussian {
        #[serde(default = "default_mu")]
        mu: f64,
        sensitivity: f64,
        epsilon: f64,
        delta: f64,
        #[serde(flatten)]
        optional: OptionalSettings,
    },
}

fn default_mu() -> f64 {0.0}

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
fn file_to_string(filepath: &String) -> Option<String> {
    let mut contents = String::new();
    match File::open(&filepath).and_then(|mut file| file.read_to_string(&mut contents)){
        Ok(_) => {
            Some(contents)
        }
        Err(_) => {None}
    }
}

fn add_noise_to_records(tag: &String, mut records: Value) -> Value {
    // Check if there is a settings file for the given tag
    let settings_file: String = format!("{}.toml", tag);
    let contents = file_to_string(&settings_file);
    if let Some(content) = contents {
        // Attempt to parse the TOML contents into a TomlValue and ensure records is a JSON object
        if let (Ok(decoded), Value::Object(obj)) = (toml::from_str::<HashMap<String, Noise>>(&content), &mut records) {
            for (record_key, record_value) in obj.iter_mut() {
                // Check if the record_key has a valid entry in the TOML data
                if let Some(setting) = decoded.get(record_key) {
                    // Match against the setting type
                    match setting {
                        Noise::Laplace {
                            mu,
                            sensitivity,
                            epsilon,
                            optional,
                        } => {
                            let b = sensitivity / epsilon;
                            let laplace = Laplace::new(*mu, b)
                                .expect("Invalid Laplace parameters");
                            *record_value = json!(add_noise_to_value(
                                Laplace(laplace),
                                record_value.as_f64().expect("Value is non-numeric"),
                                optional
                            ));
                        }
                        
                        Noise::Gaussian {
                            mu,
                            sensitivity,
                            epsilon,
                            delta,
                            optional,
                        } => {
                            let sigma = (2.0 * (1.25 / delta).ln() * sensitivity.powi(2)) / epsilon.powi(2).sqrt();
                            let gaussian = Gaussian::new(*mu, sigma)
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
    records
}
