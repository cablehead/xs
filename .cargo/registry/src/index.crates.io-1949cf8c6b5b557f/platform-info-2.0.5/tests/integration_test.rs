// spell-checker:ignore (API) nodename osname sysname

use platform_info::*;

#[test]
fn platform() -> Result<(), String> {
    let info = PlatformInfo::new().unwrap();

    let sysname = info.sysname().to_string_lossy();
    let nodename = info.nodename().to_string_lossy();
    let release = info.release().to_string_lossy();
    let version = info.version().to_string_lossy();
    let machine = info.machine().to_string_lossy();
    let osname = info.osname().to_string_lossy();

    assert!(!sysname.is_empty());
    assert!(!nodename.is_empty());
    assert!(!release.is_empty());
    assert!(!version.is_empty());
    assert!(!machine.is_empty());
    assert!(!osname.is_empty());

    // assert!(false);
    Ok(())
}

#[test]
fn platform_no_invisible_contents() -> Result<(), String> {
    let info = PlatformInfo::new().unwrap();

    let sysname = info.sysname().to_string_lossy();
    let nodename = info.nodename().to_string_lossy();
    let release = info.release().to_string_lossy();
    let version = info.version().to_string_lossy();
    let machine = info.machine().to_string_lossy();
    let osname = info.osname().to_string_lossy();

    let s = format!("sysname='{sysname}';nodename='{nodename}';release='{release}';version='{version}';machine='{machine}';osname='{osname}'");
    println!("s = [{}]\"{}\"", s.len(), s);

    // let re = regex::Regex::new("[^[[:print:]]]").unwrap(); // matches invisible (and emojis)
    let re = regex::Regex::new("[^[[:print:]]\\p{Other_Symbol}]").unwrap(); // matches invisible only (not emojis)
    assert_eq!(re.find(&s), None);

    Ok(())
}

#[test]
fn platform_clone() -> Result<(), String> {
    let info = PlatformInfo::new().unwrap();
    #[allow(clippy::redundant_clone)] // ignore `clippy::redundant_clone` warning for direct testing
    let info_copy = info.clone();
    println!("{info:?}");
    assert_eq!(info_copy, info);
    Ok(())
}
