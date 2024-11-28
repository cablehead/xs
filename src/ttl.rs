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
            TTL::Time(duration) => format!("ttl=time:{}", duration.as_secs()),
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
    fn test_to_query() {
        assert_eq!(TTL::Forever.to_query(), "ttl=forever");
        assert_eq!(TTL::Ephemeral.to_query(), "ttl=ephemeral");
        assert_eq!(
            TTL::Time(Duration::from_secs(3600)).to_query(),
            "ttl=time:3600"
        );
        assert_eq!(TTL::Head(2).to_query(), "ttl=head:2");
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

        // Default TTL when `ttl` is missing but query exists
        assert_eq!(TTL::from_query(Some("foo=bar")), Ok(TTL::Forever));

        // Invalid cases
        assert!(TTL::from_query(Some("ttl=time")).is_err()); // Missing duration
        assert!(TTL::from_query(Some("ttl=head")).is_err()); // Missing n
        assert!(TTL::from_query(Some("ttl=head&n=0")).is_err()); // Invalid n
        assert!(TTL::from_query(Some("ttl=invalid")).is_err()); // Invalid type
    }

    #[test]
    fn test_ttl_round_trip() {
        let ttls = vec![
            TTL::Forever,
            TTL::Ephemeral,
            TTL::Time(Duration::from_secs(3600)),
            TTL::Head(2),
        ];

        for ttl in ttls {
            let query = ttl.to_query();
            let parsed = TTL::from_query(Some(&query)).expect("Failed to parse query");
            assert_eq!(parsed, ttl, "Round trip failed for TTL: {:?}", ttl);
        }
    }

    #[test]
    fn test_ttl_json_round_trip() {
        // Define the TTL variants to test
        let ttls = vec![
            TTL::Forever,
            TTL::Ephemeral,
            TTL::Time(Duration::from_secs(3600)),
            TTL::Head(2),
        ];

        for ttl in ttls {
            // Serialize TTL to JSON
            let json = serde_json::to_string(&ttl).expect("Failed to serialize TTL to JSON");

            eprintln!("json: {}", json);

            // Deserialize JSON back into TTL
            let deserialized: TTL =
                serde_json::from_str(&json).expect("Failed to deserialize JSON back to TTL");

            // Assert that the deserialized value matches the original
            assert_eq!(
                deserialized, ttl,
                "JSON round-trip failed for TTL: {:?}",
                ttl
            );
        }
    }
}
