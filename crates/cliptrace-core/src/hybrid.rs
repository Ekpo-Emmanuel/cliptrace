use std::collections::HashMap;

use crate::rrf::{RRF_K, reciprocal_rank_fusion};
use crate::search::fts_or_query;
use crate::store::{SearchHit, Store, StoreError};
use crate::tags::{BrowseFilter, parse_hash_tags};
use crate::timeparse::{local_from_unix_ms, parse_time_phrase};

pub struct HybridOutcome {
    pub hits: Vec<SearchHit>,
    pub keyword_only: bool,
}

/// Same hybrid path as the CLI: timeparse, FTS OR, optional vector, RRF k=60.
pub fn hybrid_search(
    store: &Store,
    query: &str,
    now_ms: i64,
    limit: usize,
    query_vector: Option<&[f32]>,
) -> Result<HybridOutcome, StoreError> {
    let (stripped, hash_tags) = parse_hash_tags(query);
    let parsed = parse_time_phrase(&stripped, local_from_unix_ms(now_ms));
    let mut filter = BrowseFilter::default();
    filter.tags.extend(hash_tags);
    let pool = limit.max(50);
    let keyword_hits =
        store.search_keyword_scoped(&parsed.text, pool, parsed.range, &filter)?;
    let can_vector = query_vector.is_some() && fts_or_query(&parsed.text).is_some();
    if let (Some(vector), true) = (query_vector, can_vector) {
        let vector_hits =
            store.search_vector_scoped(vector, pool, parsed.range, &filter)?;
        let keyword_ids: Vec<i64> = keyword_hits.iter().map(|h| h.id).collect();
        let vector_ids: Vec<i64> = vector_hits.iter().map(|h| h.id).collect();
        let merged = reciprocal_rank_fusion(&[keyword_ids, vector_ids], RRF_K, limit);
        let mut by_id = HashMap::new();
        for hit in keyword_hits.iter().chain(vector_hits.iter()) {
            by_id.entry(hit.id).or_insert_with(|| hit.clone());
        }
        let hits = merged
            .into_iter()
            .filter_map(|id| by_id.remove(&id))
            .collect();
        return Ok(HybridOutcome {
            hits,
            keyword_only: false,
        });
    }
    Ok(HybridOutcome {
        hits: keyword_hits.into_iter().take(limit.max(1)).collect(),
        keyword_only: true,
    })
}
