use std::collections::HashMap;

pub const RRF_K: i32 = 60;

/// Reciprocal rank fusion. `lists` are ranked ids, best first. Ranks are 1-based.
pub fn reciprocal_rank_fusion(lists: &[Vec<i64>], k: i32, limit: usize) -> Vec<i64> {
    let mut scores: HashMap<i64, f64> = HashMap::new();
    for list in lists {
        for (i, id) in list.iter().enumerate() {
            let rank = (i as i32) + 1;
            *scores.entry(*id).or_insert(0.0) += 1.0 / (f64::from(k + rank));
        }
    }
    let mut ranked: Vec<(i64, f64)> = scores.into_iter().collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    ranked.into_iter().take(limit.max(1)).map(|(id, _)| id).collect()
}

pub fn rank_index(list: &[i64], id: i64) -> Option<usize> {
    list.iter().position(|x| *x == id).map(|i| i + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rrf_prefers_agreement() {
        let kw = vec![20, 10, 30];
        let vecs = vec![20, 40, 10];
        let merged = reciprocal_rank_fusion(&[kw, vecs], RRF_K, 5);
        assert_eq!(merged[0], 20);
        assert_eq!(merged[1], 10);
        assert!(merged.contains(&30));
        assert!(merged.contains(&40));
    }
}
