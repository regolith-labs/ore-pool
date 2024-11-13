use std::collections::HashMap;
use std::str::FromStr;

use crate::error::Error;
use crate::operator::LockedMultipliers;
use actix_cors::Cors;
use actix_web::http::header;
use steel::Pubkey;

pub fn create_cors() -> Cors {
    Cors::default()
        .allowed_origin_fn(|_origin, _req_head| {
            // origin.as_bytes().ends_with(b"ore.supply") || // Production origin
            // origin == "http://localhost:8080" // Local development origin
            true
        })
        .allowed_methods(vec!["GET", "POST"]) // Methods you want to allow
        .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
        .allowed_header(header::CONTENT_TYPE)
        .max_age(3600)
}

// Add new function to load multipliers from TOML
pub fn load_locked_multipliers(
    locked_multiplier_file_path: &str,
) -> Result<LockedMultipliers, Error> {
    let content = std::fs::read_to_string(locked_multiplier_file_path)
        .map_err(|e| Error::Internal(format!("Failed to read multipliers file: {}", e)))?;

    let table: toml::Table = toml::from_str(&content)
        .map_err(|e| Error::Internal(format!("Failed to parse TOML: {}", e)))?;

    let mut multipliers = HashMap::new();

    for (key, value) in table {
        let pubkey = Pubkey::from_str(&key)
            .map_err(|e| Error::Internal(format!("Invalid pubkey {}: {}", key, e)))?;

        let array = value
            .as_array()
            .ok_or_else(|| Error::Internal(format!("Value for {} is not an array", key)))?;

        let mut pairs = Vec::new();
        for item in array {
            let tuple = item
                .as_array()
                .ok_or_else(|| Error::Internal("Multiplier entry must be an array".to_string()))?;

            if tuple.len() != 2 {
                return Err(Error::Internal(
                    "Multiplier tuple must have 2 elements".to_string(),
                ));
            }

            let timestamp = tuple[0]
                .as_integer()
                .ok_or_else(|| Error::Internal("First element must be an integer".to_string()))?;
            let multiplier = tuple[1]
                .as_integer()
                .ok_or_else(|| Error::Internal("Second element must be an integer".to_string()))?;

            pairs.push((timestamp as u64, multiplier as u64));
        }

        multipliers.insert(pubkey, pairs);
    }

    Ok(LockedMultipliers::from_map(multipliers))
}
