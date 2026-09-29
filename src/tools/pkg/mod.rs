pub mod cache;
pub mod commands;
pub mod discovery;
pub mod features;
pub mod hash;
pub mod index;
pub mod lock;
pub mod manifest;
pub mod resolver;
pub mod toml;
pub mod types;

#[cfg(test)]
mod tests;

pub use cache::*;
pub use commands::*;
pub use discovery::*;
pub use features::*;
pub use hash::*;
pub use index::*;
pub use lock::*;
pub use manifest::*;
pub use resolver::*;
pub use toml::*;
pub use types::*;

/// Serializes tests that read or mutate the process-global
/// `ALYA_REGISTRY_INDEX` (test threads share one process; set/restore
/// pairs race otherwise — observed as deterministic macOS-Intel CI red
/// when a slow installer test holds `file://` while a fast fetch test
/// runs). Poison-tolerant: a panicking holder must not cascade.
#[cfg(test)]
pub(crate) static REGISTRY_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub(crate) fn lock_registry_env() -> std::sync::MutexGuard<'static, ()> {
    REGISTRY_ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
