use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Enum representing the TTL (Time-To-Live) for an event.
#[derive(Default, PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TTL {
    #[default]
    Forever, // Event is kept indefinitely.
    Ephemeral,      // Event is not stored; only active subscribers can see it.
    Time(Duration), // Event is kept for a custom duration in seconds.
    Head(u32),      // Retains only the last n events for a topic (n >= 1).
}

impl TTL {
    /// Converts a `TTL` into its query string representation.
    pub fn to_query(&self) -> String {
        match self {
            TTL::Forever => "ttl=forever".to_string(),
            TTL::Ephemeral => "ttl=ephemeral".to_string(),
            TTL::Time(duration) => format!("ttl=time&duration={}", duration.as_secs()),
            TTL::Head(n) => format!("ttl=head&n={}", n),
        }
    }

    /// Parses a `TTL` from a query string.
    pub fn from_query(query: Option<&str>) -> Result<Self, String> {
        let params = match query {
            None => return Ok(TTL::Forever), // Default to Forever
            Some(q) => serde_urlencoded::from_str::<HashMap<String, String>>(q)
                .map_err(|_| "invalid query string".to_string())?,
        };

        // Extract the `ttl` string
        let ttl_str = params.get("ttl").ok_or("missing ttl type")?;

        // Handle additional parameters for specific TTL types
        match ttl_str.as_str() {
            "forever" => Ok(TTL::Forever),
            "ephemeral" => Ok(TTL::Ephemeral),
            "time" => {
                let duration = params
                    .get("duration")
                    .ok_or("missing duration for 'time' TTL".to_string())?
                    .parse::<u64>()
                    .map_err(|_| "invalid duration for 'time' TTL".to_string())?;
                Ok(TTL::Time(Duration::from_secs(duration)))
            }
            "head" => {
                let n = params
                    .get("n")
                    .ok_or("missing 'n' for 'head' TTL".to_string())?
                    .parse::<u32>()
                    .map_err(|_| "invalid 'n' value for 'head' TTL".to_string())?;
                if n < 1 {
                    Err("'n' must be >= 1 for 'head' TTL".to_string())
                } else {
                    Ok(TTL::Head(n))
                }
            }
            _ => Err("invalid ttl type".to_string()),
        }
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
            Ok(TTL::Time(Duration::from_secs(duration)))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize() {
        let ttl: TTL = Default::default();
        let serialized = serde_json::to_string(&ttl).unwrap();
        assert_eq!(serialized, r#""forever""#);

        let ttl = TTL::Time(Duration::from_secs(1));
        let serialized = serde_json::to_string(&ttl).unwrap();
        assert_eq!(serialized, r#"{"time":{"secs":1,"nanos":0}}"#);
    }

    #[test]
    fn test_parse_ttl() {
        assert_eq!(parse_ttl("forever"), Ok(TTL::Forever));
        assert_eq!(parse_ttl("ephemeral"), Ok(TTL::Ephemeral));
        assert_eq!(
            parse_ttl("time:3600"),
            Ok(TTL::Time(Duration::from_secs(3600)))
        );
        assert_eq!(parse_ttl("head:3"), Ok(TTL::Head(3)));

        // Invalid cases
        assert!(parse_ttl("time:abc").is_err());
        assert!(parse_ttl("head:0").is_err());
        assert!(parse_ttl("unknown").is_err());
    }

    #[test]
    fn test_from_query() {
        assert_eq!(TTL::from_query(None), Ok(TTL::Forever));
        assert_eq!(TTL::from_query(Some("ttl=forever")), Ok(TTL::Forever));
        assert_eq!(TTL::from_query(Some("ttl=ephemeral")), Ok(TTL::Ephemeral));
        assert_eq!(
            TTL::from_query(Some("ttl=time&duration=3600")),
            Ok(TTL::Time(Duration::from_secs(3600)))
        );
        assert_eq!(TTL::from_query(Some("ttl=head&n=2")), Ok(TTL::Head(2)));

        // Invalid cases
        assert!(TTL::from_query(Some("ttl=time")).is_err()); // Missing duration
        assert!(TTL::from_query(Some("ttl=head")).is_err()); // Missing n
        assert!(TTL::from_query(Some("ttl=head&n=0")).is_err()); // Invalid n
        assert!(TTL::from_query(Some("ttl=invalid")).is_err()); // Invalid type
    }
}
