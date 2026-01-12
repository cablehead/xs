use std::{collections::HashMap, fmt};

use js_sys::{JsString, Reflect};

pub const BROWSER_INTERFACE: &str = "browserif";

/// Represents a network interface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interface {
    is_up: bool,
}

impl fmt::Display for Interface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "navigator.onLine={}", self.is_up)
    }
}

impl Interface {
    async fn new() -> Self {
        let is_up = Self::is_up();
        tracing::debug!(onLine = is_up, "Fetched globalThis.navigator.onLine");
        Self {
            is_up: is_up.unwrap_or(true),
        }
    }

    fn is_up() -> Option<bool> {
        let navigator = Reflect::get(
            js_sys::global().as_ref(),
            JsString::from("navigator").as_ref(),
        )
        .ok()?;

        let is_up = Reflect::get(&navigator, JsString::from("onLine").as_ref()).ok()?;

        is_up.as_bool()
    }

    /// The name of the interface.
    pub(crate) fn name(&self) -> &str {
        BROWSER_INTERFACE
    }
}

/// Intended to store the state of the machine's network interfaces, routing table, and
/// other network configuration. For now it's pretty basic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    /// Maps from an interface name interface.
    pub interfaces: HashMap<String, Interface>,

    /// Whether this machine has an IPv6 Global or Unique Local Address
    /// which might provide connectivity.
    pub have_v6: bool,

    /// Whether the machine has some non-localhost, non-link-local IPv4 address.
    pub have_v4: bool,

    //// Whether the current network interface is considered "expensive", which currently means LTE/etc
    /// instead of Wifi. This field is not populated by `get_state`.
    pub(crate) is_expensive: bool,

    /// The interface name for the machine's default route.
    ///
    /// It is not yet populated on all OSes.
    ///
    /// When set, its value is the map key into `interface` and `interface_ips`.
    pub(crate) default_route_interface: Option<String>,

    /// The HTTP proxy to use, if any.
    pub(crate) http_proxy: Option<String>,

    /// The URL to the Proxy Autoconfig URL, if applicable.
    pub(crate) pac: Option<String>,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for iface in self.interfaces.values() {
            write!(f, "{iface}")?;
            if let Some(ref default_if) = self.default_route_interface {
                if iface.name() == default_if {
                    write!(f, " (default)")?;
                }
            }
            if f.alternate() {
                writeln!(f)?;
            } else {
                write!(f, "; ")?;
            }
        }
        Ok(())
    }
}

impl State {
    /// Returns the state of all the current machine's network interfaces.
    ///
    /// It does not set the returned `State.is_expensive`. The caller can populate that.
    pub async fn new() -> Self {
        let mut interfaces = HashMap::new();
        let have_v6 = false;
        let have_v4 = false;

        interfaces.insert(BROWSER_INTERFACE.to_string(), Interface::new().await);

        State {
            interfaces,
            have_v4,
            have_v6,
            is_expensive: false,
            default_route_interface: Some(BROWSER_INTERFACE.to_string()),
            http_proxy: None,
            pac: None,
        }
    }

    /// Is this a major change compared to the `old` one?.
    pub fn is_major_change(&self, old: &State) -> bool {
        // All changes are major.
        // In the browser, there only are changes from online to offline
        self != old
    }
}
