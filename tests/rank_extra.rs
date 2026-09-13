//! Exact basename (name without extension) matching a query term ranks higher.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use flashseek::rank::{rank_hits_with_terms, score};
use flashseek::{FileRecord, Hit};

fn hit(name: &str, modified: SystemTime) -> Hit {
    Hit {
        record: FileRecord {
            id: 1,
            parent_id: None,
            name: name.into(),
            path: PathBuf::from(format!(r"C:\docs\{name}")),
            size: 1,
            modified,
            is_dir: false,
        },
        score: 0.0,
        name_match: true,
        path_match: false,
        content_match: false,
        snippet: None,
    }
}

#[test]
fn exact_basename_equals_query_term_outranks_partial_name() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
    let invoice = hit("invoice.txt", now);
    let backup = hit("invoice-backup.txt", now);
    let terms = vec!["invoice".to_string()];

    assert!(
        score(&invoice, now, &terms) > score(&backup, now, &terms),
        "exact basename must score above a name that only contains the term"
    );

    let mut hits = vec![backup, invoice];
    rank_hits_with_terms(&mut hits, now, &terms);
    assert_eq!(hits[0].record.name, "invoice.txt");
    assert!(hits[0].score > hits[1].score);
}
