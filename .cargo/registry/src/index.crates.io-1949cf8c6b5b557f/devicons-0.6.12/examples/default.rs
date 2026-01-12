use devicons::icon_for_file;
use std::path::Path;

fn main() {
    // Create a new file instance for a directory "my_folder/"
    let path = Path::new("my_folder/");

    // Get the icon for this directory with the default theme
    let icon = icon_for_file(path, &None);

    // Print its corresponding icon
    println!("Filename: {}", path.to_string_lossy());
    println!("Icon: {}", icon.icon);
    println!("Color: {}", icon.color);
}
