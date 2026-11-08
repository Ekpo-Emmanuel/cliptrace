use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use thiserror::Error;

use crate::store::{EMBED_DIMS, SEARCH_TEXT_CHARS, Store, StoreError};

pub const BGE_QUERY_PREFIX: &str = "Represent this sentence for searching relevant passages: ";

static EMBED_WARNED: AtomicBool = AtomicBool::new(false);

const BACKFILL_BATCH: usize = 8;
const BACKFILL_SLEEP: Duration = Duration::from_millis(80);

#[derive(Debug, Error)]
pub enum EmbedError {
    #[error("onnx/fastembed failed to load: {0}")]
    Onnx(String),
    #[error("embedding dim {0} != {EMBED_DIMS}")]
    Dim(usize),
    #[error(transparent)]
    Store(#[from] StoreError),
}

pub struct Embedder {
    model: TextEmbedding,
}

impl Embedder {
    pub fn load(cache_dir: &Path, onnx_dll: &Path) -> Result<Self, EmbedError> {
        apply_onnxruntime_env(onnx_dll);
        if !onnx_dll.is_file() {
            return Err(EmbedError::Onnx(format!(
                "onnxruntime.dll missing at {}",
                onnx_dll.display()
            )));
        }
        std::fs::create_dir_all(cache_dir).map_err(|e| EmbedError::Onnx(e.to_string()))?;
        let cached = dir_has_onnx(cache_dir);
        if cached {
            // Safety: set before hf-hub/fastembed run; no other threads yet or worker is dedicated.
            unsafe {
                std::env::set_var("HF_HUB_OFFLINE", "1");
            }
        }
        let options = InitOptions::new(EmbeddingModel::BGESmallENV15)
            .with_cache_dir(cache_dir.to_path_buf())
            .with_intra_threads(2)
            .with_show_download_progress(!cached);
        let model =
            TextEmbedding::try_new(options).map_err(|e| EmbedError::Onnx(e.to_string()))?;
        Ok(Self { model })
    }

    pub fn embed_one(&mut self, text: &str) -> Result<Vec<f32>, EmbedError> {
        let truncated: String = text.chars().take(SEARCH_TEXT_CHARS).collect();
        let vectors = self
            .model
            .embed(vec![truncated], None)
            .map_err(|e| EmbedError::Onnx(e.to_string()))?;
        let Some(vec) = vectors.into_iter().next() else {
            return Err(EmbedError::Onnx("model returned no vector".into()));
        };
        if vec.len() != EMBED_DIMS {
            return Err(EmbedError::Dim(vec.len()));
        }
        Ok(vec)
    }

    pub fn embed_query(&mut self, text: &str, use_prefix: bool) -> Result<Vec<f32>, EmbedError> {
        if use_prefix {
            self.embed_one(&format!("{BGE_QUERY_PREFIX}{text}"))
        } else {
            self.embed_one(text)
        }
    }
}

#[derive(Debug)]
pub struct EmbedTestReport {
    pub model: &'static str,
    pub load_ms: u128,
    pub embed1_ms: u128,
    pub embed2_ms: u128,
    pub dims: usize,
    pub first3: [f32; 3],
}

/// Load once, embed the same string twice. Does not open SQLite.
pub fn run_embed_test(
    cache_dir: &Path,
    onnx_dll: &Path,
    text: &str,
) -> Result<EmbedTestReport, EmbedError> {
    let load_start = std::time::Instant::now();
    let mut embedder = Embedder::load(cache_dir, onnx_dll)?;
    let load_ms = load_start.elapsed().as_millis();
    let t1 = std::time::Instant::now();
    let vector = embedder.embed_one(text)?;
    let embed1_ms = t1.elapsed().as_millis();
    let t2 = std::time::Instant::now();
    let _ = embedder.embed_one(text)?;
    let embed2_ms = t2.elapsed().as_millis();
    Ok(EmbedTestReport {
        model: "BGESmallENV15",
        load_ms,
        embed1_ms,
        embed2_ms,
        dims: vector.len(),
        first3: [vector[0], vector[1], vector[2]],
    })
}

pub struct EmbedJob {
    pub id: i64,
    pub search_text: String,
}

pub struct EmbedWorker {
    tx: Sender<EmbedJob>,
}

impl EmbedWorker {
    /// Spawn a worker that loads the model on the first job, not at startup.
    pub fn spawn(cache_dir: PathBuf, db_path: PathBuf, onnx_dll: PathBuf) -> Self {
        let (job_tx, job_rx) = mpsc::channel::<EmbedJob>();
        let _ = thread::Builder::new()
            .name("cliptrace-embed".into())
            .spawn(move || {
                let store = match Store::open(&db_path) {
                    Ok(s) => s,
                    Err(err) => {
                        warn_once(&format!("embed store open failed: {err}"));
                        return;
                    }
                };
                let mut embedder: Option<Embedder> = None;
                while let Ok(job) = job_rx.recv() {
                    let text_len = job.search_text.len();
                    if embedder.is_none() {
                        match Embedder::load(&cache_dir, &onnx_dll) {
                            Ok(e) => embedder = Some(e),
                            Err(err) => {
                                warn_once(&err.to_string());
                                continue;
                            }
                        }
                    }
                    let Some(model) = embedder.as_mut() else {
                        continue;
                    };
                    match model.embed_one(&job.search_text) {
                        Ok(vector) => match store.upsert_embedding(job.id, &vector) {
                            Ok(()) => {
                                eprintln!(
                                    "embed ok id={} dims={} text_len={}",
                                    job.id, EMBED_DIMS, text_len
                                );
                            }
                            Err(err) => {
                                eprintln!(
                                    "embed store fail id={} text_len={}: {err}",
                                    job.id, text_len
                                );
                            }
                        },
                        Err(err) => {
                            eprintln!("embed fail id={} text_len={}: {err}", job.id, text_len);
                        }
                    }
                }
            });
        Self { tx: job_tx }
    }

    pub fn submit(&self, id: i64, search_text: String) {
        if self.tx.send(EmbedJob { id, search_text }).is_err() {
            warn_once("embed worker is not running");
        }
    }
}

pub fn backfill_batch_size() -> usize {
    BACKFILL_BATCH
}

pub fn backfill_sleep() -> Duration {
    BACKFILL_SLEEP
}

pub fn warn_once(message: &str) {
    if !EMBED_WARNED.swap(true, Ordering::SeqCst) {
        eprintln!("warning: embeddings unavailable ({message}); capture and keyword search still work");
    }
}

pub fn apply_onnxruntime_env(dll: &Path) {
    // Safety: called before ONNX/hf-hub on this thread; PATH/ORT_DYLIB_PATH
    // must be visible to the dynamic loader.
    unsafe {
        std::env::set_var("ORT_DYLIB_PATH", dll);
        if let Some(dir) = dll.parent() {
            let prefix = dir.to_string_lossy();
            let path = std::env::var("PATH").unwrap_or_default();
            if !path
                .split(';')
                .any(|p| p.eq_ignore_ascii_case(prefix.as_ref()))
            {
                std::env::set_var("PATH", format!("{prefix};{path}"));
            }
        }
    }
}

fn dir_has_onnx(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("onnx"))
        {
            return true;
        }
        if path.is_dir() && dir_has_onnx(&path) {
            return true;
        }
    }
    false
}
