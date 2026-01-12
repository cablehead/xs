#[test]
fn checksum() {
    let output = std::process::Command::new("cargo")
        .env("CHECK", "1")
        .args(&["run", "-p", "generate-api", "--", "--check"])
        .output()
        .unwrap();

    println!("{}", String::from_utf8_lossy(&output.stdout));
    eprintln!("{}", String::from_utf8_lossy(&output.stderr));

    assert!(
        output.status.success(),
        "lib.rs file has been modified! Please run `cargo run -p generate-api --release`",
    )
}
