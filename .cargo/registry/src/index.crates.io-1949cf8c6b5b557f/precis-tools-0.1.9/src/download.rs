//! Module for downloading files from the internet.

use crate::Error;
use reqwest::header::USER_AGENT;
use std::env;
use std::fs;
use std::path::Path;

fn get_csv_file_name(ucd_version: &str) -> String {
    format!("precis-tables-{}.csv", ucd_version)
}

fn get_precis_csv_tables_uri(ucd_version: &str) -> String {
    format!(
        "https://www.iana.org/assignments/precis-tables-{}/{}",
        ucd_version,
        get_csv_file_name(ucd_version)
    )
}

fn get_unicode_ucd_uri(ucd_version: &str) -> String {
    format!("https://www.unicode.org/Public/{}/ucd", ucd_version)
}

fn download(url: &str, dest: &Path) -> Result<(), Error> {
    let pkg_name = env!("CARGO_PKG_NAME");

    let client = reqwest::blocking::Client::new();
    let text = client
        .get(url)
        .header(USER_AGENT, pkg_name)
        .send()
        .unwrap()
        .text()
        .unwrap();
    Ok(fs::write(dest, text)?)
}

/// Gets a ucd file from the Internet
/// # Arguments
/// * `ucd_version`: Unicode version
/// * `dest`: Destination directory
/// * `file`: File name
/// # Returns
/// `Ok(())` if the file was downloaded successfully, `Err(Error)` otherwise
pub fn get_ucd_file(ucd_version: &str, dest: &Path, file: &str) -> Result<(), Error> {
    let url = format!("{}/{}", get_unicode_ucd_uri(ucd_version), file);
    let dest_path = dest.join(file);
    download(&url, &dest_path)
}

/// Gets a csv file from the Internet
/// # Arguments
/// * `ucd_version`: Unicode version
/// * `dest`: Destination directory
/// # Returns
/// `Ok(())` if the file was downloaded successfully, `Err(Error)` otherwise
pub fn get_csv_file(ucd_version: &str, dest: &Path) -> Result<(), Error> {
    let dest_path = dest.join(get_csv_file_name(ucd_version));
    download(&get_precis_csv_tables_uri(ucd_version), &dest_path)
}
