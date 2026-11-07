pub mod capture;
pub mod config;
pub mod store;

pub use capture::{CaptureConfig, CaptureError, ClipEvent, run_listener};
pub use config::{Config, ConfigError, Paths, load_or_create};
pub use store::{NewClip, Store, StoreError, UpsertOutcome, clip_hash, normalize_text};
