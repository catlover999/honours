use rand::{rngs::StdRng, thread_rng, RngCore, SeedableRng};
use rv::{
    dist::{Distribution, Distribution::*, Gaussian, Laplace},
    prelude::Rv,
};
use serde::Deserialize;
use serde_json::{json, Number, Value};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    fs,
    hash::{Hash, Hasher},
    slice,
    str,
};

#[no_mangle]
pub extern "C" fn filter_dp(
    tag: *const u8,
    tag_len: u32,
    _time_sec: u32,
    _time_nsec: u32,
    record: *const u8,
    record_len: u32,
) -> *const u8 {
    // Process tag and record
    let tag: String =
        str::from_utf8(unsafe { slice::from_raw_parts(tag, tag_len as usize) })
            .expect("Invalid UTF-8 in tag")
            .to_string();
    let record: Value = serde_json::from_slice(unsafe {
        slice::from_raw_parts(record, record_len as usize)
    })
    .expect("Invalid JSON in record");

    // Apply noise to the records
    let noisy_records: Value = add_noise_to_records(&tag, record);

    noisy_records.to_string().as_ptr()
}

fn add_noise_to_records(tag: &String, mut records: Value) -> Value {
    // Check if there is a settings file for the given tag
    match load_configuration(tag) {
        Ok(config) => {
            println!("Loaded config");
            if let Value::Object(ref mut map) = records {
                for (record_key, record_value) in map.iter_mut() {
                    // Match against the setting type
                    match check_settings_for_record(record_key, record_value, &config) {
                        Err(error) => {
                            eprintln!("{}", error)
                        }
                        Ok(()) => {}
                    }
                }
            }
        }
        Err(error) => {
            eprintln!("{}", error)
        }
    }
    records
}

fn load_configuration(tag: &str) -> Result<HashMap<String, Noise>, String> {
    let settings_file = format!("{}.toml", tag);
    let contents = fs::read_to_string(&settings_file)
        .map_err(|e| format!("Failed to read settings file: {}", e))?;
    toml::from_str::<HashMap<String, Noise>>(&contents)
        .map_err(|e| format!("Failed to parse settings: {}", e))
}

fn check_settings_for_record(
    record_key: &String,
    record_value: &mut Value,
    config: &HashMap<String, Noise>,
) -> Result<(), String> {
    println!("{}:{}", record_key, record_value);
    if let Some(setting) = config.get(record_key) {
        match setting {
            Noise::Laplace {
                mu,
                sensitivity,
                epsilon,
                optional,
            } => {
                println!("Matched laplace! {},{},{}", mu, sensitivity, epsilon);
                let b = sensitivity / epsilon;
                let laplace = Laplace::new(*mu, b).map_err(|e| e.to_string())?;
                *record_value = json!(add_noise_to_value(
                    Laplace(laplace),
                    record_value.as_number().ok_or("Value not numeric")?.clone(),
                    optional
                )?);
            }
            Noise::Gaussian {
                mu,
                sensitivity,
                epsilon,
                delta,
                optional,
            } => {
                let sigma =
                    ((2.0 * (1.25 / delta).ln() * sensitivity.powi(2)) / epsilon.powi(2)).sqrt();
                let gaussian = Gaussian::new(*mu, sigma).map_err(|e| e.to_string())?;
                *record_value = json!(add_noise_to_value(
                    Gaussian(gaussian),
                    record_value.as_number().ok_or("Value not numeric")?.clone(),
                    optional
                )?);
            }
        }
    }
    Ok(())
}

fn add_noise_to_value(
    distribution: Distribution,
    value: Number,
    optional: &OptionalSettings,
) -> Result<Number, String> {
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
    println!("Noise:{}, value{}", noise, value);
    // Adds the noise to the value using the spesified unit, returns as a serde_json Number
    match &optional.unit {
        Units::int => Ok(Number::from(value.as_i64().unwrap() + (noise as i64))),
        Units::float => {
            Ok(Number::from_f64(value.as_f64().unwrap() + (noise as f64)).unwrap())
        }
    }
}

// Define deserialization types for Toml settings files (and where applicable, defaults)
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
fn default_mu() -> f64 {
    0.0
}
#[derive(Deserialize)]
struct OptionalSettings {
    rng_seed: Option<String>,
    #[serde(default = "Units::default_unit")]
    unit: Units,
}
#[derive(Deserialize)]
#[allow(non_camel_case_types)]
enum Units {
    int,
    float,
}
impl Units {
    fn default_unit() -> Self {
        Units::float
    }
}
