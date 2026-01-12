use devicons::FileIcon;

fn main() {
    let icon = FileIcon::from("Cargo.toml");

    println!("Icon: {}", icon.icon);
    println!("Color: {}", icon.color);
}
