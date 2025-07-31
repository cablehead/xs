pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug)]
pub struct NotFound;

impl std::fmt::Display for NotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Not found")
    }
}

impl std::error::Error for NotFound {}

impl NotFound {
    /// Check if an error is our custom NotFound error
    pub fn is_not_found(err: &Error) -> bool {
        err.downcast_ref::<NotFound>().is_some()
    }
}

/// Check if an error has ErrorKind::NotFound in its chain
pub fn has_not_found_io_error(err: &Error) -> bool {
    // Check if it's directly an IO error with NotFound kind
    if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
        return io_err.kind() == std::io::ErrorKind::NotFound;
    }

    // Check the error chain for IO errors with NotFound kind
    let mut source = err.source();
    while let Some(err) = source {
        if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
            if io_err.kind() == std::io::ErrorKind::NotFound {
                return true;
            }
        }
        source = err.source();
    }

    false
}
