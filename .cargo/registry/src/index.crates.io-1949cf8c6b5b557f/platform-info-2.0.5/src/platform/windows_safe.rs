// spell-checker:ignore (abbrev) MSVC
// spell-checker:ignore (API) sysname osname nodename
// spell-checker:ignore (jargon) armv aarch
// spell-checker:ignore (rust) nonminimal repr stdcall uninit
// spell-checker:ignore (uutils) coreutils uutils
// spell-checker:ignore (vars) mmbr mmrb
// spell-checker:ignore (VSCode) endregion
// spell-checker:ignore (WinAPI) ctypes CWSTR DWORDLONG dwStrucVersion FARPROC FIXEDFILEINFO HIWORD HMODULE libloaderapi LOWORD LPCSTR LPCVOID LPCWSTR lpdw LPDWORD lplp LPOSVERSIONINFOEXW LPSYSTEM lptstr LPVOID LPWSTR minwindef ntdef ntstatus OSVERSIONINFOEXW processthreadsapi PUINT SMALLBUSINESS SUITENAME sysinfo sysinfoapi sysinfoapi TCHAR TCHARs ULONGLONG WCHAR WCHARs winapi winbase winver WSTR wstring
// spell-checker:ignore (WinOS) ntdll

#![warn(unused_results)] // enable warnings for unused results

use std::convert::TryFrom;
use std::io;
use std::mem::{self, MaybeUninit};
use std::ptr;

use winapi::shared::minwindef::*;
use winapi::shared::ntdef::NTSTATUS;
use winapi::shared::ntstatus::*;
use winapi::um::libloaderapi::*;
use winapi::um::processthreadsapi::GetCurrentProcess;
use winapi::um::sysinfoapi;
use winapi::um::sysinfoapi::*;
use winapi::um::winbase::*;
use winapi::um::winnt::*;
use winapi::um::winver::*;

use super::util::{to_c_string, to_c_wstring, CWSTR};
use super::{WinApiFileVersionInfo, WinApiSystemInfo};

use super::PathStr;
use super::WinOSError;

//===

// VS_FIXEDFILEINFO
/// WinAPI structure which contains version information for a file.
///
/// Implements [`VS_FIXEDFILEINFO`].
// ref: [`VS_FIXEDFILEINFO`](https://learn.microsoft.com/en-us/windows/win32/api/verrsrc/ns-verrsrc-vs_fixedfileinfo) @@ <https://archive.is/1sJgX>
#[allow(non_snake_case)]
#[allow(unused_variables)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct VS_FIXEDFILEINFO {
    pub dwSignature: DWORD,
    pub dwStrucVersion: DWORD,
    pub dwFileVersionMS: DWORD,
    pub dwFileVersionLS: DWORD,
    pub dwProductVersionMS: DWORD,
    pub dwProductVersionLS: DWORD,
    pub dwFileFlagsMask: DWORD,
    pub dwFileFlags: DWORD,
    pub dwFileOS: DWORD,
    pub dwFileType: DWORD,
    pub dwFileSubtype: DWORD,
    pub dwFileDateMS: DWORD,
    pub dwFileDateLS: DWORD,
}

//===

//#region unsafe code

impl WinApiSystemInfo {
    #[allow(non_snake_case)]
    /// Returns `wProcessorArchitecture` extracted from the [`SYSTEM_INFO`](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/ns-sysinfoapi-system_info) structure.
    /// <br> Refer to [`SYSTEM_INFO`](https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/ns-sysinfoapi-system_info) for more information.
    pub fn wProcessorArchitecture(&self) -> WORD {
        unsafe { self.0.u.s().wProcessorArchitecture }
    }
}

// create_OSVERSIONINFOEXW
/// *Returns* an owned, mutable [`OSVERSIONINFOEXW`] structure (fully initialized).
// ref: [`OSVERSIONINFOEXW`](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-osversioninfoexw) @@ <https://archive.is/n4hBb>
#[allow(non_snake_case)]
pub fn create_OSVERSIONINFOEXW(
) -> Result<OSVERSIONINFOEXW, crate::lib_impl::BoxedThreadSafeStdError> {
    let os_info_size = DWORD::try_from(mem::size_of::<OSVERSIONINFOEXW>())?;
    let mut os_info: OSVERSIONINFOEXW = unsafe { mem::zeroed() };
    os_info.dwOSVersionInfoSize = os_info_size;
    Ok(os_info)
}

// NOTE: WinAPI_... functions are thin-wrapper translations of the underlying WinOS API functions into safe functions

// WinAPI_FreeLibrary
/// Frees the loaded dynamic-link library (DLL) module, decrementing its reference count.
/// When the reference count reaches zero, the module is unloaded from the address space of the calling process and the
/// handle is no longer valid.
///
/// *Returns* BOOL ~ `FALSE` (aka zero) for fn *failure*; o/w non-`FALSE` (aka non-zero) for fn *success*.
///
/// Wraps WinOS [`Kernel32/FreeLibrary(...)`](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-freelibrary).
#[allow(non_snake_case)]
pub fn WinAPI_FreeLibrary(module: HMODULE /* from `hModule: HMODULE` */) -> BOOL {
    // FreeLibrary
    // pub unsafe fn FreeLibrary(hLibModule: HMODULE) -> BOOL
    // ref: <https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-freelibrary> @@ <https://archive.is/jWCsU>
    // * *returns* BOOL ~ `FALSE` (aka zero) for fn *failure*; o/w non-`FALSE` (aka non-zero) for fn *success*
    unsafe { FreeLibrary(module) }
}

// WinAPI_GetComputerNameExW
/// Retrieves a NetBIOS or DNS name associated with the local computer; stored into WCHAR vector (`buffer`).
///
/// * `name_type` ~ (in) specify requested type of computer name (as [COMPUTER_NAME_FORMAT](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/ne-sysinfoapi-computer_name_format))
/// * `buffer` ~ (out)
///   - for non-`FALSE` return, contains the requested computer name as WSTR (of length `size`)
///   - for `FALSE` return, unchanged
/// * `size` ~ (out)
///   - for non-`FALSE` return, contains the number of TCHARs (aka WCHARs) copied to the destination buffer, *not including* the terminating null character
///   - for `FALSE` return, contains the buffer size required for the result, *including* the terminating null character
///
/// *Returns* BOOL ~ `FALSE` (aka zero) for fn *failure*; o/w non-`FALSE` (aka non-zero) for fn *success*.
///
///### Notes
///
/// Vector capacity cannot be rigorously set because capacity is set as a minimum and may be "rounded up" by the
/// implementation. So, for the supplied `buffer`, `buffer.len()`, *not* `buffer.capacity()`, is used as the measure of
/// usable buffer size.
///
/// Supplying a zero-length `buffer` (or alternatively, `None`) as input will return a value specifying the actual
/// required buffer size for the system path.
///
/// Wraps WinOS [`Kernel32/GetComputerNameExW(...)`](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getcomputernameexw).
#[allow(non_snake_case)]
pub fn WinAPI_GetComputerNameExW<'a, T>(
    name_type: COMPUTER_NAME_FORMAT,
    buffer: T,        /* from `lpBuffer: LPWSTR` */
    size: &mut DWORD, /* from `nSize: LPDWORD` */
) -> BOOL
where
    T: Into<Option<&'a mut Vec<WCHAR>>>,
{
    // pub unsafe fn GetComputerNameExW(NameType: COMPUTER_NAME_FORMAT, lpBuffer: LPWSTR, nSize: LPDWORD) -> BOOL
    // ref: <https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getcomputernameexw> @@ <https://archive.is/Lgb7p>
    // * `name_type` ~ (in) [COMPUTER_NAME_FORMAT](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/ne-sysinfoapi-computer_name_format) @@ <https://archive.is/s18y0>
    // * `nSize` ~ (in) specifies the size of the destination buffer (*lpBuffer) in TCHARs (aka WCHARs)
    // * `nSize` ~ (out) on *fn failure* (aka `FALSE` return), receives the buffer size required for the result, *including* the terminating null character
    // * `nSize` ~ (out) on *fn success*, receives the number of TCHARs (aka WCHARs) copied to the destination buffer, *not including* the terminating null character
    // * *returns* BOOL ~ `FALSE` (aka zero) for fn *failure*; o/w non-`FALSE` (aka non-zero) for fn *success*
    let maybe_buffer = buffer.into();
    let (buffer_ptr, length) = match maybe_buffer {
        Some(buf) => (buf.as_mut_ptr(), DWORD::try_from(buf.len()).unwrap_or(0)),
        None => (ptr::null_mut(), 0),
    };
    *size = length;
    let result = unsafe { GetComputerNameExW(name_type, buffer_ptr, size) };
    assert!((result == FALSE) || (*size <= length)); // safety sanity check; panics on out-of-bounds memory writes (buffer overrun)
    result
}

// WinAPI_GetCurrentProcess
/// *Returns* a pseudo handle for the current process.
///
/// Wraps WinOS [`Kernel32/GetCurrentProcess()`](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getcurrentprocess).
#[allow(dead_code)] // * fn is used by test(s)
#[allow(non_snake_case)]
pub fn WinAPI_GetCurrentProcess() -> HANDLE {
    // GetCurrentProcess
    // pub unsafe fn GetCurrentProcess() -> HANDLE
    // ref: <https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getcurrentprocess> @@ <https://archive.is/AmB3f>
    unsafe { GetCurrentProcess() }
}

// WinAPI_GetFileVersionInfoSizeW
/// Determines whether the operating system can retrieve version information for a specified file (`file_path`).
/// If version information is available, GetFileVersionInfoSize returns the size, in bytes, of that information.
///
/// *Returns* DWORD ~ zero for fn *failure*; o/w size of the file version information, in *bytes*, for fn *success*.
///
/// Wraps WinOS [`Version/GetFileVersionInfoSizeW(...)`](https://learn.microsoft.com/en-us/windows/win32/api/winver/nf-winver-getfileversioninfosizew).
#[allow(non_snake_case)]
pub fn WinAPI_GetFileVersionInfoSizeW<P: AsRef<PathStr>>(
    file_path: P, /* used to generate `lptstrFilename: LPCWSTR` */ // lpdwHandle: *mut DWORD, /* ignored/not-needed */
) -> DWORD {
    // GetFileVersionInfoSizeW
    // pub unsafe fn GetFileVersionInfoSizeW(lptstrFilename: LPCWSTR, lpdwHandle: *mut DWORD) -> DWORD
    // ref: <https://learn.microsoft.com/en-us/windows/win32/api/winver/nf-winver-getfileversioninfosizew> @@ <https://archive.is/AdMHL>
    // * returns DWORD ~ on *failure*, 0
    // * returns DWORD ~ on *success*, size of the file version information, in *bytes*
    let file_path_cws: CWSTR = to_c_wstring(file_path.as_ref());
    unsafe {
        GetFileVersionInfoSizeW(file_path_cws.as_ptr(), ptr::null_mut() /* ignored */)
    }
}

// WinAPI_GetFileVersionInfoW
/// Retrieves version information for the specified file (`file_path`); stored into BYTE vector (`data`).
///
/// *Returns* BOOL ~ `FALSE` (aka zero) for fn *failure*; o/w non-`FALSE` (aka non-zero) for fn *success*.
///
/// Wraps WinOS [`Version/GetFileVersionInfoW(...)`](https://learn.microsoft.com/en-us/windows/win32/api/winver/nf-winver-getfileversioninfow).
#[allow(non_snake_case)]
pub fn WinAPI_GetFileVersionInfoW<P: AsRef<PathStr>>(
    file_path: P, /* used to generate `lptstrFilename: LPCWSTR` */
    // dwHandle: DWORD, /* ignored/not-needed */
    // dwLen: DWORD,  /* not-needed */
    data: &mut Vec<BYTE>, /* from `lpData: *mut winapi::ctypes::c_void` */
) -> BOOL {
    // GetFileVersionInfoW
    // pub unsafe fn GetFileVersionInfoW(lptstrFilename: LPCWSTR, dwHandle: DWORD, dwLen: DWORD, lpData: *mut c_void) -> BOOL
    // ref: <https://learn.microsoft.com/en-us/windows/win32/api/winver/nf-winver-getfileversioninfow> @@ <https://archive.is/4rx6D>
    // * handle/dwHandle == *ignored*
    // * length/dwLen == maximum size (in bytes) of buffer at data_ptr/lpData
    // * *returns* BOOL ~ `FALSE` (aka zero) for fn *failure*, o/w non-`FALSE` (aka non-zero) for fn *success*
    let file_path_cws: CWSTR = to_c_wstring(file_path.as_ref());
    unsafe {
        GetFileVersionInfoW(
            file_path_cws.as_ptr(),
            0, /* ignored */
            DWORD::try_from(data.capacity()).unwrap(),
            data.as_mut_ptr() as *mut _,
        )
    }
}

// WinAPI_GetNativeSystemInfo
/// *Returns* information (as `SYSTEM_INFO`) about the current system to an application running under WOW64.
///
/// If the function is called from a 64-bit application, it is equivalent to the GetSystemInfo function.
/// If the function is called from an x86 or x64 application running on a 64-bit system that does not have an Intel64 or
//  x64 processor (such as ARM64), it will return information as if the system is x86 only if x86 emulation is supported
/// (or x64 if x64 emulation is also supported).
///
/// Wraps WinOS [`Kernel32/GetNativeSystemInfo(...)`](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getnativesysteminfo).
#[allow(non_snake_case)]
pub fn WinAPI_GetNativeSystemInfo() -> SYSTEM_INFO {
    // GetNativeSystemInfo
    // pub unsafe fn GetNativeSystemInfo(lpSystemInfo: LPSYSTEM_INFO)
    // ref: <https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getnativesysteminfo> @@ <https://archive.is/UV2S2>
    let mut sysinfo = MaybeUninit::<SYSTEM_INFO>::uninit();
    unsafe {
        GetNativeSystemInfo(sysinfo.as_mut_ptr());
        // SAFETY: `GetNativeSystemInfo()` always succeeds => `sysinfo` was initialized
        sysinfo.assume_init()
    }
}

// WinAPI_GetProcAddress
/// *Returns* the address of an exported function/procedure or variable (`symbol_name`) from the specified library (`module`).
///
/// Wraps WinOS [`Kernel32/GetProcAddress(...)`](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getprocaddress).
#[allow(non_snake_case)]
pub fn WinAPI_GetProcAddress<P: AsRef<PathStr>>(
    module: HMODULE, /* from `hModule: HMODULE` */
    symbol_name: P,  /* used to generate `lpProcName: LPCSTR` */
) -> FARPROC {
    // GetProcAddress
    // pub unsafe fn GetProcAddress(hModule: HMODULE, lpProcName: LPCSTR) -> FARPROC
    // ref: <https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getprocaddress> @@ <https://archive.is/ZPVMr>
    let symbol_name_cs = to_c_string(symbol_name.as_ref());
    unsafe { GetProcAddress(module, symbol_name_cs.as_ptr()) }
}

// WinAPI_GetSystemDirectoryW
/// Retrieves the path of the system directory; stored into a WCHAR vector (`buffer`).
///
/// * `buffer`
///   - for non-zero return (*success*) with adequate buffer size, `buffer` will contain the requested WinOS System Directory path as a WSTR
///   - for zero (*failure*) or non-zero (*success*) return with inadequate buffer size, `buffer` will be unchanged
///
/// *Returns* UINT
///   - zero for fn *failure*
///   - fn *success* with adequate buffer size, contains the number of WCHARs (aka TCHARs) copied to the destination buffer, *not including* the terminating null character
///   - fn *success* with inadequate buffer size, contains the buffer size required for the requested path, *including* the terminating null character
///
///### Notes
///
/// Vector capacity cannot be rigorously set because capacity is set as a minimum and may be "rounded up" by the
/// implementation. So, for the supplied `buffer`, `buffer.len()`, *not* `buffer.capacity()`, is used as the measure of
/// usable buffer size.
///
/// Supplying a zero-length `buffer` (or alternatively, `None`) as input will return a value specifying the actual
/// required buffer size for the system path.
///
/// Wraps WinOS [`Kernel32/GetSystemDirectoryW(...)`](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getsystemdirectoryw).
#[allow(non_snake_case)]
pub fn WinAPI_GetSystemDirectoryW<'a, T>(
    buffer: T, /* from `lpBuffer: LPWSTR` */ //  uSize: UINT, /* not needed */
) -> UINT
where
    T: Into<Option<&'a mut Vec<WCHAR>>>,
{
    // GetSystemDirectoryW
    // pub unsafe fn GetSystemDirectoryW(lpBuffer: LPWSTR, uSize: UINT) -> UINT
    // ref: <https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getsystemdirectoryw> @@ <https://archive.is/OTVW8>
    // * `uSize` ~ (in) specifies the maximum size of the destination buffer (*lpBuffer) in TCHARs (aka WCHARs)
    // * returns UINT ~ on fn *failure*, 0
    // * returns UINT ~ on fn *success* and uSize <= length(System Directory string), size of required destination buffer (in TCHARs [aka WCHARs]), *including* the terminating null character
    // * returns UINT ~ on fn *success* and uSize > length(System Directory string), the number of TCHARs (aka WCHARs) copied to the destination buffer, *not including* the terminating null character
    let (buffer_ptr, length) = match buffer.into() {
        Some(buf) => (buf.as_mut_ptr(), UINT::try_from(buf.len()).unwrap_or(0)),
        None => (ptr::null_mut(), 0),
    };
    unsafe { GetSystemDirectoryW(buffer_ptr, length) }
}

// WinAPI_LoadLibrary
/// *Returns* a module handle for the specified module (`module_name`), loading the library, if needed, and increasing
/// the per-process reference count.
///
/// * `module_name` ~ requested library/module <br>
///   Note: full path specification of `module_name` has increased safety. If only base name and extension is specified,
///   modules with colliding names may cause the return of a random module handle.
///
/// This function is multithread-safe.
/// But this function cannot be called in certain contexts, such as recursively, by any code path called by a `DllMain`
/// function, or within a "module initializer" (such as a C++ constructor for a global variable).
///
/// Wraps WinOS [`Kernel32/LoadLibraryW(...)`](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getmodulehandlew).
// ref: <https://github.com/rust-lang/rust/issues/78444>
// ref: <https://stackoverflow.com/questions/7364846/loading-dll-via-getmodulehandle-loadlibrary-and-using-freelibrary>
#[allow(non_snake_case)]
pub fn WinAPI_LoadLibrary<P: AsRef<PathStr>>(
    module_name: P, /* used to generate `lpFileName: LPCWSTR` */
) -> HMODULE {
    // LoadLibraryW
    // pub unsafe fn LoadLibraryW(lpFileName: LPCWSTR) -> HMODULE
    // ref: <https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibraryw> @@ <https://archive.is/N3Fxf>
    let module_name_cws: CWSTR = to_c_wstring(module_name.as_ref());
    unsafe { LoadLibraryW(module_name_cws.as_ptr()) }
}

// WinAPI_VerifyVersionInfoW
/// Compares a set of operating system version requirements (`version_info`, `type_mask`, and `condition_mask`) to the
/// corresponding values for the currently running version of the system.
///
/// *Returns* BOOL ~ `FALSE` (aka zero) if resource is non-existent or requirements are not met; o/w non-`FALSE` (aka non-zero)
///
/// Wraps WinOS [`Kernel32/VerifyVersionInfoW(...)`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-verifyversioninfow).
#[allow(non_snake_case)]
pub fn WinAPI_VerifyVersionInfoW(
    version_info: &OSVERSIONINFOEXW, /* from `lpVersionInformation: LPOSVERSIONINFOEXW` */
    type_mask: DWORD,                /* from `dwTypeMask: DWORD` */
    condition_mask: DWORDLONG,       /* from `dwlConditionMask: DWORDLONG` */
) -> BOOL {
    // VerifyVersionInfoW
    // pub unsafe fn VerifyVersionInfoW(lpVersionInformation: LPOSVERSIONINFOEXW, dwTypeMask: DWORD, dwlConditionMask: DWORDLONG) -> BOOL
    // ref: <https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-verifyversioninfow> @@ <https://archive.is/1h5FF>
    // version_info/lpVersionInformation ~ pointer to (const) version info requirements for comparison
    // type_mask ~ mask indicating members of version_info to be tested
    // condition_mask ~ type of comparison for each version_info member to be compared
    // * returns BOOL ~ `FALSE` (aka zero) for non-existent resource or invalid requirements; o/w non-`FALSE` (aka non-zero)
    let version_info_ptr: *const OSVERSIONINFOEXW = version_info;
    unsafe {
        VerifyVersionInfoW(
            version_info_ptr as *mut OSVERSIONINFOEXW, // version_info_ptr *is* `*const OSVERSIONINFOEXW` but misdefined by function declaration
            type_mask,
            condition_mask,
        )
    }
}

// WinAPI_VerQueryValueW
/// Retrieves specified version information (`query`) as a view (`info_view`) into the specified version-information
/// resource (`version_info`).
///
/// *Returns* BOOL ~ `FALSE` (aka zero) for fn *failure*; o/w non-zero for fn *success*.
///
/// Wraps WinOS [`Version/VerQueryValueW(...)`](https://learn.microsoft.com/en-us/windows/win32/api/winver/nf-winver-verqueryvaluew).
#[allow(non_snake_case)]
pub fn WinAPI_VerQueryValueW<'a, S: AsRef<str>>(
    version_info: &'a [BYTE],       /* from `pBlock: LPCVOID` */
    query: S,                       /* from `lpSubBlock: LPCWSTR` */
    info_view: &'a mut LPVOID,      /* from `lplpBuffer: &mut LPVOID` */
    info_view_length: &'a mut UINT, /* from `puLen: PUINT` */
) -> BOOL {
    // VerQueryValueW
    // pub unsafe fn VerQueryValueW(pBlock: LPCVOID, lpSubBlock: LPCWSTR, lplpBuffer: &mut LPVOID, puLen: PUINT) -> BOOL
    // ref: <https://learn.microsoft.com/en-us/windows/win32/api/winver/nf-winver-verqueryvaluew> @@ <https://archive.is/VqvGQ>
    // version_info_ptr/pBlock ~ pointer to (const) version info
    // info_view/lplpBuffer ~ pointer into version info supplied by version_info_ptr (no new allocations)
    // info_view_length/puLen ~ pointer to size (in characters [TCHARs/WCHARs] for "version info values", in bytes for translation array or root block)
    // * returns BOOL ~ `FALSE` (aka zero) for invalid/non-existent resource; o/w non-`FALSE` (aka non-zero)
    let version_info_ptr = version_info.as_ptr() as LPCVOID;
    unsafe {
        VerQueryValueW(
            version_info_ptr,
            to_c_wstring(query.as_ref()).as_ptr(),
            info_view,
            info_view_length,
        )
    }
}

// WinAPI_VerSetConditionMask
/// Sets the bits of a 64-bit value to indicate the comparison operator to use for a specified operating system version
/// attribute (ie, specific member of version_info [[`OSVERSIONINFOEXW`]]). This function is used, possibly repeatedly, to build the
/// `condition_mask` parameter of the [WinAPI_VerifyVersionInfoW] function.
///
/// * condition_mask ~ baseline value for type of comparisons for each member of version info
/// * type_mask ~ mask indicating the member of version info whose comparison operator is being set
/// * condition ~ comparison type
///
/// *Returns* ULONGLONG ~ updated condition_mask
///
/// Wraps WinOS [`Kernel32/VerSetConditionMask(...)`](https://learn.microsoft.com/en-us/windows/win32/api/winnt/nf-winnt-versetconditionmask).
#[allow(non_snake_case)]
pub fn WinAPI_VerSetConditionMask(
    condition_mask: ULONGLONG,
    type_mask: DWORD,
    condition: BYTE,
) -> ULONGLONG {
    // VerSetConditionMask
    // pub unsafe fn VerSetConditionMask(ConditionMask: ULONGLONG, TypeMask: DWORD, Condition: BYTE) -> ULONGLONG
    // ref: <https://learn.microsoft.com/en-us/windows/win32/api/winnt/nf-winnt-versetconditionmask> @@ <https://archive.is/hJtIB>
    // condition_mask ~ baseline value for type of comparisons for each member of version info (ie, `OSVERSIONINFOEXW`)
    // type_mask ~ mask indicating the member of version info whose comparison operator is being set
    // condition ~ comparison type
    // * returns ULONGLONG ~ updated condition_mask
    unsafe { sysinfoapi::VerSetConditionMask(condition_mask, type_mask, condition) }
}

// WinOsFileVersionInfoQuery_root
/// *Returns* the "root" version information *as view of the internal [`VS_FIXEDFILEINFO`] structure* within the
/// specified version-information resource (`version_info`).
///
/// Uses WinOS [`Version/WinAPI_VerQueryValueW(...)`](https://learn.microsoft.com/en-us/windows/win32/api/winver/nf-winver-verqueryvaluew).
#[allow(non_snake_case)]
pub fn WinOsFileVersionInfoQuery_root(
    version_info: &WinApiFileVersionInfo,
) -> Result<&VS_FIXEDFILEINFO, WinOSError> {
    // NOTE: this function could be expanded to cover root, translation, and information queries by using an enum for a return value

    // VerQueryValueW
    // pub unsafe fn VerQueryValueW(pBlock: LPCVOID, lpSubBlock: LPCWSTR, lplpBuffer: &mut LPVOID, puLen: PUINT) -> BOOL
    // ref: <https://learn.microsoft.com/en-us/windows/win32/api/winver/nf-winver-verqueryvaluew> @@ <https://archive.is/VqvGQ>
    // version_info_ptr/pBlock ~ pointer to (const) version info
    // info_view/lplpBuffer ~ pointer into version info supplied by version_info_ptr (no new allocations)
    // info_view_length/puLen ~ pointer to size (in characters [TCHARs/WCHARs] for "version info values", in bytes for translation array or root block)
    // * returns BOOL ~ `FALSE` (aka zero) for invalid/non-existent resource; o/w non-`FALSE` (aka non-zero) for *fn success*

    let version_info_data = &version_info.data;

    let mut data_view = ptr::null_mut(); // view into the `version_info_data` block
    let mut data_view_size = 0;

    let query = "\\"; // "root" query ~ requests the VS_FIXEDFILEINFO structure from within the supplied `version_info`
    let fixed_file_info_size = UINT::try_from(mem::size_of::<VS_FIXEDFILEINFO>())?; // expected returned data_view_size
    if WinAPI_VerQueryValueW(
        version_info_data,
        query,
        &mut data_view,
        &mut data_view_size,
    ) == 0
        || (data_view_size != fixed_file_info_size)
    {
        return Err(Box::new(io::Error::last_os_error()));
    }

    assert!(version_info_data.len() >= usize::try_from(data_view_size)?);
    assert!(data_view_size == fixed_file_info_size);
    assert!(!data_view.is_null());
    // * lifetime of block/info is the same as input argument version_info
    Ok(unsafe { &*(data_view as *const VS_FIXEDFILEINFO) })
}

// KERNEL32_IsWow64Process
/// *Returns* an assertion of whether the specified `process` is running under WOW64 on an Intel64 or x64 processor.
///
/// Wraps [`Kernel32/IsWow64Process`](https://learn.microsoft.com/en-us/windows/win32/api/wow64apiset/nf-wow64apiset-iswow64process).
#[allow(dead_code)] // * fn is used by test(s)
#[allow(non_snake_case)]
pub fn KERNEL32_IsWow64Process(process: HANDLE) -> Result<bool, WinOSError> {
    // kernel32.dll/IsWow64Process
    // extern "stdcall" fn(HANDLE, *mut BOOL) -> BOOL
    // ref: <https://learn.microsoft.com/en-us/windows/win32/api/wow64apiset/nf-wow64apiset-iswow64process> @@ <https://archive.is/K00m6>
    let module_file = "kernel32.dll";
    let symbol_name = "IsWow64Process";
    let module_path = super::WinOsGetSystemDirectory()?.join(module_file);
    // let func = super::WinOsGetModuleProcAddress(module_path, procedure); // loads module "permanently" (for the life of current process)
    let module = WinAPI_LoadLibrary(module_path);
    let func = WinAPI_GetProcAddress(module, symbol_name);
    if func.is_null() {
        return Err(Box::from(format!(
            "Unable to find DLL procedure '{}' within '{}'",
            symbol_name, module_file
        )));
    }

    let func: extern "stdcall" fn(HANDLE, *mut BOOL) -> BOOL =
        unsafe { mem::transmute(func as *const ()) };

    let mut is_wow64: BOOL = FALSE;
    let result: BOOL = func(process, &mut is_wow64);

    let _ = WinAPI_FreeLibrary(module); // FreeLibrary() failure/success can be safely ignored

    Ok((result != FALSE/* func() succeeded` */) && (is_wow64 != FALSE))
}

// NTDLL_RtlGetVersion
/// *Returns* version information about the currently running operating system.
///
/// Wraps [`NTDLL/RtlGetVersion`](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-rtlgetversion).
#[allow(non_snake_case)]
pub fn NTDLL_RtlGetVersion() -> Result<OSVERSIONINFOEXW, WinOSError> {
    // ntdll.dll/RtlGetVersion
    // extern "stdcall" fn(*mut RTL_OSVERSIONINFOEXW) -> NTSTATUS
    // ref: <https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-rtlgetversion> @@ <https://archive.is/H1Ls2>
    // ref: [`RTL_OSVERSIONINFOEXW`](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/ns-wdm-_osversioninfoexw) @@ <https://archive.is/CtlZS>
    // note: OSVERSIONINFOEXW == RTL_OSVERSIONINFOEXW ; ref: <https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-osversioninfoexw> @@ <https://archive.is/n4hBb>
    let module_file = "ntdll.dll";
    let symbol_name = "RtlGetVersion";
    let module_path = super::WinOsGetSystemDirectory()?.join(module_file);
    // let func = super::WinOsGetModuleProcAddress(module_path, procedure); // loads module "permanently" (for the life of current process)
    let module = WinAPI_LoadLibrary(module_path);
    let func = WinAPI_GetProcAddress(module, symbol_name);
    if func.is_null() {
        return Err(Box::from(format!(
            "Unable to find DLL procedure '{}' within '{}'",
            symbol_name, module_file
        )));
    }
    let func: extern "stdcall" fn(*mut RTL_OSVERSIONINFOEXW) -> NTSTATUS =
        unsafe { mem::transmute(func as *const ()) };

    let mut os_version_info = match create_OSVERSIONINFOEXW() {
        Ok(value) => value,
        Err(_) => return Err(Box::from("Unable to create OSVERSIONINFOEXW".to_string())),
    };

    let result: NTSTATUS = func(&mut os_version_info);

    let _ = WinAPI_FreeLibrary(module); // FreeLibrary() failure/success can be safely ignored

    if result == STATUS_SUCCESS {
        Ok(os_version_info)
    } else {
        Err(Box::from(format!(
            "RtlGetVersion() failed (result/status: {})",
            result
        )))
    }
}

//#endregion (unsafe code)

//=== Tests

#[test]
fn structure_clone() {
    let ffi = VS_FIXEDFILEINFO {
        dwSignature: 0,
        dwStrucVersion: 0,
        dwFileVersionMS: 0,
        dwFileVersionLS: 0,
        dwProductVersionMS: 0,
        dwProductVersionLS: 0,
        dwFileFlagsMask: 0,
        dwFileFlags: 0,
        dwFileOS: 0,
        dwFileType: 0,
        dwFileSubtype: 0,
        dwFileDateMS: 0,
        dwFileDateLS: 0,
    };
    println!("{:?}", ffi);
    #[allow(clippy::clone_on_copy)] // ignore `clippy::clone_on_copy` warning for direct testing
    let ffi_clone = ffi.clone();
    assert_eq!(ffi_clone, ffi);
}
