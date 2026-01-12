/// display the content of the current directory

use {
    std::{env, io, path::PathBuf},
    umask::Mode,
};

fn list_files() -> io::Result<()> {
    let root = env::current_dir()?;
    println!("Current dir: {}", root.to_string_lossy());
    let mut paths: Vec<PathBuf> = root.read_dir()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    paths.sort_unstable();
    for path in paths {
        let mode = Mode::try_from(&path)?;
        let name = path.file_name().unwrap().to_string_lossy();
        println!("{}  {}", mode, name);
    }
    Ok(())
}

fn main() -> io::Result<()> {
    list_files()
}
