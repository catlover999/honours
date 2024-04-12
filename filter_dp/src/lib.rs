use anyhow::{anyhow, Result};
use rand::{rngs::StdRng, thread_rng, RngCore, SeedableRng};
use rv::{
    dist::{Distribution, Distribution::*, Gaussian, Laplace},
    prelude::Rv,
};
use serde::Deserialize;
use serde_json::{json, Number, Value};
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    ffi::CString,
    fs,
    hash::{Hash, Hasher},
    slice, str,
};

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
        .map_err(|e| anyhow!(e))
        .unwrap();

    let mut json_records =
        serde_json::from_slice(unsafe { slice::from_raw_parts(records, record_len as usize) })
            .map_err(|e| anyhow!(e))
            .unwrap();

    // Apply noise to the records
    let noisy_records = match add_noise_to_records(tag, &mut json_records) {
        Ok(noisy) => noisy,
        Err(e) => {
            // Prints to stderr. Fluent Bit hooks it's stderr into WAMR. Depending on deployment requirements, it may make sense to use an alternative logging library.
            eprintln!("{}", e.to_string());
            &mut json_records
        }
    };

    // Leak the CString into a raw pointer to avoid it being deallocated
    CString::new(noisy_records.to_string())
        .expect("CString::new failed")
        .into_raw() as *const u8
}

fn add_noise_to_records<'a>(tag: &str, records: &'a mut Value) -> Result<&'a mut Value> {
    // Check if there is a settings file for the given tag
    if let Ok(config) = load_configuration(tag) {
        #[cfg(debug_assertions)]
        println!("Loaded config: {}", tag);
        if let Value::Object(ref mut map) = records {
            for (record_key, record_value) in map.iter_mut() {
                process_setting_for_record(record_key, record_value, &config)
                    .map_err(|e| anyhow!(e))?;
            }
        }
    }
    Ok(records)
}

fn load_configuration(tag: &str) -> Result<HashMap<String, Noise>> {
    let settings_file = format!("filters/{}.toml", tag);
    let contents = fs::read_to_string(&settings_file)
        .map_err(|e| anyhow!("Failed to read settings file: {}", e))?;
    toml::from_str::<HashMap<String, Noise>>(&contents)
        .map_err(|e| anyhow!("Failed to parse settings: {}", e))
}

fn process_setting_for_record(
    record_key: &str,
    record_value: &mut Value,
    config: &HashMap<String, Noise>,
) -> Result<()> {
    #[cfg(debug_assertions)]
    println!("key: {}, value: {}", record_key, record_value);

    if let Some(setting) = config.get(record_key) {
        match setting {
            Noise::Laplace {
                mu,
                sensitivity,
                epsilon,
                optional,
            } => {
                let b = sensitivity / epsilon;
                let laplace = Laplace::new(*mu, b).map_err(|e| anyhow!(e))?;
                *record_value = json!(add_noise_to_value(
                    Laplace(laplace),
                    record_value
                        .as_f64()
                        .ok_or_else(|| anyhow!("Value not numeric"))?,
                    optional
                )
                .map_err(|e| anyhow!(e))?);
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
                let gaussian = Gaussian::new(*mu, sigma).map_err(|e| anyhow!(e))?;
                *record_value = json!(add_noise_to_value(
                    Gaussian(gaussian),
                    record_value
                        .as_f64()
                        .ok_or_else(|| anyhow!("Value not numeric"))?,
                    optional
                )
                .map_err(|e| anyhow!(e))?);
            }
        }
    }
    Ok(())
}

fn add_noise_to_value(
    distribution: Distribution,
    value: f64,
    optional: &OptionalSettings,
) -> Result<Number> {
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

    #[cfg(debug_assertions)]
    println!("Noise: {}", noise);

    // Adds the noise to the value using the specified unit, returns as a serde_json Number
    match optional.unit {
        Units::int => Ok(Number::from((value + noise) as i64)),
        Units::float => Ok(Number::from_f64(value + noise)
            .ok_or_else(|| anyhow!("Failed to create float number"))?),
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
