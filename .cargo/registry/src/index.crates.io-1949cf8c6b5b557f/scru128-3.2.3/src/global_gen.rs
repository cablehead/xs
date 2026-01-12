#![cfg(feature = "global_gen")]

use crate::{Scru128Generator, Scru128Id};

/// Generates a new SCRU128 ID object using the global generator.
///
/// This function is thread-safe; multiple threads in a process can call it concurrently without
/// breaking the monotonic order of generated IDs. On Unix, this function resets the generator
/// state when the process ID changes (i.e., upon forks) to avoid collisions across processes.
pub fn new() -> Scru128Id {
    use std::sync::{Mutex, OnceLock};
    static G: OnceLock<Mutex<GlobalGenInner>> = OnceLock::new();
    G.get_or_init(Default::default)
        .lock()
        .expect("scru128: could not lock global generator")
        .generate()
}

/// Generates a new SCRU128 ID encoded in the 25-digit canonical string representation using the
/// global generator.
///
/// Use this to quickly get a new SCRU128 ID as a string.
///
/// This function is thread-safe; multiple threads in a process can call it concurrently without
/// breaking the monotonic order of generated IDs. On Unix, this function resets the generator
/// state when the process ID changes (i.e., upon forks) to avoid collisions across processes.
///
/// # Examples
///
/// ```rust
/// let x = scru128::new_string(); // e.g., "036z951mhjikzik2gsl81gr7l"
///
/// assert!(regex::Regex::new(r"^[0-9a-z]{25}$").unwrap().is_match(&x));
/// ```
pub fn new_string() -> String {
    new().into()
}

/// A thin wrapper to reset the state when the process ID changes (i.e., upon Unix forks).
#[derive(Debug)]
struct GlobalGenInner {
    #[cfg(unix)]
    pid: u32,
    generator: Scru128Generator,
}

impl Default for GlobalGenInner {
    fn default() -> Self {
        Self {
            #[cfg(unix)]
            pid: std::process::id(),
            generator: Default::default(),
        }
    }
}

impl GlobalGenInner {
    fn generate(&mut self) -> Scru128Id {
        #[cfg(unix)]
        if self.pid != std::process::id() {
            *self = Default::default();
        }
        self.generator.generate()
    }
}

#[cfg(test)]
mod tests {
    /// Generates no IDs sharing same timestamp and counters under multithreading
    #[test]
    fn generates_no_ids_sharing_same_timestamp_and_counters_under_multithreading(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::{collections::HashSet, sync::mpsc, thread};

        let (tx, rx) = mpsc::channel();
        for _ in 0..4 {
            let tx = tx.clone();
            thread::Builder::new()
                .spawn(move || {
                    for _ in 0..10000 {
                        tx.send(super::new()).unwrap();
                    }
                })
                .map_err(|err| format!("failed to spawn thread: {:?}", err))?;
        }
        drop(tx);

        let mut s = HashSet::new();
        while let Ok(e) = rx.recv() {
            s.insert((e.timestamp(), e.counter_hi(), e.counter_lo()));
        }

        assert_eq!(s.len(), 4 * 10000);
        Ok(())
    }
}
