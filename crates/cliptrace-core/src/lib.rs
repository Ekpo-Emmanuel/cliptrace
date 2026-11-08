pub mod capture;
pub mod config;
pub mod search;
pub mod store;

#[cfg(feature = "embeddings")]
pub mod embed;

pub use capture::{CaptureConfig, CaptureError, ClipEvent, run_listener, set_clipboard_text};
pub use config::{Config, ConfigError, Paths, load_or_create};
#[cfg(feature = "embeddings")]
pub use embed::{EmbedError, EmbedTestReport, EmbedWorker, Embedder, run_embed_test};
pub use search::{escape_fts_query, relative_time};
pub use store::{
    EMBED_DIMS, EMBED_MODEL, NewClip, SEARCH_TEXT_CHARS, SearchHit, StoredEmbedding, Store,
    StoreError, UpsertOutcome, clip_hash, normalize_text,
};
