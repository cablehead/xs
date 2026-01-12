// This file is part of the uutils coreutils package.
//
// (c) Alex Lyon <arcterus@mail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.

// Note: there no standardization of values for platform info (or `uname`), so mimic some current practices
// busybox-v1.35.0 * `busybox uname -a` => "Windows_NT HOSTNAME 10.0 19044 x86_64 MS/Windows"
// python-v3.8.3 => `uname_result(system='Windows', node='HOSTNAME', release='10', version='10.0.19044', machine='AMD64')`

// refs/research:
// [rust ~ std::ffi](https://doc.rust-lang.org/std/ffi)
// [rust ~ std::os::windows::ffi](https://doc.rust-lang.org/std/os/windows/ffi)
// [WTF-8/WTF-16](https://simonsapin.github.io/wtf-8/#ill-formed-utf-16) @@ <https://archive.is/MG7Aa>
// [UCS-2/UTF-8/UTF-16](https://unascribed.com/b/2019-08-02-the-tragedy-of-ucs2.html) @@ <https://archive.is/x4SxI>
// [Byte-to/from-String Conversions](https://nicholasbishop.github.io/rust-conversions) @@ <https://archive.is/AnDCY>
// [NT Version Info](https://en.wikipedia.org/wiki/Windows_NT) @@ <https://archive.is/GnnvF>
// [NT Version Info (summary)](https://simple.wikipedia.org/wiki/Windows_NT) @@ <https://archive.is/T2StZ>
// [NT Version Info (detailed)](https://en.wikipedia.org/wiki/Comparison_of_Microsoft_Windows_versions#Windows_NT) @@ <https://archive.is/FSkhj>

// spell-checker:ignore (abbrev/acronyms) MSVC POSIX SuperH
// spell-checker:ignore (API) sysname osname nodename
// spell-checker:ignore (jargon) armv aarch hasher mmbr
// spell-checker:ignore (people) Roy Ivy III * rivy
// spell-checker:ignore (rust) repr stdcall uninit
// spell-checker:ignore (uutils) coreutils uutils
// spell-checker:ignore (WinAPI) ctypes CWSTR DWORDLONG dwStrucVersion FARPROC FIXEDFILEINFO HIWORD HMODULE libloaderapi LOWORD LPCSTR LPCVOID LPCWSTR lpdw LPDWORD lplp LPOSVERSIONINFOEXW LPSYSTEM lptstr LPVOID LPWSTR minwindef ntdef ntstatus OSVERSIONINFOEXW processthreadsapi PUINT SMALLBUSINESS SUITENAME sysinfo sysinfoapi sysinfoapi TCHAR TCHARs ULONGLONG VERSIONINFO WCHAR WCHARs winapi winbase winver WSTR wstring
// spell-checker:ignore (WinOS) ntdll

#![warn(unused_results)] // enable warnings for unused results

use std::convert::TryFrom;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::io;
use std::os::windows::ffi::OsStringExt;

use winapi::shared::minwindef::*;
use winapi::um::sysinfoapi::*;
use winapi::um::winnt::*;

use crate::{PlatformInfoAPI, PlatformInfoError, UNameAPI};

use super::PathStr;
use super::PathString;

type WinOSError = crate::lib_impl::BoxedThreadSafeStdError;

mod windows_safe;
use windows_safe::*;

//===

// PlatformInfo
/// Handles initial retrieval and holds cached information for the current platform (Windows/WinOS in this case).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformInfo {
    /// Cached computer name.
    pub computer_name: OsString,
    /// Wraps a cached [`WinApiSystemInfo`].
    pub system_info: WinApiSystemInfo,
    /// Wraps a cached [`WinOsVersionInfo`].
    pub version_info: WinOsVersionInfo,
    // * private-use fields
    sysname: OsString,
    nodename: OsString,
    release: OsString,
    version: OsString,
    machine: OsString,
    osname: OsString,
}

impl PlatformInfoAPI for PlatformInfo {
    // * note: due to the method of information retrieval, this *may* fail
    fn new() -> Result<Self, PlatformInfoError> {
        let computer_name = WinOsGetComputerName()?;
        let system_info = WinApiSystemInfo(WinAPI_GetNativeSystemInfo());
        let version_info = os_version_info()?;

        let sysname = determine_sysname();
        let nodename = computer_name.clone();
        let release = version_info.release.clone();
        let version = version_info.version.clone();
        let machine = determine_machine(&system_info);
        let osname = determine_osname(&version_info);

        Ok(Self {
            computer_name,
            system_info,
            version_info,
            /* private use */
            sysname,
            nodename,
            release,
            version,
            machine,
            osname,
        })
    }
}

impl UNameAPI for PlatformInfo {
    fn sysname(&self) -> &OsStr {
        &self.sysname
    }

    fn nodename(&self) -> &OsStr {
        &self.nodename
    }

    fn release(&self) -> &OsStr {
        &self.release
    }

    fn version(&self) -> &OsStr {
        &self.version
    }

    fn machine(&self) -> &OsStr {
        &self.machine
    }

    fn osname(&self) -> &OsStr {
        &self.osname
    }
}

//===

// WinApiSystemInfo
/// Contains information about the current computer system.
///
/// Wraps [SYSTEM_INFO](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/ns-sysinfoapi-system_info).
#[derive(Clone, Copy /* , Debug, PartialEq, Eq *//* note: implemented elsewhere */)]
pub struct WinApiSystemInfo(
    // ref: <https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/ns-sysinfoapi-system_info> @@ <https://archive.is/cqbrj>
    SYSTEM_INFO,
);

// WinOsVersionInfo
/// Contains WinOS version information as [OsString]'s; for more info, see [NT Version Info (detailed)](https://en.wikipedia.org/wiki/Comparison_of_Microsoft_Windows_versions#Windows_NT).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinOsVersionInfo {
    // ref: [NT Version Info (detailed)](https://en.wikipedia.org/wiki/Comparison_of_Microsoft_Windows_versions#Windows_NT) @@ <https://archive.is/FSkhj>
    /// "Friendly" OS name (eg, "Windows 10")
    pub os_name: OsString,
    /// General/main OS version (eg, "10.0")
    pub release: OsString,
    /// Specific OS version (eg, "19045")
    pub version: OsString,
}

//===

pub mod util {
    use std::ffi::CString;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    use winapi::um::winnt::*;

    /// WinOS wide-character string buffer
    /// <br>Note: `WCHAR` (aka `TCHAR`) == `wchar_t` == `u16`
    #[allow(clippy::upper_case_acronyms)]
    pub type WSTR = Vec<WCHAR>;
    /// NUL-terminated WinOS wide-character string buffer
    /// <br>Note: `WCHAR` (aka `TCHAR`) == `wchar_t` == `u16`
    #[allow(clippy::upper_case_acronyms)]
    pub type CWSTR = Vec<WCHAR>;

    // to_c_string()
    /// Convert the leading non-NUL content of any string (which is cheaply convertible to an OsStr) into a CString, without error.
    ///
    /// Any non-Unicode sequences are replaced with [U+FFFD (REPLACEMENT CHARACTER)](https://en.wikipedia.org/wiki/Specials_(Unicode_block)).
    pub fn to_c_string<S: AsRef<OsStr>>(os_str: S) -> CString {
        let nul = '\0';
        let s = os_str.as_ref().to_string_lossy();
        let leading_s = s.split(nul).next().unwrap_or(""); // string slice of leading non-NUL characters

        let maybe_c_string = CString::new(leading_s);
        assert!(maybe_c_string.is_ok()); //* failure here == algorithmic/logic error => panic
        maybe_c_string.unwrap()
    }

    /// Convert the leading non-NUL content of any string (which is cheaply convertible to an OsStr) into a CWSTR, without error.
    pub fn to_c_wstring<S: AsRef<OsStr>>(os_str: S) -> CWSTR {
        let nul: WCHAR = 0;
        let mut wstring: WSTR = os_str.as_ref().encode_wide().collect();
        wstring.push(nul);

        let maybe_index_first_nul = wstring.iter().position(|&i| i == nul);
        assert!(maybe_index_first_nul.is_some()); //* failure here == algorithmic/logic error => panic
        let index_first_nul = maybe_index_first_nul.unwrap();
        assert!(index_first_nul < wstring.len()); //* failure here == algorithmic/logic error => panic
        CWSTR::from(&wstring[..(index_first_nul + 1)])
    }
}

//===

// MmbrVersion
/// Contains a version specification as major, minor, build, and revision DWORDs (ie, from *major*.*minor*.*build*.*release* version style).
#[derive(Clone, Debug, PartialEq, Eq)]
struct MmbrVersion {
    major: DWORD,
    minor: DWORD,
    build: DWORD,
    release: DWORD,
}

// WinApiFileVersionInfo
/// Contains file version info (`VS_VERSIONINFO`) wrapped as a byte vector (`data`).
///
/// Wraps [VS_VERSIONINFO](https://learn.microsoft.com/en-us/windows/win32/menurc/vs-versioninfo).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinApiFileVersionInfo {
    data: Vec<BYTE>,
}

//===

impl Debug for WinApiSystemInfo {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("WinApiSystemInfo")
            .field("wProcessorArchitecture", &self.wProcessorArchitecture())
            .field("dwPageSize", &self.0.dwPageSize)
            .field(
                "lpMinimumApplicationAddress",
                &self.0.lpMinimumApplicationAddress,
            )
            .field(
                "lpMaximumApplicationAddress",
                &self.0.lpMaximumApplicationAddress,
            )
            .field("dwActiveProcessorMask", &self.0.dwActiveProcessorMask)
            .field("dwNumberOfProcessors", &self.0.dwNumberOfProcessors)
            .field("dwProcessorType", &self.0.dwProcessorType)
            .field("dwAllocationGranularity", &self.0.dwAllocationGranularity)
            .field("wAllocationGranularity", &self.0.wProcessorLevel)
            .field("wAllocationRevision", &self.0.wProcessorRevision)
            .finish()
    }
}

impl PartialEq for WinApiSystemInfo {
    fn eq(&self, other: &Self) -> bool {
        (
            self.wProcessorArchitecture(),
            self.0.dwPageSize,
            self.0.lpMinimumApplicationAddress,
            self.0.lpMaximumApplicationAddress,
            self.0.dwActiveProcessorMask,
            self.0.dwNumberOfProcessors,
            self.0.dwProcessorType,
            self.0.dwAllocationGranularity,
            self.0.wProcessorLevel,
            self.0.wProcessorRevision,
        ) == (
            other.wProcessorArchitecture(),
            other.0.dwPageSize,
            other.0.lpMinimumApplicationAddress,
            other.0.lpMaximumApplicationAddress,
            other.0.dwActiveProcessorMask,
            other.0.dwNumberOfProcessors,
            other.0.dwProcessorType,
            other.0.dwAllocationGranularity,
            other.0.wProcessorLevel,
            other.0.wProcessorRevision,
        )
    }
}

impl Eq for WinApiSystemInfo {}

//===

// WinOSGetComputerName
/// *Returns* a NetBIOS or DNS name associated with the local computer.
#[allow(non_snake_case)]
fn WinOsGetComputerName() -> Result<OsString, WinOSError> {
    //## NameType ~ using "ComputerNameDnsHostname" vs "ComputerNamePhysicalDnsHostname"
    // * "ComputerNamePhysicalDnsHostname" *may* have a different (more specific) name when in a DNS cluster
    // * `uname -n` may show the more specific cluster name (see https://clusterlabs.org/pacemaker/doc/deprecated/en-US/Pacemaker/1.1/html/Clusters_from_Scratch/_short_node_names.html)
    // * under Linux/Wine, they are *exactly* the same ([from Wine patches msgs](https://www.winehq.org/pipermail/wine-patches/2002-November/004080.html))
    // * probably want the more specific in-cluster name, but, functionally, any difference will be very rare
    // ref: [COMPUTER_NAME_FORMAT](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/ne-sysinfoapi-computer_name_format) @@ <https://archive.is/s18y0>
    let name_type = ComputerNamePhysicalDnsHostname; // or ComputerNameDnsHostname

    let mut size: DWORD = 0;
    let _ = WinAPI_GetComputerNameExW(name_type, None, &mut size);
    let mut data = vec![0; usize::try_from(size)?];
    let result = WinAPI_GetComputerNameExW(name_type, &mut data, &mut size);
    if result == FALSE {
        return Err(Box::new(io::Error::last_os_error()));
    }
    Ok(OsString::from_wide(&data[..usize::try_from(size)?]))
}

// WinOsGetFileVersionInfo
/// *Returns* the file version information block for the specified file (`file_path`).
#[allow(non_snake_case)]
fn WinOsGetFileVersionInfo<P: AsRef<PathStr>>(
    file_path: P,
) -> Result<WinApiFileVersionInfo, WinOSError> {
    let file_version_size = WinAPI_GetFileVersionInfoSizeW(&file_path);
    if file_version_size == 0 {
        return Err(Box::new(io::Error::last_os_error()));
    }
    let mut data: Vec<BYTE> = vec![0; usize::try_from(file_version_size)?];
    let result = WinAPI_GetFileVersionInfoW(&file_path, &mut data);
    if result == FALSE {
        return Err(Box::new(io::Error::last_os_error()));
    }
    Ok(WinApiFileVersionInfo { data })
}

// WinOSGetSystemDirectory
/// *Returns* a resolved path to the Windows System Directory (aka `%SystemRoot%`).
#[allow(non_snake_case)]
fn WinOsGetSystemDirectory() -> Result<PathString, WinOSError> {
    let required_capacity: UINT = WinAPI_GetSystemDirectoryW(None);
    let mut data = vec![0; usize::try_from(required_capacity)?];
    let result = WinAPI_GetSystemDirectoryW(&mut data);
    if result == 0 {
        return Err(Box::new(io::Error::last_os_error()));
    }
    let path = PathString::from(OsString::from_wide(&data[..usize::try_from(result)?]));
    Ok(path)
}

//===

// os_version_info
/// *Returns* OS version info (as [`WinOsVersionInfo`]) using a DLL procedure call, with fallback version info from a
/// known system file.
///
/// This call and fallback recipe is necessary because Microsoft deprecated the previously used `GetVersionEx()`, making
/// it useless for Windows 8.1 and later windows versions.
// ref: <https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getversionexw> @@ <https://archive.is/bYAwT>
// ref: <https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-osversioninfoexw> @@ <https://archive.is/n4hBb>
fn os_version_info() -> Result<WinOsVersionInfo, WinOSError> {
    match os_version_info_from_dll() {
        Ok(os_info) => Ok(os_info),
        Err(_) => {
            // as a last resort, try to get the relevant info by loading the version info from a system file
            // Note: this file version may be just the current "base" version and not the actual most up-to-date version info
            // * eg: kernel32.dll (or ntdll.dll) version => "10.0.19041.2130" _vs_ `cmd /c ver` => "10.0.19044.2364"
            version_info_from_file("" /* use default file */)
            // .or. `return version_info_from_file::<_, &str>(None /* use default file */);`
        }
    }
}

// os_version_info_from_dll
/// *Returns* version info (as [`WinOsVersionInfo`]) obtained via `NTDLL/RtlGetVersion()`.
fn os_version_info_from_dll() -> Result<WinOsVersionInfo, WinOSError> {
    let os_info = NTDLL_RtlGetVersion()?;
    Ok(WinOsVersionInfo {
        os_name: winos_name(
            os_info.dwMajorVersion,
            os_info.dwMinorVersion,
            os_info.dwBuildNumber,
            os_info.wProductType,
            os_info.wSuiteMask.into(),
        )
        .into(),
        release: format!("{}.{}", os_info.dwMajorVersion, os_info.dwMinorVersion).into(),
        version: format!("{}", os_info.dwBuildNumber).into(),
    })
}

// version_info_from_file
/// *Returns* version info (as [`WinOsVersionInfo`]) obtained from `file_path`.
///
/// `file_path` ~ if empty or `None`, default to the full path of "kernel32.dll" (a known, omnipresent, system file)
fn version_info_from_file<I, P>(file_path: I) -> Result<WinOsVersionInfo, WinOSError>
where
    I: Into<Option<P>>,
    P: AsRef<PathStr>,
{
    let file_path: PathString = match file_path.into() {
        Some(ref p) if !p.as_ref().as_os_str().is_empty() => p.as_ref().into(),
        _ => WinOsGetSystemDirectory()?.join("kernel32.dll"),
    };
    let file_info = WinOsGetFileVersionInfo(file_path)?;

    let v = mmbr_from_file_version(file_info)?;

    let mut info = create_OSVERSIONINFOEXW()?;
    info.wSuiteMask = WORD::try_from(VER_SUITE_WH_SERVER)?;
    info.wProductType = VER_NT_WORKSTATION;

    let mask = WinAPI_VerSetConditionMask(0, VER_SUITENAME, VER_EQUAL);
    let suite_mask = if WinAPI_VerifyVersionInfoW(&info, VER_SUITENAME, mask) != FALSE {
        VER_SUITE_WH_SERVER
    } else {
        0
    };

    let mask = WinAPI_VerSetConditionMask(0, VER_PRODUCT_TYPE, VER_EQUAL);
    let product_type = if WinAPI_VerifyVersionInfoW(&info, VER_PRODUCT_TYPE, mask) != FALSE {
        VER_NT_WORKSTATION
    } else {
        0
    };

    Ok(WinOsVersionInfo {
        os_name: winos_name(v.major, v.minor, v.build, product_type, suite_mask).into(),
        release: format!("{}.{}", v.major, v.minor).into(),
        version: format!("{}", v.build).into(),
    })
}

// mmbr_from_file_version
/// *Returns* version (as an [`MmbrVersion`]) copied from a view (aka slice) into the supplied `file_version_info`.
fn mmbr_from_file_version(
    file_version_info: WinApiFileVersionInfo,
) -> Result<MmbrVersion, WinOSError> {
    let info = WinOsFileVersionInfoQuery_root(&file_version_info)?;
    Ok(MmbrVersion {
        major: DWORD::from(HIWORD(info.dwProductVersionMS)),
        minor: DWORD::from(LOWORD(info.dwProductVersionMS)),
        build: DWORD::from(HIWORD(info.dwProductVersionLS)),
        release: DWORD::from(LOWORD(info.dwProductVersionLS)),
    })
}

// winos_name
/// *Returns* "friendly" WinOS name.
fn winos_name(
    major: DWORD,
    minor: DWORD,
    build: DWORD,
    product_type: BYTE,
    suite_mask: DWORD,
) -> String {
    // [NT Version Info (detailed)](https://en.wikipedia.org/wiki/Comparison_of_Microsoft_Windows_versions#Windows_NT) @@ <https://archive.is/FSkhj>
    let default_name = if product_type == VER_NT_WORKSTATION {
        format!("{} {}.{}", "Windows", major, minor)
    } else {
        format!("{} {}.{}", "Windows Server", major, minor)
    };

    let name = match major {
        5 => match minor {
            0 => "Windows 2000",
            1 => "Windows XP",
            2 if product_type == VER_NT_WORKSTATION => "Windows XP Professional x64 Edition",
            2 if suite_mask == VER_SUITE_WH_SERVER => "Windows Home Server",
            2 => "Windows Server 2003",
            _ => &default_name,
        },
        6 => match minor {
            0 if product_type == VER_NT_WORKSTATION => "Windows Vista",
            0 => "Windows Server 2008",
            1 if product_type != VER_NT_WORKSTATION => "Windows Server 2008 R2",
            1 => "Windows 7",
            2 if product_type != VER_NT_WORKSTATION => "Windows Server 2012",
            2 => "Windows 8",
            3 if product_type != VER_NT_WORKSTATION => "Windows Server 2012 R2",
            3 => "Windows 8.1",
            _ => &default_name,
        },
        10 => match minor {
            0 if product_type == VER_NT_WORKSTATION && (build >= 22000) => "Windows 11",
            0 if product_type != VER_NT_WORKSTATION && (14000..17000).contains(&build) => {
                "Windows Server 2016"
            }
            0 if product_type != VER_NT_WORKSTATION && (17000..19000).contains(&build) => {
                "Windows Server 2019"
            }
            0 if product_type != VER_NT_WORKSTATION && (build >= 20000) => "Windows Server 2022",
            _ => "Windows 10",
        },
        _ => &default_name,
    };

    name.to_string()
}

//===

fn determine_machine(system_info: &WinApiSystemInfo) -> OsString {
    let arch = system_info.wProcessorArchitecture();

    // ref: [SYSTEM_INFO structure](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/ns-sysinfoapi-system_info) @@ <https://archive.is/cqbrj>
    // ref: [LLVM Triples](https://llvm.org/doxygen/classllvm_1_1Triple.html) @@ <https://archive.is/MwVL8>
    // ref: [SuperH](https://en.wikipedia.org/wiki/SuperH) @@ <https://archive.is/ckr6a>
    // ref: [OldNewThing ~ SuperH](https://devblogs.microsoft.com/oldnewthing/20190805-00/?p=102749) @@ <https://archive.is/KWlyV>
    let arch_str = match arch {
        PROCESSOR_ARCHITECTURE_AMD64 => "x86_64",
        PROCESSOR_ARCHITECTURE_INTEL => match system_info.0.wProcessorLevel {
            4 => "i486",
            5 => "i586",
            6 => "i686",
            _ => "i386",
        },
        PROCESSOR_ARCHITECTURE_IA64 => "ia64",
        PROCESSOR_ARCHITECTURE_ARM => "arm", // `arm` may be under-specified compared to GNU implementations
        PROCESSOR_ARCHITECTURE_ARM64 => "aarch64", // alternatively, `arm64` may be more correct
        PROCESSOR_ARCHITECTURE_MIPS => "mips",
        PROCESSOR_ARCHITECTURE_PPC => "powerpc",
        PROCESSOR_ARCHITECTURE_ALPHA | PROCESSOR_ARCHITECTURE_ALPHA64 => "alpha",
        PROCESSOR_ARCHITECTURE_SHX => "superh", // a "SuperH" processor
        _ => "unknown",
    };

    OsString::from(arch_str)
}

fn determine_osname(version_info: &WinOsVersionInfo) -> OsString {
    let mut osname = OsString::from(crate::lib_impl::HOST_OS_NAME);
    osname.extend([
        OsString::from(" ("),
        version_info.os_name.clone(),
        OsString::from(")"),
    ]);
    osname
}

fn determine_sysname() -> OsString {
    // As of 2023-02, possible Windows kernels == [ "Windows_9x", "Windows_NT" ]
    // * "Windows_9x" hit end-of-service-life on 2006-07-11 (ref: [Windows_9x](https://en.wikipedia.org/wiki/Windows_9x) @@ <https://archive.is/wip/K6fhN>)
    OsString::from("Windows_NT") // compatible with `busybox` and MS (from std::env::var("OS"))
}

//=== Tests

#[test]
fn test_sysname() {
    let info = PlatformInfo::new().unwrap();
    let sysname = info.sysname().to_os_string();
    let expected = std::env::var_os("OS").unwrap_or_else(|| OsString::from("Windows_NT"));
    println!("sysname=[{}]'{:#?}'", sysname.len(), sysname);
    assert_eq!(sysname, expected);
}

#[test]
#[allow(non_snake_case)]
fn test_nodename_no_trailing_NUL() {
    let info = PlatformInfo::new().unwrap();
    let nodename = info.nodename().to_string_lossy();
    let trimmed = nodename.trim().trim_end_matches('\0');
    println!("nodename=[{}]'{}'", nodename.len(), nodename);
    assert_eq!(nodename, trimmed);
}

#[test]
fn test_machine() {
    let is_wow64 = KERNEL32_IsWow64Process(WinAPI_GetCurrentProcess()).unwrap_or_else(|err| {
        println!("ERR: IsWow64Process(): {:#?}", err);
        false
    });

    let target = if cfg!(target_arch = "x86_64") || (cfg!(target_arch = "x86") && is_wow64) {
        vec!["x86_64"]
    } else if cfg!(target_arch = "x86") {
        vec!["i386", "i486", "i586", "i686"]
    } else if cfg!(target_arch = "arm") {
        vec!["arm"]
    } else if cfg!(target_arch = "aarch64") {
        // NOTE: keeping both of these until the correct behavior is sorted out
        vec!["arm64", "aarch64"]
    } else if cfg!(target_arch = "powerpc") {
        vec!["powerpc"]
    } else if cfg!(target_arch = "mips") {
        vec!["mips"]
    } else {
        // NOTE: the other architecture are currently not valid targets for Rust (in fact, I am
        //       almost certain some of these are not even valid targets for the Windows build)
        vec!["unknown"]
    };
    println!("target={:#?}", target);

    let info = PlatformInfo::new().unwrap();
    let machine = info.machine().to_string_lossy();
    println!("machine=[{}]'{}'", machine.len(), machine);

    assert!(target.contains(&&machine[..]));
}

#[test]
fn test_osname() {
    let info = PlatformInfo::new().unwrap();
    let osname = info.osname().to_string_lossy();
    println!("osname=[{}]'{}'", osname.len(), osname);
    assert!(osname.starts_with(crate::lib_impl::HOST_OS_NAME));
}

#[test]
fn test_version_vs_version() {
    let version_via_dll = os_version_info_from_dll().unwrap();
    let version_via_file = version_info_from_file::<_, &str>(None).unwrap();
    assert!(version_via_file == version_info_from_file("").unwrap());

    println!("version (via dll) = '{:#?}'", version_via_dll);
    println!("version (via known file) = '{:#?}'", version_via_file);

    assert_eq!(version_via_dll.os_name, version_via_file.os_name);
    assert_eq!(version_via_dll.release, version_via_file.release);
    // the "version" portions may differ, but should have only slight variation
    // * assume that "version" is convertible to u32 + "version" from file is always earlier/smaller and may differ only below the thousands digit
    // * ref: [NT Version Info (detailed)](https://en.wikipedia.org/wiki/Comparison_of_Microsoft_Windows_versions#Windows_NT) @@ <https://archive.is/FSkhj>
    let version_via_dll_n = version_via_dll
        .version
        .to_string_lossy()
        .parse::<u32>()
        .unwrap();
    let version_via_file_n = version_via_file
        .version
        .to_string_lossy()
        .parse::<u32>()
        .unwrap();
    assert!(version_via_dll_n.checked_sub(version_via_file_n) < Some(1000));
}

#[test]
fn test_known_winos_names() {
    // ref: [NT Version Info (detailed)](https://en.wikipedia.org/wiki/Comparison_of_Microsoft_Windows_versions#Windows_NT) @@ <https://archive.is/FSkhj>
    assert_eq!(
        winos_name(3, 1, 528, VER_NT_WORKSTATION, VER_SUITE_PERSONAL),
        "Windows 3.1"
    );
    assert_eq!(
        winos_name(3, 5, 807, VER_NT_WORKSTATION, VER_SUITE_PERSONAL),
        "Windows 3.5"
    );
    assert_eq!(
        winos_name(3, 51, 1057, VER_NT_WORKSTATION, VER_SUITE_PERSONAL),
        "Windows 3.51"
    );
    assert_eq!(
        winos_name(4, 0, 1381, VER_NT_WORKSTATION, VER_SUITE_PERSONAL),
        "Windows 4.0"
    );
    assert_eq!(
        winos_name(5, 0, 2195, VER_NT_WORKSTATION, VER_SUITE_PERSONAL),
        "Windows 2000"
    );
    assert_eq!(
        winos_name(5, 1, 2600, VER_NT_WORKSTATION, VER_SUITE_PERSONAL),
        "Windows XP"
    );
    assert_eq!(
        winos_name(5, 2, 3790, VER_NT_WORKSTATION, VER_SUITE_PERSONAL),
        "Windows XP Professional x64 Edition"
    );
    assert_eq!(
        winos_name(5, 2, 3790, VER_NT_SERVER, VER_SUITE_WH_SERVER),
        "Windows Home Server"
    );
    assert_eq!(
        winos_name(5, 2, 3790, VER_NT_SERVER, VER_SUITE_SMALLBUSINESS),
        "Windows Server 2003"
    );
    assert_eq!(
        winos_name(5, 2, 3790, VER_NT_SERVER, VER_SUITE_SMALLBUSINESS),
        "Windows Server 2003"
    );
    assert_eq!(
        winos_name(6, 0, 6000, VER_NT_WORKSTATION, VER_SUITE_PERSONAL),
        "Windows Vista"
    );
    assert_eq!(
        winos_name(6, 0, 6001, VER_NT_SERVER, VER_SUITE_SMALLBUSINESS),
        "Windows Server 2008"
    );
    assert_eq!(
        winos_name(6, 1, 7600, VER_NT_WORKSTATION, VER_SUITE_PERSONAL),
        "Windows 7"
    );
    assert_eq!(
        winos_name(6, 1, 7600, VER_NT_SERVER, VER_SUITE_SMALLBUSINESS),
        "Windows Server 2008 R2"
    );
    assert_eq!(
        winos_name(6, 2, 9200, VER_NT_SERVER, VER_SUITE_SMALLBUSINESS),
        "Windows Server 2012"
    );
    assert_eq!(
        winos_name(6, 2, 9200, VER_NT_WORKSTATION, VER_SUITE_PERSONAL),
        "Windows 8"
    );
    assert_eq!(
        winos_name(6, 3, 9600, VER_NT_WORKSTATION, VER_SUITE_PERSONAL),
        "Windows 8.1"
    );
    assert_eq!(
        winos_name(6, 3, 9600, VER_NT_SERVER, VER_SUITE_SMALLBUSINESS),
        "Windows Server 2012 R2"
    );
    assert_eq!(
        winos_name(10, 0, 10240, VER_NT_WORKSTATION, VER_SUITE_PERSONAL),
        "Windows 10"
    );
    assert_eq!(
        winos_name(10, 0, 17134, VER_NT_WORKSTATION, VER_SUITE_PERSONAL),
        "Windows 10"
    );
    assert_eq!(
        winos_name(10, 0, 19141, VER_NT_WORKSTATION, VER_SUITE_PERSONAL),
        "Windows 10"
    );
    assert_eq!(
        winos_name(10, 0, 19145, VER_NT_WORKSTATION, VER_SUITE_PERSONAL),
        "Windows 10"
    );
    assert_eq!(
        winos_name(10, 0, 14393, VER_NT_SERVER, VER_SUITE_SMALLBUSINESS),
        "Windows Server 2016"
    );
    assert_eq!(
        winos_name(10, 0, 17763, VER_NT_SERVER, VER_SUITE_SMALLBUSINESS),
        "Windows Server 2019"
    );
    assert_eq!(
        winos_name(10, 0, 20348, VER_NT_SERVER, VER_SUITE_SMALLBUSINESS),
        "Windows Server 2022"
    );
    assert_eq!(
        winos_name(10, 0, 22000, VER_NT_WORKSTATION, VER_SUITE_PERSONAL),
        "Windows 11"
    );
    assert_eq!(
        winos_name(10, 0, 22621, VER_NT_WORKSTATION, VER_SUITE_PERSONAL),
        "Windows 11"
    );
    // test unmatched versions (triggers `_` matches and returns a `default_name`)
    assert_eq!(
        winos_name(5, 9, 3790, VER_NT_WORKSTATION, VER_SUITE_PERSONAL),
        "Windows 5.9"
    );
    assert_eq!(
        winos_name(5, 9, 3790, VER_NT_SERVER, VER_SUITE_SMALLBUSINESS),
        "Windows Server 5.9"
    );
    assert_eq!(
        winos_name(6, 9, 9600, VER_NT_WORKSTATION, VER_SUITE_PERSONAL),
        "Windows 6.9"
    );
    assert_eq!(
        winos_name(6, 9, 9600, VER_NT_SERVER, VER_SUITE_SMALLBUSINESS),
        "Windows Server 6.9"
    );
}

#[test]
fn structure_clone() {
    let info = PlatformInfo::new().unwrap();
    println!("{:?}", info);
    #[allow(clippy::redundant_clone)] // ignore `clippy::redundant_clone` warning for direct testing
    let info_copy = info.clone();
    assert_eq!(info_copy, info);

    let mmbr = MmbrVersion {
        major: 1,
        minor: 2,
        build: 3,
        release: 4,
    };
    println!("{:?}", mmbr);
    #[allow(clippy::redundant_clone)] // ignore `clippy::redundant_clone` warning for direct testing
    let mmbr_copy = mmbr.clone();
    assert_eq!(mmbr_copy, mmbr);

    let fvi = WinApiFileVersionInfo {
        data: vec![1, 2, 3, 4],
    };
    println!("{:?}", fvi);
    #[allow(clippy::redundant_clone)] // ignore `clippy::redundant_clone` warning for direct testing
    let fvi_copy = fvi.clone();
    assert_eq!(fvi_copy, fvi);
}
