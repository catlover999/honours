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
    os::raw::c_char,
    slice, str,
};

#[no_mangle]
pub extern "C" fn filter_dp(
    tag: *const c_char,
    tag_len: u32,
    _time_sec: u32,
    _time_nsec: u32,
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
    let mut noisy_records = add_noise_to_records(&tag, record);

    // Process time value. Not used by filter but is required to be passed back
    //let time: String = Utc
    //    .timestamp_opt(time_sec as i64, time_nsec)
    //    .single()
    //    .expect("Invalid timestamp")
    //    .format("%Y-%m-%dT%H:%M:%S.%9f %z")
    //    .to_string();

    //add_record(&mut noisy_records, "time".to_string(), time);
    //add_record(&mut noisy_records, "tag".to_string(), tag);

    noisy_records.to_string().as_ptr()
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

#[derive(Deserialize)]
struct OptionalSettings {
    rng_seed: Option<String>,
    #[serde(default = "Units::default_unit")]
    unit: Units,
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

fn default_mu() -> f64 {
    0.0
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
    match &optional.unit {
        Units::int => Ok(Number::from(value.as_i64().unwrap() + (noise as i64))),
        Units::float => {Ok(Number::from_f64(value.as_f64().unwrap() + (noise.round() as f64)).unwrap())}
    }
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
    if let Some(setting) = config.get(record_key) {
        match setting {
            Noise::Laplace {
                mu,
                sensitivity,
                epsilon,
                optional,
            } => {
                let b = sensitivity / epsilon;
                let laplace = Laplace::new(*mu, b).map_err(|e| e.to_string())?;
                *record_value = json!(add_noise_to_value(
                    Laplace(laplace),
                    record_value.as_number().ok_or("Value not numeric")?.clone(),
                    optional
                ));
            },
            Noise::Gaussian {
                mu,
                sensitivity,
                epsilon,
                delta,
                optional,
            } => {
                let sigma = ((2.0 * (1.25 / delta).ln() * sensitivity.powi(2)) / epsilon.powi(2)).sqrt();
                let gaussian = Gaussian::new(*mu, sigma).map_err(|e| e.to_string())?;
                *record_value = json!(add_noise_to_value(
                    Gaussian(gaussian),
                    record_value.as_number().ok_or("Value not numeric")?.clone(),
                    optional
                ));
            },
        }
    }
    Ok(())
}

fn add_noise_to_records(tag: &String, mut records: Value) -> Value {
    // Check if there is a settings file for the given tag
    let mut errors: Vec<String> = Vec::new();
    match load_configuration(tag) {
        Ok(config) => {
            if let Value::Object(ref mut map) = records {
                for (record_key, record_value) in map.iter_mut() {
                    // Match against the setting type
                    match check_settings_for_record(record_key, record_value, &config) {
                        Err(error) => {errors.push(error);},
                        Ok(()) => {}
                    }
                }
            }
        }
        Err(error) => {errors.push(error);},
    }  
    record_error(&mut records, errors);
    records
}


fn record_error(records: &mut Value, errors: Vec<String>) {
    for error in errors {
        match records["errors"] {
            Value::Array(ref mut errors) => {
                // If "errors" is an array, push the new error message.
                errors.push(Value::String(error));
            },
            _ => {
                // If "errors" does not exist or is not an array, create a new array with the error.
                records["errors"] = Value::Array(vec![Value::String(error)]);
            }
        }
    }
}