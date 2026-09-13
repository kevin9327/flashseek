use std::time::{Duration, SystemTime};

use crate::types::Hit;

const NAME_WEIGHT: f64 = 1000.0;
const PATH_WEIGHT: f64 = 100.0;
const CONTENT_WEIGHT: f64 = 10.0;
const RECENCY_WEIGHT: f64 = 50.0;
const RECENCY_WINDOW_SECS: f64 = 7.0 * 24.0 * 3600.0;

/// Name hits outrank path hits; recency adds a decaying boost.
pub fn rank_hits(hits: &mut [Hit], now: SystemTime) {
    for hit in hits.iter_mut() {
        hit.score = score(hit, now);
    }
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.record.name.cmp(&b.record.name))
    });
}

pub fn score(hit: &Hit, now: SystemTime) -> f64 {
    let mut s = 0.0;
    if hit.name_match {
        s += NAME_WEIGHT;
    } else if hit.path_match {
        s += PATH_WEIGHT;
    }
    if hit.content_match {
        s += CONTENT_WEIGHT;
    }
    if let Ok(age) = now.duration_since(hit.record.modified) {
        let secs = age.as_secs_f64();
        if secs < RECENCY_WINDOW_SECS {
            s += RECENCY_WEIGHT * (1.0 - secs / RECENCY_WINDOW_SECS);
        }
    }
    s
}

pub fn recency_window() -> Duration {
    Duration::from_secs(RECENCY_WINDOW_SECS as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FileRecord;
    use std::path::PathBuf;

    fn hit(name: &str, name_match: bool, path_match: bool, modified: SystemTime) -> Hit {
        Hit {
            record: FileRecord {
                id: 1,
                parent_id: None,
                name: name.into(),
                path: PathBuf::from(format!("C:\\docs\\{name}")),
                size: 1,
                modified,
                is_dir: false,
            },
            score: 0.0,
            name_match,
            path_match,
            content_match: false,
            snippet: None,
        }
    }

    #[test]
    fn name_outranks_path_and_recent_wins_tie_break_on_name_when_equal() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let mut hits = vec![
            hit("pathy.txt", false, true, now),
            hit("namey.txt", true, false, now),
        ];
        rank_hits(&mut hits, now);
        assert_eq!(hits[0].record.name, "namey.txt");
        assert!(hits[0].score > hits[1].score);
    }
}
