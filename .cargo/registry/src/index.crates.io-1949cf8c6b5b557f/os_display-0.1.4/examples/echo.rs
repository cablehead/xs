use os_display::Quotable;

fn main() {
    for arg in std::env::args_os().skip(1) {
        println!("Native: {}", arg.maybe_quote());
        #[cfg(all(windows, feature = "unix"))]
        {
            if let Some(arg) = arg.to_str() {
                println!("Unix: {}", os_display::Quoted::unix(arg).force(false));
            }
        }
        #[cfg(all(not(windows), feature = "windows"))]
        {
            if let Some(arg) = arg.to_str() {
                println!("Windows: {}", os_display::Quoted::windows(arg).force(false));
            }
        }
    }
}
