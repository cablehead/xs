use std::io::{self, Read};
use std::str::FromStr;

use scru128::Scru128Id;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Scru128Components {
    timestamp: f64,
    counter_hi: u32,
    counter_lo: u32,
    node: String,
}

pub fn generate() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let id = scru128::new();
    Ok(id.to_string())
}

pub fn unpack(input: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let id = if input == "-" {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer.trim().to_string()
    } else {
        input.to_string()
    };

    let scru_id = Scru128Id::from_str(&id)?;

    let timestamp = scru_id.timestamp() as f64 / 1000.0;
    let counter_hi = scru_id.counter_hi();
    let counter_lo = scru_id.counter_lo();
    let node = format!("{:08x}", scru_id.entropy());

    let components = Scru128Components {
        timestamp,
        counter_hi,
        counter_lo,
        node,
    };

    Ok(serde_json::to_string_pretty(&components)?)
}

pub fn unpack_to_json(
    input: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let scru_id = Scru128Id::from_str(input)?;

    let timestamp = scru_id.timestamp() as f64 / 1000.0;
    let counter_hi = scru_id.counter_hi();
    let counter_lo = scru_id.counter_lo();
    let node = format!("{:08x}", scru_id.entropy());

    let components = Scru128Components {
        timestamp,
        counter_hi,
        counter_lo,
        node,
    };

    Ok(serde_json::to_value(components)?)
}

pub fn pack_from_json(
    json: serde_json::Value,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let components: Scru128Components = serde_json::from_value(json)?;

    let timestamp = (components.timestamp * 1000.0) as u64;
    let entropy = u32::from_str_radix(&components.node, 16)?;

    let scru_id = Scru128Id::from_fields(
        timestamp,
        components.counter_hi,
        components.counter_lo,
        entropy,
    );

    Ok(scru_id.to_string())
}

pub fn pack() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    let components: Scru128Components = serde_json::from_str(&buffer)?;

    let timestamp = (components.timestamp * 1000.0) as u64;
    let entropy = u32::from_str_radix(&components.node, 16)?;

    let scru_id = Scru128Id::from_fields(
        timestamp,
        components.counter_hi,
        components.counter_lo,
        entropy,
    );

    Ok(scru_id.to_string())
}
