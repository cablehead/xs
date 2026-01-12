use devicons::{icon_for_file, Theme};
use std::path::Path;

fn main() {
    // Create a new file instance for "example.txt"
    let path = Path::new("example.txt");

    // Get the icon for this file with the dark theme
    let icon = icon_for_file(path, &Some(Theme::Dark));

    // Print its corresponding icon
    println!("Filename: {}", path.to_string_lossy());
    println!("Icon: {}", icon.icon);
    println!("Color: {}", icon.color);
}
