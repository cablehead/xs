//! Store schema migrations. See ADR 0005 for the topic rename this runs.

mod v0_to_v1;

#[cfg(test)]
mod tests;

use crate::store::Store;

/// Reserved key in `idx_topic` for the store's schema version. The leading
/// `\0` byte is below any valid topic's first byte (topics must start with
/// `[a-zA-Z_]`), so this key cannot collide with topic-derived entries.
pub const SCHEMA_VERSION_KEY: &[u8] = b"\0schema_version";

/// Current schema version. Bump and add a migrate_vN_to_vN+1 step when the
/// topic vocabulary changes.
pub const CURRENT_VERSION: u32 = 1;

#[derive(Debug)]
pub enum MigrationError {
    UnknownVersion(u32, u32),
    Store(String),
    Fjall(fjall::Error),
}

impl std::fmt::Display for MigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrationError::UnknownVersion(v, current) => {
                write!(f, "unknown schema version: {v} (highest known is {current})")
            }
            MigrationError::Store(s) => write!(f, "store error during migration: {s}"),
            MigrationError::Fjall(e) => write!(f, "fjall error during migration: {e}"),
        }
    }
}

impl std::error::Error for MigrationError {}

impl From<fjall::Error> for MigrationError {
    fn from(e: fjall::Error) -> Self {
        MigrationError::Fjall(e)
    }
}

/// Run all pending migrations on the store. Idempotent: a store already at
/// `CURRENT_VERSION` is a no-op.
pub fn migrate(store: &Store) -> Result<(), MigrationError> {
    let mut version = read_schema_version(store)?;
    while version < CURRENT_VERSION {
        match version {
            0 => v0_to_v1::run(store)?,
            v => return Err(MigrationError::UnknownVersion(v, CURRENT_VERSION)),
        }
        version += 1;
        write_schema_version(store, version)?;
    }
    Ok(())
}

pub(crate) fn read_schema_version(store: &Store) -> Result<u32, MigrationError> {
    let raw = store.idx_topic.get(SCHEMA_VERSION_KEY)?;
    let Some(bytes) = raw else { return Ok(0) };
    if bytes.len() != 4 {
        return Ok(0);
    }
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

pub(crate) fn write_schema_version(store: &Store, v: u32) -> Result<(), MigrationError> {
    let mut batch = store.db.batch();
    batch.insert(&store.idx_topic, SCHEMA_VERSION_KEY, &v.to_le_bytes()[..]);
    batch.commit()?;
    store.db.persist(fjall::PersistMode::SyncAll)?;
    Ok(())
}
