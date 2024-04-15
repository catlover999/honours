use log::{debug, error, warn};
use rand::{rngs::StdRng, thread_rng, RngCore, SeedableRng};
use rv::{
    dist::{Distribution, Distribution::*, Gaussian, Laplace},
    prelude::Rv,
};
use serde::Deserialize;
use serde_json::{json, Number, Value};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    error::Error,
    ffi::CString,
    fs,
    hash::{Hash, Hasher},
    slice, str,
};

/// Filters event records by applying differentially private noise.
///
/// # Arguments
///
/// * `tag` - A pointer to the event's tag as a byte array.
/// * `tag_len` - The length of the tag byte array.
/// * `_time_sec` - The timestamp of the event in seconds (unused).
/// * `_time_nsec` - The timestamp of the event in nanoseconds (unused).
/// * `records` - A pointer to the event's records as a byte array.
/// * `record_len` - The length of the records byte array.
///
/// # Returns
///
/// A pointer to the JSON-formtted perturbed records as a C-compatible string.
#[no_mangle]
pub extern "C" fn filter_dp(
    tag: *const u8,
    tag_len: u32,
    _time_sec: u32,
    _time_nsec: u32,
    records: *const u8,
    record_len: u32,
) -> *const u8 {
    // Process event's tag and records into Rust types
    let tag = str::from_utf8(unsafe { slice::from_raw_parts(tag, tag_len as usize) })
        .map_err(|e| error!("{}", e))
        .unwrap();
    let mut json_records =
        serde_json::from_slice(unsafe { slice::from_raw_parts(records, record_len as usize) })
            .map_err(|e| error!("{}", e))
            .unwrap();

    // Apply noise to the records
    let noisy_records = match add_noise_to_records(tag, &mut json_records) {
        Ok(noisy) => noisy,
        Err(e) => {
            error!("{}", e.to_string());
            &mut json_records
        }
    };

    // Leak the CString into a raw pointer to avoid it being deallocated
    CString::new(noisy_records.to_string())
        .expect("CString::new failed")
        .into_raw() as *const u8
}

/// Adds differential privacy noise to the records based on the configuration for the given tag.
///
/// # Arguments
///
/// * `tag` - The tag associated with the records.
/// * `records` - A mutable reference to the JSON records.
///
/// # Returns
///
/// A mutable reference to the noisy records on success, or an error on failure.
fn add_noise_to_records<'a>(
    tag: &str,
    records: &'a mut Value,
) -> Result<&'a mut Value, Box<dyn Error>> {
    // Check if there is a settings file for the given tag
    if let Ok(config) = load_configuration(tag) {
        debug!("Loaded config: {}", tag);
        if let Value::Object(ref mut map) = records {
            for (record_key, record_value) in map.iter_mut() {
                if let Err(e) = process_setting_for_record(record_key, record_value, &config) {
                    warn!("{}", e);
                }
            }
        }
    }
    Ok(records)
}

/// Loads the configuration for the given tag from a TOML file.
///
/// # Arguments
///
/// * `tag` - The tag associated with the configuration.
///
/// # Returns
///
/// A `HashMap` containing the noise configuration on success, or an error on failure.
fn load_configuration(tag: &str) -> Result<HashMap<String, Noise>, Box<dyn Error>> {
    let settings_file = format!("filters/{}.toml", tag);
    let contents = fs::read_to_string(&settings_file)?;
    let toml = toml::from_str::<HashMap<String, Noise>>(&contents)?;
    Ok(toml)
}

/// Perturbs a value based on the noise configuration for a specific record key-value pair.
///
/// # Arguments
///
/// * `record_key` - The key of the record.
/// * `record_value` - A mutable reference to the value of the record.
/// * `config` - A reference to the noise configuration.
///
/// # Returns
///
/// `()` on success, or an error on failure.
fn process_setting_for_record(
    record_key: &str,
    record_value: &mut Value,
    config: &HashMap<String, Noise>,
) -> Result<(), Box<dyn Error>> {
    debug!("key: {}, value: {}", record_key, record_value);

    if let Some(setting) = config.get(record_key) {
        let value: f64;
        match record_value {
            Value::Number(n) => match n.as_f64() {
                Some(n) => {
                    value = n;
                }
                None => return Err("Can't convert to a float".into()),
            },
            Value::String(s) => match s.parse::<f64>() {
                Ok(n) => value = n,
                Err(e) => {
                    return Err(e.into());
                }
            },
            _ => {
                return Err("Value not numeric".into());
            }
        }
        match setting {
            Noise::Laplace {
                mu,
                sensitivity,
                epsilon,
                optional,
            } => {
                let b = sensitivity / epsilon;
                let laplace = Laplace::new(*mu, b)?;
                *record_value = json!(add_noise_to_value(Laplace(laplace), value, optional)
                    .map_err(|e| warn!("{}", e)));
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
                let gaussian = Gaussian::new(*mu, sigma)?;
                *record_value = json!(add_noise_to_value(Gaussian(gaussian), value, optional)?);
            }
        }
    }
    Ok(())
}

/// Adds noise to a value based on the specified distribution and optional settings.
///
/// # Arguments
///
/// * `distribution` - The noise distribution to use.
/// * `value` - The value to add noise to.
/// * `optional` - The optional settings for noise generation.
///
/// # Returns
///
/// A a serde_json `Number` representing the noisy value on success, or an error on failure.
fn add_noise_to_value(
    distribution: Distribution,
    value: f64,
    optional: &OptionalSettings,
) -> Result<Number, Box<dyn Error>> {
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

    debug!("Noise: {}", noise);

    // Adds the noise to the value using the specified unit, returns as a serde_json Number
    match optional.unit {
        Units::int => Ok(Number::from((value + noise) as i64)),
        Units::float => Ok(Number::from_f64(value + noise).unwrap()),
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
