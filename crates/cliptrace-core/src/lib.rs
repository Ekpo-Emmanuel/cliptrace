pub mod autostart;
pub mod capture;
pub mod config;
pub mod hybrid;
pub mod rrf;
pub mod search;
pub mod secrets;
pub mod store;
pub mod tags;
pub mod timeparse;

#[cfg(feature = "embeddings")]
pub mod embed;

pub use autostart::{AutostartError, set_autostart};
pub use capture::{CaptureConfig, CaptureError, ClipEvent, run_listener, set_clipboard_text};
pub use hybrid::{HybridOutcome, hybrid_search};
pub use config::{Config, ConfigError, Paths, load_or_create, save};
#[cfg(feature = "embeddings")]
pub use embed::{
    BGE_QUERY_PREFIX, EmbedError, EmbedTestReport, EmbedWorker, Embedder, backfill_batch_size,
    backfill_sleep, run_embed_test, warn_once,
};
pub use rrf::{RRF_K, rank_index, reciprocal_rank_fusion};
pub use secrets::{
    SecretKind, detect, detect_span, mask_secret, redact_secrets, secret_hint, secret_search_text,
};
pub use search::{escape_fts_query, format_local_ms, fts_or_query, relative_time};
pub use store::{
    EMBED_DIMS, EMBED_MODEL, NewClip, RescanReport, SEARCH_TEXT_CHARS, SearchHit, Store, StoreError,
    StoredEmbedding, TagCount, UpsertOutcome, clip_hash, normalize_text,
};
pub use tags::{BrowseFilter, MAX_TAG_CHARS, MAX_TAGS_PER_CLIP, normalize_tag, parse_hash_tags};
pub use timeparse::{ParsedQuery, local_from_unix_ms, parse_time_phrase};
