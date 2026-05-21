//! Resolves the [`iroh::RelayMode`] from an `XS_RELAY_URL` environment variable.
//!
//! xs's iroh endpoints (both server-side in [`crate::listener`] and client-side
//! in [`crate::client::connect`]) historically hardcoded [`RelayMode::Default`],
//! which routes via n0's public relay infrastructure. That's the right default,
//! but operators have legitimate reasons to override:
//!
//! - **Regions where n0's defaults aren't reachable.** The Asia-Pacific
//!   regional relay (`apse-1.relay.iroh.network`) currently has no public DNS
//!   A record, so iroh nodes in that region hang on
//!   `home_relay().initialized().await`. Operators self-host a relay in the
//!   region and point xs at it.
//! - **Compliance / air-gapped networks** where outbound to relay.iroh.network
//!   isn't permitted.
//! - **Local testing** with a self-hosted dev relay.
//!
//! Setting `XS_RELAY_URL=https://my-relay.example.com` makes both the listener
//! and the connect path use [`RelayMode::Custom`] with that single relay.
//! Setting `XS_RELAY_URL=disabled` selects [`RelayMode::Disabled`] (no relay at
//! all — works only when peers are directly reachable, e.g. on a LAN/VPC).
//! Empty/unset preserves the existing [`RelayMode::Default`] behavior so this
//! patch is a strict superset.

use iroh::{RelayMap, RelayMode};
use iroh_base::RelayUrl;

/// The env var consulted by [`relay_mode_from_env`].
pub const ENV_VAR: &str = "XS_RELAY_URL";

/// Resolves the relay configuration from the [`ENV_VAR`] env var.
///
/// - Empty or unset → [`RelayMode::Default`] (n0 public relays — the existing default).
/// - `"disabled"` (case-insensitive) → [`RelayMode::Disabled`].
/// - Otherwise treated as a relay URL → [`RelayMode::Custom`] with a single-entry [`RelayMap`].
///   If the URL fails to parse, logs a warning and falls back to [`RelayMode::Default`]
///   so a typo doesn't break otherwise-working setups.
pub fn relay_mode_from_env() -> RelayMode {
    let raw = std::env::var(ENV_VAR).unwrap_or_default();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return RelayMode::Default;
    }
    if trimmed.eq_ignore_ascii_case("disabled") {
        tracing::info!("{ENV_VAR}=disabled — using RelayMode::Disabled");
        return RelayMode::Disabled;
    }
    match trimmed.parse::<RelayUrl>() {
        Ok(url) => {
            tracing::info!("{ENV_VAR}={trimmed} — using RelayMode::Custom");
            RelayMode::Custom(RelayMap::from(url))
        }
        Err(err) => {
            tracing::warn!(
                "{ENV_VAR}={trimmed} failed to parse as RelayUrl ({err}); falling back to RelayMode::Default"
            );
            RelayMode::Default
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests temporarily set env vars; serialise them to avoid cross-test races.
    fn lock() -> std::sync::MutexGuard<'static, ()> {
        static M: std::sync::Mutex<()> = std::sync::Mutex::new(());
        M.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn unset_returns_default() {
        let _g = lock();
        // SAFETY: under the test mutex, no concurrent env access.
        unsafe { std::env::remove_var(ENV_VAR) };
        assert_eq!(relay_mode_from_env(), RelayMode::Default);
    }

    #[test]
    fn empty_returns_default() {
        let _g = lock();
        unsafe { std::env::set_var(ENV_VAR, "") };
        assert_eq!(relay_mode_from_env(), RelayMode::Default);
        unsafe { std::env::remove_var(ENV_VAR) };
    }

    #[test]
    fn whitespace_returns_default() {
        let _g = lock();
        unsafe { std::env::set_var(ENV_VAR, "   ") };
        assert_eq!(relay_mode_from_env(), RelayMode::Default);
        unsafe { std::env::remove_var(ENV_VAR) };
    }

    #[test]
    fn disabled_keyword_returns_disabled() {
        let _g = lock();
        unsafe { std::env::set_var(ENV_VAR, "DISABLED") };
        assert_eq!(relay_mode_from_env(), RelayMode::Disabled);
        unsafe { std::env::remove_var(ENV_VAR) };
    }

    #[test]
    fn valid_url_returns_custom() {
        let _g = lock();
        unsafe { std::env::set_var(ENV_VAR, "https://relay.example.com") };
        match relay_mode_from_env() {
            RelayMode::Custom(map) => assert_eq!(map.len(), 1),
            other => panic!("expected RelayMode::Custom, got {other:?}"),
        }
        unsafe { std::env::remove_var(ENV_VAR) };
    }

    #[test]
    fn malformed_url_falls_back_to_default() {
        let _g = lock();
        unsafe { std::env::set_var(ENV_VAR, "not a url") };
        assert_eq!(relay_mode_from_env(), RelayMode::Default);
        unsafe { std::env::remove_var(ENV_VAR) };
    }
}
