use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Enum representing the TTL (Time-To-Live) for an event.
#[derive(Default, PartialEq, Eq, Clone, Debug)]
pub enum TTL {
    #[default]
    Forever, // Event is kept indefinitely.
    Ephemeral,      // Event is not stored; only active subscribers can see it.
    Time(Duration), // Event is kept for a custom duration
    Head(u32),      // Retains only the last n events for a topic (n >= 1).
}

impl TTL {
    /// Converts a `TTL` into its query string representation.
    pub fn to_query(&self) -> String {
        match self {
            TTL::Forever => "ttl=forever".to_string(),
            TTL::Ephemeral => "ttl=ephemeral".to_string(),
            TTL::Time(duration) => format!("ttl=time:{}", duration.as_millis()),
            TTL::Head(n) => format!("ttl=head:{}", n),
        }
    }

    /// Parses a `TTL` from a query string.
    pub fn from_query(query: Option<&str>) -> Result<Self, String> {
        // Parse query string into key-value pairs
        let params = match query {
            None => return Ok(TTL::default()), // Use default TTL if query is None
            Some(q) => serde_urlencoded::from_str::<HashMap<String, String>>(q)
                .map_err(|_| "invalid query string".to_string())?,
        };

        // Extract the `ttl` parameter if it exists
        if let Some(ttl_str) = params.get("ttl") {
            parse_ttl(ttl_str)
        } else {
            Ok(TTL::default()) // Use default TTL if `ttl` is not present
        }
    }
}

impl Serialize for TTL {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            TTL::Forever => serializer.serialize_str("forever"),
            TTL::Ephemeral => serializer.serialize_str("ephemeral"),
            TTL::Time(duration) => {
                serializer.serialize_str(&format!("time:{}", duration.as_millis()))
            }
            TTL::Head(n) => serializer.serialize_str(&format!("head:{}", n)),
        }
    }
}

impl<'de> Deserialize<'de> for TTL {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        parse_ttl(&s).map_err(serde::de::Error::custom)
    }
}

/// Parses a raw TTL string and converts it to the `TTL` enum.
pub fn parse_ttl(s: &str) -> Result<TTL, String> {
    match s {
        "forever" => Ok(TTL::Forever),
        "ephemeral" => Ok(TTL::Ephemeral),
        _ if s.starts_with("time:") => {
            let duration_str = &s[5..];
            let duration = duration_str
                .parse::<u64>()
                .map_err(|_| "Invalid duration for 'time' TTL".to_string())?;
            Ok(TTL::Time(Duration::from_millis(duration)))
        }
        _ if s.starts_with("head:") => {
            let n_str = &s[5..];
            let n = n_str
                .parse::<u32>()
                .map_err(|_| "Invalid 'n' value for 'head' TTL".to_string())?;
            if n < 1 {
                Err("'n' must be >= 1 for 'head' TTL".to_string())
            } else {
                Ok(TTL::Head(n))
            }
        }
        _ => Err("Invalid TTL format".to_string()),
    }
}
