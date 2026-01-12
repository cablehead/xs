use std::collections::HashMap;

use nested_enum_utils::common_fields;
use serde::Deserialize;
use snafu::{Backtrace, OptionExt, ResultExt, Snafu};
use tracing::warn;
use wmi::{COMLibrary, FilterValue, WMIConnection};

use super::DefaultRouteDetails;

/// API Docs: <https://learn.microsoft.com/en-us/previous-versions/windows/desktop/wmiiprouteprov/win32-ip4routetable>
#[derive(Deserialize, Debug)]
#[allow(non_camel_case_types, non_snake_case)]
struct Win32_IP4RouteTable {
    Name: String,
}

#[common_fields({
    backtrace: Option<Backtrace>,
})]
#[derive(Debug, Snafu)]
#[non_exhaustive]
pub enum Error {
    #[allow(dead_code)] // not sure why we have this here?
    #[snafu(display("IO"))]
    Io { source: std::io::Error },
    #[snafu(display("not route found"))]
    NoRoute {},
    #[snafu(display("WMI"))]
    Wmi { source: wmi::WMIError },
}

fn get_default_route() -> Result<DefaultRouteDetails, Error> {
    let com_con = COMLibrary::new().context(WmiSnafu)?;
    let wmi_con = WMIConnection::new(com_con).context(WmiSnafu)?;

    let query: HashMap<_, _> = [("Destination".into(), FilterValue::Str("0.0.0.0"))].into();
    let route: Win32_IP4RouteTable = wmi_con
        .filtered_query(&query)
        .context(WmiSnafu)?
        .drain(..)
        .next()
        .context(NoRouteSnafu)?;

    Ok(DefaultRouteDetails {
        interface_name: route.Name,
    })
}

pub async fn default_route() -> Option<DefaultRouteDetails> {
    match get_default_route() {
        Ok(route) => Some(route),
        Err(err) => {
            warn!("failed to retrieve default route: {:#?}", err);
            None
        }
    }
}
