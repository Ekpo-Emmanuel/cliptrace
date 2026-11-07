pub mod capture;
pub mod config;
pub mod search;
pub mod store;

pub use capture::{CaptureConfig, CaptureError, ClipEvent, run_listener, set_clipboard_text};
pub use config::{Config, ConfigError, Paths, load_or_create};
pub use search::{escape_fts_query, relative_time};
pub use store::{NewClip, SearchHit, Store, StoreError, UpsertOutcome, clip_hash, normalize_text};
