use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::thread;

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use thiserror::Error;

use crate::store::{EMBED_DIMS, SEARCH_TEXT_CHARS, Store, StoreError};

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
    pub fn load(cache_dir: &Path) -> Result<Self, EmbedError> {
        Self::load_model(cache_dir, EmbeddingModel::BGESmallENV15)
    }

    pub fn load_model(cache_dir: &Path, model_kind: EmbeddingModel) -> Result<Self, EmbedError> {
        std::fs::create_dir_all(cache_dir).map_err(|e| EmbedError::Onnx(e.to_string()))?;
        let options = InitOptions::new(model_kind)
            .with_cache_dir(cache_dir.to_path_buf())
            .with_intra_threads(2)
            .with_show_download_progress(true);
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
}

#[derive(Debug)]
pub struct EmbedTestReport {
    pub model: &'static str,
    pub load_ms: u128,
    pub embed_ms: u128,
    pub dims: usize,
    pub first3: [f32; 3],
}

/// Load + embed once. Does not open SQLite.
pub fn run_embed_test(cache_dir: &Path, text: &str) -> Result<Vec<EmbedTestReport>, EmbedError> {
    let mut reports = Vec::new();
    reports.push(timed_embed(
        cache_dir,
        EmbeddingModel::BGESmallENV15,
        "BGESmallENV15",
        text,
    )?);
    if reports[0].embed_ms > 300 {
        reports.push(timed_embed(
            cache_dir,
            EmbeddingModel::BGESmallENV15Q,
            "BGESmallENV15Q",
            text,
        )?);
    }
    Ok(reports)
}

fn timed_embed(
    cache_dir: &Path,
    model_kind: EmbeddingModel,
    name: &'static str,
    text: &str,
) -> Result<EmbedTestReport, EmbedError> {
    let load_start = std::time::Instant::now();
    let mut embedder = Embedder::load_model(cache_dir, model_kind)?;
    let load_ms = load_start.elapsed().as_millis();
    let embed_start = std::time::Instant::now();
    let vector = embedder.embed_one(text)?;
    let embed_ms = embed_start.elapsed().as_millis();
    Ok(EmbedTestReport {
        model: name,
        load_ms,
        embed_ms,
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
    /// Load the ONNX model on a background thread. Returns after load succeeds.
    /// Capture can then enqueue jobs without waiting on embed.
    pub fn spawn(cache_dir: PathBuf, db_path: PathBuf) -> Result<Self, EmbedError> {
        let (job_tx, job_rx) = mpsc::channel::<EmbedJob>();
        let (ready_tx, ready_rx) = mpsc::channel();

        thread::Builder::new()
            .name("cliptrace-embed".into())
            .spawn(move || {
                let mut embedder = match Embedder::load(&cache_dir) {
                    Ok(e) => e,
                    Err(err) => {
                        let _ = ready_tx.send(Err(err));
                        return;
                    }
                };
                let store = match Store::open(&db_path) {
                    Ok(s) => s,
                    Err(err) => {
                        let _ = ready_tx.send(Err(EmbedError::Store(err)));
                        return;
                    }
                };
                let _ = ready_tx.send(Ok(()));
                while let Ok(job) = job_rx.recv() {
                    let text_len = job.search_text.len();
                    match embedder.embed_one(&job.search_text) {
                        Ok(vector) => match store.upsert_embedding(job.id, &vector) {
                            Ok(()) => {
                                eprintln!(
                                    "embed ok id={} dims={} text_len={}",
                                    job.id, EMBED_DIMS, text_len
                                );
                            }
                            Err(err) => {
                                eprintln!("embed store fail id={} text_len={}: {err}", job.id, text_len);
                            }
                        },
                        Err(err) => {
                            eprintln!("embed fail id={} text_len={}: {err}", job.id, text_len);
                        }
                    }
                }
            })
            .map_err(|e| EmbedError::Onnx(e.to_string()))?;

        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self { tx: job_tx }),
            Ok(Err(err)) => Err(err),
            Err(_) => Err(EmbedError::Onnx("embed worker thread exited".into())),
        }
    }

    pub fn submit(&self, id: i64, search_text: String) {
        if let Err(err) = self.tx.send(EmbedJob { id, search_text }) {
            eprintln!("embed enqueue fail id={id}: {err}");
        }
    }
}
