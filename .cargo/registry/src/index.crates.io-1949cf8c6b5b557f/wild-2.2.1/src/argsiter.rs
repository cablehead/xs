use crate::globiter::GlobArgs;
use std::ffi::OsString;
use std::fmt;

/// Windows replacement for `std::env::ArgsOs`
#[cfg_attr(test, allow(dead_code))]
pub struct ArgsOs {
    pub(crate) args: GlobArgs<'static>,
    pub(crate) current_arg_globs: Option<glob::Paths>,
}

impl ArgsOs {
    /// Expects result of `GetCommandLineW`
    #[inline]
    pub(crate) fn from_raw_command_line(cmd: &'static [u16]) -> Self {
        Self {
            args: GlobArgs::new(cmd),
            current_arg_globs: None,
        }
    }
}

/// Windows replacement for `std::env::Args`
pub struct Args {
    pub(crate) iter: ArgsOs,
}

fn first_non_error<T,E,I>(iter: &mut I) -> Option<T> where I: Iterator<Item=Result<T,E>> {
    loop {
        match iter.next() {
            Some(Ok(item)) => return Some(item),
            None => return None,
            Some(Err(_)) => {},
        }
    }
}

impl Iterator for Args {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|s| s.to_string_lossy().to_string())
    }
}

impl Iterator for ArgsOs {
    type Item = OsString;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(path) = self.current_arg_globs.as_mut().and_then(first_non_error) {
            return Some(path.into_os_string());
        }
        let arg = self.args.next()?; // if None — end of args
        let glob_opts = glob::MatchOptions { case_sensitive: false, ..Default::default() };
        if let Some(Ok(mut glob_iter)) = arg.pattern.as_ref().map(move |pat| glob::glob_with(pat, glob_opts)) {
            let first_glob = first_non_error(&mut glob_iter);
            self.current_arg_globs = Some(glob_iter);
            match first_glob {
                Some(path) => Some(path.into_os_string()),
                None => {
                    // non-matching patterns are passed as regular strings
                    self.current_arg_globs = None;
                    Some(arg.text)
                },
            }
            // Invalid patterns are passed as regular strings
        } else {
            // valid, but non-wildcard args passed as is, in order to avoid normalizing slashes
            Some(arg.text)
        }
    }
}

impl fmt::Debug for Args {
    #[cold]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.iter.fmt(f)
    }
}

impl fmt::Debug for ArgsOs {
    #[cold]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.args.fmt(f)
    }
}


#[test]
fn finds_cargo_toml() {
    let cmd = "foo.exe _not_?a?_[f]ilename_ \"_not_?a?_[p]attern_\" Cargo.tom?".chars().map(|c| c as u16).collect::<Vec<_>>();
    let args = ArgsOs::from_raw_command_line(Box::leak(cmd.into_boxed_slice()));
    let iter = Args { iter: args };
    assert_eq!("\"foo.exe _not_?a?_[f]ilename_ \\\"_not_?a?_[p]attern_\\\" Cargo.tom?\"", format!("{:?}", iter));
    let args: Vec<_> = iter.collect();
    assert_eq!(4, args.len());
    assert_eq!("foo.exe", &args[0]);
    assert_eq!("_not_?a?_[f]ilename_", &args[1]);
    assert_eq!("_not_?a?_[p]attern_", &args[2]);
    assert_eq!("Cargo.toml", &args[3]);
}

#[test]
fn unquoted_slashes_unchanged() {
    let cmd = r#"foo.exe //// .. ./ \\\\"#.chars().map(|c| c as u16).collect::<Vec<_>>();
    let args = ArgsOs::from_raw_command_line(Box::leak(cmd.into_boxed_slice()));
    let iter = Args { iter: args };
    let args: Vec<_> = iter.collect();
    assert_eq!(5, args.len());
    assert_eq!("foo.exe", &args[0]);
    assert_eq!("////", &args[1]);
    assert_eq!("..", &args[2]);
    assert_eq!("./", &args[3]);
    assert_eq!(r#"\\\\"#, &args[4]);
}

#[test]
fn finds_readme_case_insensitive() {
    let cmd = "foo.exe _not_?a?_[f]ilename_ \"_not_?a?_[p]attern_\" read*.MD".chars().map(|c| c as u16).collect::<Vec<_>>();
    let iter = ArgsOs::from_raw_command_line(Box::leak(cmd.into_boxed_slice()));
    let args: Vec<_> = iter.map(|c| c.to_string_lossy().to_string()).collect();
    assert_eq!(4, args.len());
    assert_eq!("foo.exe", &args[0]);
    assert_eq!("_not_?a?_[f]ilename_", &args[1]);
    assert_eq!("_not_?a?_[p]attern_", &args[2]);
    assert_eq!("README.md", &args[3]);
}
