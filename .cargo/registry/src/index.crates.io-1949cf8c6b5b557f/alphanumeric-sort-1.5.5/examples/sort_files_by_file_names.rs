use std::path::{Path, PathBuf};

const FILES_PATH: &str = "examples/files";

fn main() {
    let files = Path::new(FILES_PATH);

    let dir_files_iter = files.read_dir().unwrap();

    let mut sub_files: Vec<PathBuf> = dir_files_iter.map(|f| f.unwrap().path()).collect();

    sub_files.sort();

    println!("Native Sort: {:#?}", sub_files);

    sub_files.sort();

    alphanumeric_sort::sort_path_slice(&mut sub_files);

    println!("Alphanumeric Sort: {:#?}", sub_files);
}
