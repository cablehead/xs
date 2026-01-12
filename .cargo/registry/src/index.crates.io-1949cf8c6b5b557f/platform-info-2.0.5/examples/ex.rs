// examples/ex.rs
// * use `cargo run --example ex` to execute this example

// spell-checker:ignore (API) nodename osname sysname

use platform_info::{PlatformInfo, PlatformInfoAPI, UNameAPI};

fn main() {
    let info = PlatformInfo::new().expect("Unable to determine platform info");
    // println!("info={:#?}", info); // note: info *may* contain extra platform-specific fields

    println!("{}", info.sysname().to_string_lossy());
    println!("{}", info.nodename().to_string_lossy());
    println!("{}", info.release().to_string_lossy());
    println!("{}", info.version().to_string_lossy());
    println!("{}", info.machine().to_string_lossy());
    println!("{}", info.osname().to_string_lossy());
}
