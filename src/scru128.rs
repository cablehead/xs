use std::io::{self, Read};
use std::str::FromStr;

use scru128::Scru128Id;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Scru128Components {
    ts_ms: u64,
    counter_hi: u32,
    counter_lo: u32,
    node: String,
}

pub fn generate() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let id = scru128::new();
    Ok(id.to_string())
}

fn components_of(scru_id: Scru128Id) -> Scru128Components {
    Scru128Components {
        ts_ms: scru_id.timestamp(),
        counter_hi: scru_id.counter_hi(),
        counter_lo: scru_id.counter_lo(),
        node: format!("{:08x}", scru_id.entropy()),
    }
}

fn id_of(
    components: Scru128Components,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let entropy = u32::from_str_radix(&components.node, 16)?;
    let scru_id = Scru128Id::from_fields(
        components.ts_ms,
        components.counter_hi,
        components.counter_lo,
        entropy,
    );
    Ok(scru_id.to_string())
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
    Ok(serde_json::to_string_pretty(&components_of(scru_id))?)
}

pub fn unpack_to_json(
    input: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let scru_id = Scru128Id::from_str(input)?;
    Ok(serde_json::to_value(components_of(scru_id))?)
}

pub fn pack_from_json(
    json: serde_json::Value,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let components: Scru128Components = serde_json::from_value(json)?;
    id_of(components)
}

pub fn pack() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    let components: Scru128Components = serde_json::from_str(&buffer)?;
    id_of(components)
}
