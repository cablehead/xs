use std::io;
use std::path::Path;

use filesize::PathExt;

fn display_path<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref();
    let meta = path.symlink_metadata()?;
    let logical = meta.len();
    let physical = path.size_on_disk_fast(&meta)?;

    println!(
        "{:>9} {:>9} {:>9.2}x {}",
        logical,
        physical,
        physical as f64 / logical as f64,
        path.display()
    );

    Ok(())
}

fn main() -> io::Result<()> {
    println!("{:>9} {:>9} {:>9} Path", "Logical", "Physical", "Ratio");

    for path in std::env::args_os().skip(1) {
        if let Err(e) = display_path(&path) {
            eprintln!("{}: {}", path.to_string_lossy(), e);
        }
    }

    Ok(())
}
