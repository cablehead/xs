//! Cross-platform pseudo-terminal child process.
//!
//! Unix: openpty + fork. Windows: ConPTY.

/// A child process running inside a pseudo-terminal.
///
/// `reader` carries terminal output (VT sequences).
/// `writer` accepts input (keystrokes).
/// `handle` controls the PTY lifecycle (resize, kill).
pub struct PtyChild {
    pub reader: std::fs::File,
    pub writer: std::fs::File,
    pub handle: PtyHandle,
}

impl PtyChild {
    /// Spawn `cmd` inside a new PTY with the given dimensions.
    pub fn spawn(cmd: &str, cols: u16, rows: u16) -> Result<Self, String> {
        platform::spawn(cmd, cols, rows)
    }
}

impl PtyHandle {
    /// Resize the PTY.
    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), String> {
        platform::resize(self, cols, rows)
    }

    /// Kill the child process.
    pub fn kill(&mut self) {
        platform::kill(self);
    }
}

// ---- Unix ----

#[cfg(unix)]
pub struct PtyHandle {
    /// Raw fd for ioctl. Valid as long as reader/writer are alive.
    primary_fd: std::os::unix::io::RawFd,
    pid: nix::unistd::Pid,
}

#[cfg(unix)]
mod platform {
    use super::*;
    use nix::libc;
    use nix::pty::openpty;
    use nix::sys::signal::Signal;
    use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
    use nix::unistd::{close, dup2, execvp, fork, setsid, ForkResult};
    use std::ffi::CString;
    use std::os::unix::io::{FromRawFd, IntoRawFd};

    pub fn spawn(cmd: &str, cols: u16, rows: u16) -> Result<PtyChild, String> {
        let ws = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        let pty = openpty(Some(&ws), None).map_err(|e| format!("openpty: {e}"))?;
        let primary_fd = pty.master.into_raw_fd();
        let secondary_fd = pty.slave.into_raw_fd();

        match unsafe { fork() } {
            Ok(ForkResult::Child) => {
                let _ = close(primary_fd);
                let _ = setsid();

                // Set controlling terminal
                unsafe {
                    libc::ioctl(secondary_fd, libc::TIOCSCTTY as _, 0);
                }

                let _ = dup2(secondary_fd, 0);
                let _ = dup2(secondary_fd, 1);
                let _ = dup2(secondary_fd, 2);
                if secondary_fd > 2 {
                    let _ = close(secondary_fd);
                }

                let c_cmd = CString::new(cmd).unwrap_or_else(|_| CString::new("sh").unwrap());
                let args = [c_cmd.clone()];
                let _ = execvp(&c_cmd, &args);
                std::process::exit(1);
            }
            Ok(ForkResult::Parent { child }) => {
                let _ = close(secondary_fd);
                let primary_file = unsafe { std::fs::File::from_raw_fd(primary_fd) };
                let reader = primary_file
                    .try_clone()
                    .map_err(|e| format!("clone fd: {e}"))?;
                Ok(PtyChild {
                    reader,
                    writer: primary_file,
                    handle: PtyHandle {
                        primary_fd,
                        pid: child,
                    },
                })
            }
            Err(e) => Err(format!("fork: {e}")),
        }
    }

    pub fn resize(handle: &PtyHandle, cols: u16, rows: u16) -> Result<(), String> {
        let ws = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        unsafe {
            libc::ioctl(handle.primary_fd, libc::TIOCSWINSZ as _, &ws);
        }
        let _ = nix::sys::signal::kill(handle.pid, Signal::SIGWINCH);
        Ok(())
    }

    pub fn kill(handle: &mut PtyHandle) {
        let _ = nix::sys::signal::kill(handle.pid, Signal::SIGTERM);
        std::thread::sleep(std::time::Duration::from_millis(50));
        if let Ok(WaitStatus::StillAlive) = waitpid(handle.pid, Some(WaitPidFlag::WNOHANG)) {
            let _ = nix::sys::signal::kill(handle.pid, Signal::SIGKILL);
            let _ = waitpid(handle.pid, None);
        }
    }
}

// ---- Windows ----

#[cfg(windows)]
pub struct PtyHandle {
    hpcon: windows::Win32::System::Console::HPCON,
    process_handle: windows::Win32::Foundation::HANDLE,
    thread_handle: windows::Win32::Foundation::HANDLE,
    closed: bool,
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::ffi::c_void;
    use std::mem;
    use std::os::windows::io::{FromRawHandle, RawHandle};
    use windows::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
    use windows::Win32::System::Console::{
        ClosePseudoConsole, CreatePseudoConsole, ResizePseudoConsole, COORD, HPCON,
    };
    use windows::Win32::System::Pipes::CreatePipe;
    use windows::Win32::System::Threading::{
        CreateProcessW, DeleteProcThreadAttributeList, InitializeProcThreadAttributeList,
        TerminateProcess, UpdateProcThreadAttribute, EXTENDED_STARTUPINFO_PRESENT,
        LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
        STARTUPINFOEXW, STARTUPINFOW,
    };

    pub fn spawn(cmd: &str, cols: u16, rows: u16) -> Result<PtyChild, String> {
        unsafe { spawn_inner(cmd, cols, rows) }
    }

    unsafe fn spawn_inner(cmd: &str, cols: u16, rows: u16) -> Result<PtyChild, String> {
        // Create two pipe pairs: input and output
        let mut input_read = INVALID_HANDLE_VALUE;
        let mut input_write = INVALID_HANDLE_VALUE;
        let mut output_read = INVALID_HANDLE_VALUE;
        let mut output_write = INVALID_HANDLE_VALUE;

        CreatePipe(&mut input_read, &mut input_write, None, 0)
            .map_err(|e| format!("CreatePipe (input): {e}"))?;
        CreatePipe(&mut output_read, &mut output_write, None, 0)
            .map_err(|e| format!("CreatePipe (output): {e}"))?;

        let size = COORD {
            X: cols as i16,
            Y: rows as i16,
        };

        // ConPTY reads from input_read, writes to output_write
        let hpcon = CreatePseudoConsole(size, input_read, output_write, 0)
            .map_err(|e| format!("CreatePseudoConsole: {e}"))?;

        // Close the pipe ends now owned by the ConPTY
        let _ = CloseHandle(input_read);
        let _ = CloseHandle(output_write);

        // Set up process thread attribute list
        let mut attr_size: usize = 0;
        let _ = InitializeProcThreadAttributeList(None, 1, None, &mut attr_size);

        let mut attr_buf = vec![0u8; attr_size];
        let attr_list = LPPROC_THREAD_ATTRIBUTE_LIST(attr_buf.as_mut_ptr() as *mut _);

        InitializeProcThreadAttributeList(Some(attr_list), 1, None, &mut attr_size)
            .map_err(|e| format!("InitializeProcThreadAttributeList: {e}"))?;

        UpdateProcThreadAttribute(
            attr_list,
            0,
            PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
            Some(&hpcon as *const HPCON as *const c_void),
            mem::size_of::<HPCON>(),
            None,
            None,
        )
        .map_err(|e| format!("UpdateProcThreadAttribute: {e}"))?;

        let startup_info = STARTUPINFOEXW {
            StartupInfo: STARTUPINFOW {
                cb: mem::size_of::<STARTUPINFOEXW>() as u32,
                ..Default::default()
            },
            lpAttributeList: attr_list,
        };

        let mut cmd_wide: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0)).collect();
        let mut proc_info = PROCESS_INFORMATION::default();

        CreateProcessW(
            None,
            windows::core::PWSTR(cmd_wide.as_mut_ptr()),
            None,
            None,
            false,
            EXTENDED_STARTUPINFO_PRESENT,
            None,
            None,
            &startup_info.StartupInfo,
            &mut proc_info,
        )
        .map_err(|e| format!("CreateProcessW: {e}"))?;

        DeleteProcThreadAttributeList(attr_list);

        // Wrap pipe handles as std::fs::File
        let reader = std::fs::File::from(std::os::windows::io::OwnedHandle::from_raw_handle(
            output_read.0 as RawHandle,
        ));
        let writer = std::fs::File::from(std::os::windows::io::OwnedHandle::from_raw_handle(
            input_write.0 as RawHandle,
        ));

        Ok(PtyChild {
            reader,
            writer,
            handle: PtyHandle {
                hpcon,
                process_handle: proc_info.hProcess,
                thread_handle: proc_info.hThread,
                closed: false,
            },
        })
    }

    pub fn resize(handle: &PtyHandle, cols: u16, rows: u16) -> Result<(), String> {
        if handle.closed {
            return Err("PTY already closed".into());
        }
        let size = COORD {
            X: cols as i16,
            Y: rows as i16,
        };
        unsafe { ResizePseudoConsole(handle.hpcon, size) }
            .map_err(|e| format!("ResizePseudoConsole: {e}"))
    }

    pub fn kill(handle: &mut PtyHandle) {
        if handle.closed {
            return;
        }
        unsafe {
            let _ = TerminateProcess(handle.process_handle, 1);
            // Close the ConPTY so the output pipe gets EOF
            ClosePseudoConsole(handle.hpcon);
        }
        handle.closed = true;
    }
}

#[cfg(windows)]
impl Drop for PtyHandle {
    fn drop(&mut self) {
        unsafe {
            if !self.closed {
                windows::Win32::System::Console::ClosePseudoConsole(self.hpcon);
            }
            let _ = windows::Win32::Foundation::CloseHandle(self.process_handle);
            let _ = windows::Win32::Foundation::CloseHandle(self.thread_handle);
        }
    }
}
