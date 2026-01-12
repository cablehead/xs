use libc::{c_char, endpwent, getpwent, getpwnam, getpwuid, getuid, passwd, setpwent};
use std::ffi::{CStr, CString};

use crate::errors::{PwdError, Result};

/// The main struct for the library, a safe version
/// of the POSIX `struct passwd`
///
/// There are 2 ways to construct a `Passwd` instance (other
/// than assigning fields by hand). You can look up a user account
/// by username with `Passwd::from_name(String)`, or by uid with
/// `Passwd::from_uid(u32)`.
///
/// There is a shortcut function, `Passwd::current_user()`, which is just
/// short for `Passwd::from_uid(unsafe { libc::getuid() } as u32)`.
#[derive(Debug, Clone, PartialEq)]
pub struct Passwd {
    pub name: String,
    pub passwd: Option<String>,
    pub uid: u32,
    pub gid: u32,
    pub gecos: Option<String>,
    pub dir: String,
    pub shell: String,
}

// has to be public so it can be used, but we don't want people actually using it directly
#[derive(Debug, Clone, PartialEq)]
#[doc(hidden)]
pub struct PasswdIter {
    inited: bool,
}

impl PasswdIter {
    fn new() -> PasswdIter {
        PasswdIter { inited: false }
    }
}

impl Iterator for PasswdIter {
    type Item = Passwd;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.inited {
            unsafe {
                setpwent();
            };
            self.inited = true;
        }

        let next = unsafe { getpwent() };

        if next.is_null() {
            unsafe {
                endpwent();
            };
            return None;
        }

        if let Ok(passwd) = Passwd::from_unsafe(next) {
            Some(passwd)
        } else {
            None
        }
    }
}

fn cstr_to_string(cstr: *const c_char) -> Result<String> {
    let cstr = unsafe { CStr::from_ptr(cstr) };
    Ok(cstr
        .to_str()
        .map_err(|e| PwdError::StringConvError(format!("{:?}", e)))?
        .to_string())
}

impl Passwd {
    fn from_unsafe(pwd: *mut passwd) -> Result<Passwd> {
        if pwd.is_null() {
            return Err(PwdError::NullPtr);
        }
        // take ownership, since this shouldn't be null if we get here
        let pwd = unsafe { *pwd };
        let password = if pwd.pw_passwd.is_null() {
            None
        } else {
            Some(cstr_to_string(pwd.pw_passwd)?)
        };

        let gecos = if pwd.pw_gecos.is_null() {
            None
        } else {
            Some(cstr_to_string(pwd.pw_gecos)?)
        };

        Ok(Passwd {
            name: cstr_to_string(pwd.pw_name)?,
            passwd: password,
            uid: pwd.pw_uid as u32,
            gid: pwd.pw_gid as u32,
            gecos,
            dir: cstr_to_string(pwd.pw_dir)?,
            shell: cstr_to_string(pwd.pw_shell)?,
        })
    }

    /// Looks up the username and returns a Passwd with the user's values, if the user is found
    ///
    /// This is `Result<Option<>>` because the operation to convert a rust String to a cstring could fail
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate pwd;
    /// # use pwd::Result;
    /// use pwd::Passwd;
    ///
    /// # fn run() -> Result<()> {
    /// let pwd = Passwd::from_name("bob")?;
    ///
    /// if let Some(passwd) = pwd {
    ///     println!("uid is {}", passwd.uid);
    /// }
    /// #   Ok(())
    /// # }
    /// #
    /// # fn main() {
    /// #   if let Err(_) = run() {
    /// #     eprintln!("error running example");
    /// #   }
    /// # }
    /// ```
    pub fn from_name(name: &str) -> Result<Option<Passwd>> {
        let cname =
            CString::new(name).map_err(|e| PwdError::StringConvError(format!("{:?}", e)))?;
        let pwd = unsafe { getpwnam(cname.as_ptr()) };
        if pwd.is_null() {
            Ok(None)
        } else {
            Ok(Some(Passwd::from_unsafe(pwd)?))
        }
    }

    /// Looks up the uid and returns a Passwd with the user's values, if the user is found
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate pwd;
    /// # extern crate libc;
    /// # use pwd::Result;
    /// use libc::getuid;
    /// use pwd::Passwd;
    ///
    /// # fn run() -> Result<()> {
    /// let uid = unsafe { getuid() };
    /// let pwd = Passwd::from_uid(uid as u32);
    ///
    /// if let Some(passwd) = pwd {
    ///     println!("username is {}", passwd.name);
    /// }
    /// #   Ok(())
    /// # }
    /// #
    /// # fn main() {
    /// #   if let Err(_) = run() {
    /// #     eprintln!("error running example");
    /// #   }
    /// # }
    /// ```
    pub fn from_uid(uid: u32) -> Option<Passwd> {
        let pwd = unsafe { getpwuid(uid) };
        Passwd::from_unsafe(pwd).ok()
    }

    /// Shortcut for `Passwd::from_uid(libc::getuid() as u32)`, so see the docs for that constructor
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate pwd;
    /// # use pwd::Result;
    /// use pwd::Passwd;
    ///
    /// # fn run() -> Result<()> {
    /// let pwd = Passwd::current_user();
    ///
    /// if let Some(passwd) = pwd {
    ///     println!("username is {}", passwd.name);
    /// }
    /// #   Ok(())
    /// # }
    /// #
    /// # fn main() {
    /// #   if let Err(_) = run() {
    /// #     eprintln!("error running example");
    /// #   }
    /// # }
    /// ```
    pub fn current_user() -> Option<Passwd> {
        let uid = unsafe { getuid() };
        Passwd::from_uid(uid as u32)
    }

    /// Returns an iterator over all entries in the /etc/passwd file
    ///
    /// # Example
    ///
    /// ```rust
    /// # extern crate pwd;
    /// # use pwd::Result;
    /// use pwd::Passwd;
    ///
    /// # fn run() -> Result<()> {
    /// let passwds = Passwd::iter();
    ///
    /// for passwd in passwds {
    ///     println!("username is {}", passwd.name);
    /// }
    /// #   Ok(())
    /// # }
    /// #
    /// # fn main() {
    /// #   if let Err(_) = run() {
    /// #     eprintln!("error running example");
    /// #   }
    /// # }
    /// ```
    pub fn iter() -> PasswdIter {
        PasswdIter::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::u32;

    #[test]
    fn test_null_pwd_from_uid() {
        let should_be_none = Passwd::from_uid(u32::MAX);
        assert_eq!(should_be_none, None);
    }
}
